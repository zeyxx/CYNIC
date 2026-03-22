//! REST handler for POST /observe — development workflow observation capture.
//! Fire-and-forget: hooks POST here after each tool use.
//! Observations feed CCM crystallization over time.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::storage::Observation;

#[derive(Deserialize)]
pub struct ObserveRequest {
    pub tool: String,
    pub target: Option<String>,
    pub domain: Option<String>,
    pub status: Option<String>,
    pub context: Option<String>,
    pub project: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
}

/// Infer domain from file extension. Returns "general" if no match.
fn infer_domain(target: Option<&str>) -> String {
    target
        .and_then(|t| t.rsplit('.').next())
        .map(|ext| match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "md" => "docs",
            "toml" | "json" | "yaml" | "yml" => "config",
            _ => "general",
        })
        .unwrap_or("general")
        .to_string()
}

pub async fn observe_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ObserveRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Validate required field
    if req.tool.is_empty() || req.tool.len() > 64 {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "tool must be 1-64 characters".into(),
        })));
    }

    // Infer domain from target file extension if not provided
    let domain = req.domain.unwrap_or_else(|| infer_domain(req.target.as_deref()));

    let obs = Observation {
        project: req.project.unwrap_or_else(|| "CYNIC".into()),
        agent_id: req.agent_id.unwrap_or_else(|| "unknown".into()),
        tool: req.tool,
        target: req.target.unwrap_or_default(),
        domain,
        status: req.status.unwrap_or_else(|| "success".into()),
        context: req.context.map(|c| c.chars().take(200).collect()).unwrap_or_default(),
        session_id: req.session_id.unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // Fire-and-forget — bounded (Semaphore) + tracked (TaskTracker) + timed out (5s)
    let semaphore = Arc::clone(&state.bg_semaphore);
    match semaphore.try_acquire_owned() {
        Ok(permit) => {
            let storage = Arc::clone(&state.storage);
            let obs_clone = obs.clone();
            state.bg_tasks.spawn(async move {
                let _permit = permit; // held until task completes
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    storage.store_observation(&obs_clone),
                ).await {
                    Ok(Err(e)) => tracing::warn!(error = %e, "store_observation failed"),
                    Err(_) => tracing::warn!("store_observation timed out (5s)"),
                    _ => {}
                }
            });
        }
        Err(_) => {
            tracing::warn!("background task limit reached, observation dropped");
        }
    }

    Ok(Json(serde_json::json!({ "status": "observed" })))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── infer_domain ─────────────────────────────────────

    #[test]
    fn infer_rust() {
        assert_eq!(infer_domain(Some("src/judge.rs")), "rust");
    }

    #[test]
    fn infer_typescript() {
        assert_eq!(infer_domain(Some("App.tsx")), "typescript");
        assert_eq!(infer_domain(Some("utils.ts")), "typescript");
    }

    #[test]
    fn infer_javascript() {
        assert_eq!(infer_domain(Some("index.js")), "javascript");
        assert_eq!(infer_domain(Some("Component.jsx")), "javascript");
    }

    #[test]
    fn infer_python() {
        assert_eq!(infer_domain(Some("train.py")), "python");
    }

    #[test]
    fn infer_config_formats() {
        assert_eq!(infer_domain(Some("Cargo.toml")), "config");
        assert_eq!(infer_domain(Some("package.json")), "config");
        assert_eq!(infer_domain(Some("config.yaml")), "config");
        assert_eq!(infer_domain(Some("ci.yml")), "config");
    }

    #[test]
    fn infer_docs() {
        assert_eq!(infer_domain(Some("README.md")), "docs");
    }

    #[test]
    fn infer_unknown_extension() {
        assert_eq!(infer_domain(Some("binary.wasm")), "general");
    }

    #[test]
    fn infer_no_target() {
        assert_eq!(infer_domain(None), "general");
    }

    #[test]
    fn infer_no_extension() {
        assert_eq!(infer_domain(Some("Makefile")), "general");
    }

    // ── ObserveRequest validation ────────────────────────

    #[test]
    fn observe_request_deserializes_minimal() {
        let json = r#"{"tool":"Read"}"#;
        let req: ObserveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.tool, "Read");
        assert!(req.target.is_none());
        assert!(req.domain.is_none());
        assert!(req.session_id.is_none());
    }

    #[test]
    fn observe_request_deserializes_full() {
        let json = r#"{"tool":"Edit","target":"src/main.rs","domain":"rust","status":"success","context":"fixing bug","project":"CYNIC","agent_id":"claude-123","session_id":"sess-1"}"#;
        let req: ObserveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.tool, "Edit");
        assert_eq!(req.target.as_deref(), Some("src/main.rs"));
        assert_eq!(req.agent_id.as_deref(), Some("claude-123"));
    }
}
