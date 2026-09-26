//! REST handler for POST /events and GET /fleet/stats — infrastructure observability.
//! Events feed node routing decisions (Phase 2).

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use cynic_node::fleet_observability::{project_fleet_stats, validate_event_identity};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::storage::Event;

#[derive(Debug, Deserialize)]
pub struct EventRequest {
    pub tool: String,
    pub node: String,
    pub elapsed_ms: u64,
    pub output_bytes: u64,
    pub success: bool,
    pub metadata: Option<String>,
    pub agent_id: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FleetStatsResponse {
    pub nodes: Vec<NodeStats>,
    pub timestamp: i64,
    pub window_secs: u64,
}

pub use cynic_node::fleet_observability::FleetNodeStats as NodeStats;

/// POST /events — log infrastructure event (node latency, output size, success).
/// Fire-and-forget with bounded background task pool (K6 timeout enforcement).
pub async fn event_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EventRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if let Err(error) = validate_event_identity(&req.tool, &req.node) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: error.message().into(),
            }),
        ));
    }

    let event = Event {
        tool: req.tool,
        node: req.node,
        elapsed_ms: req.elapsed_ms,
        output_bytes: req.output_bytes,
        success: req.success,
        metadata: req.metadata.unwrap_or_default(),
        agent_id: req.agent_id.unwrap_or_else(|| "unknown".to_string()),
        timestamp: chrono::Utc::now().timestamp_millis(),
        failure_reason: req.failure_reason.unwrap_or_default(),
    };

    // Fire-and-forget — bounded + tracked + timed out (5s)
    let semaphore = Arc::clone(&state.bg_semaphore);
    match semaphore.try_acquire_owned() {
        Ok(permit) => {
            let storage = Arc::clone(&state.storage);
            let event_clone = event;
            state.bg_tasks.spawn(async move {
                let _permit = permit; // held until task completes
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    storage.store_event(&event_clone),
                )
                .await
                {
                    Ok(Err(e)) => tracing::warn!(error = %e, "store_event failed"),
                    Err(_) => tracing::warn!("store_event timed out (5s)"),
                    _ => {}
                }
            });
        }
        Err(_) => {
            tracing::warn!("background task limit reached, event dropped");
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "event dropped: background task limit reached".into(),
                }),
            ));
        }
    }

    Ok(Json(serde_json::json!({ "status": "logged" })))
}

#[derive(Debug, Deserialize)]
pub struct FleetStatsQuery {
    pub window_secs: Option<u64>,
    pub limit: Option<u32>,
}

/// GET /fleet/stats — node routing intelligence (authenticated only).
/// Returns aggregated latencies + success rates per node over a time window.
/// Used by inference router to select fastest node.
pub async fn fleet_stats_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<FleetStatsQuery>,
) -> Result<Json<FleetStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let window_secs = params.window_secs.unwrap_or(3600); // 1h default
    let limit = params.limit.unwrap_or(20);

    match state.storage.fleet_stats(window_secs, limit).await {
        Ok(stats) => {
            let nodes = project_fleet_stats(stats);

            Ok(Json(FleetStatsResponse {
                nodes,
                timestamp: chrono::Utc::now().timestamp_millis(),
                window_secs,
            }))
        }
        Err(e) => {
            tracing::warn!(error = %e, "fleet_stats query failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("fleet_stats query failed: {e}"),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_request_deserializes() {
        let json = r#"{"tool":"mcp_tailscale","node":"cynic-host-gpu","elapsed_ms":250,"output_bytes":1024,"success":true}"#;
        let req: EventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.tool, "mcp_tailscale");
        assert_eq!(req.node, "cynic-host-gpu");
        assert_eq!(req.elapsed_ms, 250);
    }

    #[test]
    fn fleet_stats_query_defaults() {
        let query: FleetStatsQuery = serde_json::from_str("{}").unwrap();
        assert!(query.window_secs.is_none());
        assert!(query.limit.is_none());
    }
}
