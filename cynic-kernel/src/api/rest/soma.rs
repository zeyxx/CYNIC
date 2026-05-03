//! REST API handler for Soma L1 — resource orchestration and GPU utilization-aware dispatch.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::orchestrator::{GateDecision, Priority, ResourceRequest};

/// Request payload for /soma/request — ask the gate if a task can be allocated now.
#[derive(Debug, Clone, Deserialize)]
pub struct SomaRequestPayload {
    /// Task identifier (e.g., "hermes-chat-1", "nightshift-eval-dog-2")
    pub task_name: String,
    /// Priority: "background" (0), "nightshift" (1), "hermes" (2)
    pub priority: String,
    /// Estimated duration in seconds
    pub estimated_duration_secs: u64,
    /// URL to llama-server (e.g., "http://127.0.0.1:8080")
    pub llama_url: String,
}

/// Response payload — the gate's decision.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "decision", content = "data")]
pub enum SomaDecisionResponse {
    /// Allocate: task can start immediately. slot_id is a unique allocation token.
    #[serde(rename = "allocate")]
    Allocate { slot_id: String },
    /// Queue: utilization too high. Retry after wait_secs.
    #[serde(rename = "queue")]
    Queue { wait_secs: u64 },
}

impl From<GateDecision> for SomaDecisionResponse {
    fn from(decision: GateDecision) -> Self {
        match decision {
            GateDecision::Allocate { slot_id } => SomaDecisionResponse::Allocate { slot_id },
            GateDecision::Queue { wait_secs } => SomaDecisionResponse::Queue { wait_secs },
        }
    }
}

/// POST /soma/request — Query the resource gate for allocation decision.
///
/// Request: { "task_name": "hermes-1", "priority": "hermes", "estimated_duration_secs": 300, "llama_url": "http://..." }
/// Response (200): { "decision": "allocate", "data": { "slot_id": "hermes-1-1714878456789" } }
///             or: { "decision": "queue", "data": { "wait_secs": 5 } }
pub async fn soma_request_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SomaRequestPayload>,
) -> Result<(StatusCode, Json<SomaDecisionResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Parse priority from string to enum
    let priority = match payload.priority.to_lowercase().as_str() {
        "background" => Priority::Background,
        "nightshift" => Priority::Nightshift,
        "hermes" => Priority::Hermes,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid priority — valid values: background, nightshift, hermes".into(),
                }),
            ));
        }
    };

    // Get or create the resource gate (singleton per state)
    let gate = state.soma_gate.clone();

    // Build the request and query the gate
    let request = ResourceRequest {
        task_name: payload.task_name,
        priority,
        estimated_duration_secs: payload.estimated_duration_secs,
        llama_url: payload.llama_url,
    };

    let decision = gate.request(request).await;

    // Return the decision
    let response = SomaDecisionResponse::from(decision);
    Ok((StatusCode::OK, Json(response)))
}
