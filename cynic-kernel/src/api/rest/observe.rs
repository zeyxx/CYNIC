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
    let domain = req.domain.unwrap_or_else(|| {
        req.target.as_deref()
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
    });

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

    // Fire-and-forget — don't block the response on DB write
    let storage = Arc::clone(&state.storage);
    let obs_clone = obs.clone();
    tokio::spawn(async move {
        if let Err(e) = storage.store_observation(&obs_clone).await {
            eprintln!("[REST/observe] Warning: failed to store observation: {}", e);
        }
    });

    Ok(Json(serde_json::json!({ "status": "observed" })))
}
