//! REST handler for POST /observe — development workflow observation capture.
//! Fire-and-forget: hooks POST here after each tool use.
//! Observations feed CCM crystallization over time.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Deserialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};

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

    // ── Ledger system (observation consensus) ──
    pub value: Option<serde_json::Value>, // The fact value
    pub confidence: Option<String>,       // observed|deduced|inferred|conjecture
    pub consumer: Option<String>,         // K15: who must act
    pub action: Option<String>,           // K15: what changes if true
    pub depends_on: Option<Vec<String>>,  // Graph: what this depends on
    pub maturity: Option<f64>,            // Kairos: is it ripe?
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

    // K15 gate: if posting to mempool/hackathon or status=critical, consumer is required
    let is_critical = req
        .domain
        .as_deref()
        .map(|d| d.contains("mempool") || d.contains("hackathon"))
        .unwrap_or(false)
        || req.status.as_deref() == Some("critical");
    if is_critical && req.consumer.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "K15: consumer field is required for critical observations".into(),
            }),
        ));
    }

    let agent_id = req.agent_id.clone();
    let source_tier =
        crate::introspection::classify_source_tier(agent_id.as_deref().unwrap_or("unknown"));
    let mut obs = crate::domain::ccm::build_observation_with_ledger(
        req.tool,
        req.target,
        req.domain,
        req.status,
        req.context,
        req.project,
        agent_id.clone(),
        req.session_id,
        req.tags,
        req.value,      // Ledger: the fact value
        req.confidence, // Ledger: epistemic label
        req.consumer,   // Ledger: K15 consumer
        req.action,     // Ledger: K15 action
        req.depends_on, // Ledger: Kairos dependencies
        req.maturity,   // Ledger: Kairos maturity
    );
    obs.source_tier = source_tier.to_string();

    // K15: Route observation to source organ if agent_id matches, else kernel storage
    let semaphore = Arc::clone(&state.bg_semaphore);
    match semaphore.try_acquire_owned() {
        Ok(permit) => {
            let storage = Arc::clone(&state.storage);
            let senses = state.senses.clone();
            let obs_clone = obs;

            state.bg_tasks.spawn(async move {
                let _permit = permit;

                // Try to route to organ by agent_id
                let mut stored = false;
                if let Some(agent) = &agent_id {
                    tracing::debug!(agent = agent, "K15: checking if agent matches hermes-x");
                    if agent.starts_with("hermes-x") || agent == "hermes-x" {
                        tracing::debug!(agent = agent, senses_count = senses.len(), "K15: agent matches, searching senses");
                        if let Some(organ) = senses.iter().find(|s| s.name() == "hermes-x") {
                            tracing::info!(agent = agent, "K15: found hermes-x organ, calling store_observation");
                            match organ.store_observation(obs_clone.clone()).await {
                                Ok(()) => {
                                    tracing::info!(agent = agent, "observation routed to hermes-x organ");
                                    stored = true;
                                }
                                Err(e) => {
                                    tracing::warn!(error = ?e, agent = agent, "organ store failed, falling back to kernel");
                                }
                            }
                        } else {
                            tracing::warn!(senses_count = senses.len(), "K15: hermes-x organ not found in senses");
                        }
                    } else {
                        tracing::debug!(agent = agent, "K15: agent does not match hermes-x pattern");
                    }
                } else {
                    tracing::debug!("K15: no agent_id provided");
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
            state.metrics.inc_observation_dropped();
            tracing::warn!("background task limit reached, observation dropped");
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "observation dropped: background task limit reached".into(),
                }),
            ));
        }
    }

    state.metrics.inc_observation_ingested();
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
