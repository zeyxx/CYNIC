//! REST handler for POST /observe — development workflow observation capture.
//! Fire-and-forget: hooks POST here after each tool use.
//! Observations feed CCM crystallization over time.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Deserialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::ccm::build_observation;

#[derive(Debug, Deserialize)]
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
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "tool must be 1-64 characters".into(),
            }),
        ));
    }

    let obs = build_observation(
        req.tool,
        req.target,
        req.domain,
        req.status,
        req.context,
        req.project,
        req.agent_id,
        req.session_id,
    );

    // Fire-and-forget — bounded (Semaphore) + tracked (TaskTracker) + timed out (5s)
    let semaphore = Arc::clone(&state.bg_semaphore);
    match semaphore.try_acquire_owned() {
        Ok(permit) => {
            let storage = Arc::clone(&state.storage);
            let obs_clone = obs;
            state.bg_tasks.spawn(async move {
                let _permit = permit; // held until task completes
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    storage.store_observation(&obs_clone),
                )
                .await
                {
                    Ok(Err(e)) => tracing::warn!(error = %e, "store_observation failed"),
                    Err(_) => tracing::warn!("store_observation timed out (5s)"),
                    _ => {}
                }
            });
        }
        Err(_) => {
            tracing::warn!("background task limit reached, observation dropped");
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "observation dropped: background task limit reached".into(),
                }),
            ));
        }
    }

    Ok(Json(serde_json::json!({ "status": "observed" })))
}

#[cfg(test)]
mod tests {
    use super::*;

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
