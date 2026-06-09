//! Dispatch endpoints — data-centric zone activity visibility.
//! Read-only queries over the existing observation stream.
//! No state management, no claim lifecycle, no TTL.

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{AppState, ErrorResponse};

#[derive(Debug, Deserialize)]
pub struct ZoneActivityQuery {
    /// File path to resolve to zone (relative to project root)
    pub file_path: Option<String>,
    /// Or query a zone directly by name
    pub zone: Option<String>,
    /// Requesting agent (excluded from results)
    pub agent_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ZoneActivityResponse {
    /// Resolved zone (None if file doesn't map to any zone)
    pub zone: Option<String>,
    /// Other agents active in this zone, ordered by most recent
    pub active_agents: Vec<AgentActivity>,
    /// Total observations in this zone in the query window
    pub observation_count: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct AgentActivity {
    pub agent_id: String,
    pub last_active: String,
    pub last_file: String,
    pub activity_count: u64,
}

/// GET /dispatch/zone-activity
///
/// Query the observation stream for recent activity in a zone.
/// Pure read — no state mutation, no claim lifecycle.
/// The hook/agent interprets freshness and decides whether to proceed.
pub async fn zone_activity_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ZoneActivityQuery>,
) -> Result<Json<ZoneActivityResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 1. Resolve zone from file_path or direct zone param
    let zone = match (&params.zone, &params.file_path) {
        (Some(z), _) => Some(z.clone()),
        (None, Some(fp)) => state.zones.resolve(fp),
        (None, None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "provide file_path or zone".into(),
                }),
            ));
        }
    };

    let Some(zone) = zone else {
        return Ok(Json(ZoneActivityResponse {
            zone: None,
            active_agents: vec![],
            observation_count: 0,
        }));
    };

    // 2. Get path prefixes for this zone
    let zone_paths = state.zones.paths_for_zone(&zone).unwrap_or_default();
    if zone_paths.is_empty() {
        return Ok(Json(ZoneActivityResponse {
            zone: Some(zone),
            active_agents: vec![],
            observation_count: 0,
        }));
    }

    // 3. Query observations — who was active in these paths recently?
    let exclude_agent = params.agent_id.as_deref().unwrap_or("");
    let project_root = &state.project_root;

    let agents = state
        .storage
        .zone_activity(&zone_paths, exclude_agent, project_root)
        .await
        .unwrap_or_default();

    let observation_count = agents.iter().map(|a| a.activity_count).sum();

    Ok(Json(ZoneActivityResponse {
        zone: Some(zone),
        active_agents: agents,
        observation_count,
    }))
}
