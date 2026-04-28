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
    pub tags: Option<Vec<String>>,
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
        req.agent_id.clone(),
        req.session_id,
        req.tags,
    );

    // K15: Route observation to source organ if agent_id matches, else kernel storage
    let semaphore = Arc::clone(&state.bg_semaphore);
    match semaphore.try_acquire_owned() {
        Ok(permit) => {
            let storage = Arc::clone(&state.storage);
            let senses = state.senses.clone();
            let obs_clone = obs;
            let agent_id = req.agent_id.clone();

            state.bg_tasks.spawn(async move {
                let _permit = permit;

                // Try to route to organ by agent_id
                let mut stored = false;
                if let Some(agent) = &agent_id {
                    if agent.starts_with("hermes-x") || agent == "hermes-x" {
                        if let Some(organ) = senses.iter().find(|s| s.name() == "hermes-x") {
                            match organ.store_observation(obs_clone.clone()).await {
                                Ok(()) => {
                                    tracing::info!(agent = agent, "observation routed to hermes-x organ");
                                    stored = true;
                                }
                                Err(e) => {
                                    tracing::warn!(error = ?e, agent = agent, "organ store failed, falling back to kernel");
                                }
                            }
                        }
                    }
                }

                // Fallback to kernel storage
                if !stored {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        storage.store_observation(&obs_clone),
                    )
                    .await
                    {
                        Ok(Err(e)) => tracing::warn!(error = %e, "kernel store_observation failed"),
                        Err(_) => tracing::warn!("kernel store_observation timed out (5s)"),
                        _ => {}
                    }
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
