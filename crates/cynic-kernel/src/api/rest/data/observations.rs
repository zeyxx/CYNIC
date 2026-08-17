//! Observation REST handlers.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::rest::response::storage_error;
use crate::api::rest::types::{AppState, ErrorResponse};
use crate::domain::sanitize::sanitize_observation_target;

#[derive(Debug, Deserialize, Default)]
pub struct ObservationsQuery {
    pub limit: Option<u32>,
    pub domain: Option<String>,
    pub agent_id: Option<String>,
    pub project: Option<String>,
    pub workstream: Option<String>,
    pub thread_id: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PublicObservationsQuery {
    pub limit: Option<u32>,
    pub domain: Option<String>,
}

pub fn public_observation_projection(
    obs: &crate::domain::storage::RawObservation,
) -> serde_json::Value {
    let created_at_str = chrono::DateTime::from_timestamp(
        obs.created_at / 1000,
        ((obs.created_at % 1000) * 1_000_000) as u32,
    )
    .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    serde_json::json!({
        "tool": obs.tool,
        "target": sanitize_observation_target(&obs.target),
        "domain": obs.domain,
        "status": obs.status,
        "project": obs.project,
        "tags": obs.tags,
        "source_tier": obs.source_tier,
        "created_at": created_at_str,
    })
}

pub async fn public_observations_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<PublicObservationsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(100).min(100);
    match state
        .storage
        .list_observations_raw(q.domain.as_deref(), None, limit)
        .await
    {
        Ok(rows) => Ok(Json(
            rows.iter().map(public_observation_projection).collect(),
        )),
        Err(e) => {
            tracing::warn!(error = %e, "public observations list failed");
            Err(storage_error())
        }
    }
}

/// GET /observations — raw workflow observations.
pub async fn observations_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ObservationsQuery>,
) -> Result<Json<Vec<crate::domain::storage::RawObservation>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(100).min(100);
    match state
        .storage
        .list_observations_filtered(
            q.domain.as_deref(),
            q.agent_id.as_deref(),
            q.project.as_deref(),
            q.workstream.as_deref(),
            q.thread_id.as_deref(),
            q.target.as_deref(),
            limit,
        )
        .await
    {
        Ok(rows) => Ok(Json(rows)),
        Err(e) => {
            tracing::warn!(error = %e, "observations list failed");
            Err(storage_error())
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct SessionsQuery {
    pub limit: Option<u32>,
}

/// GET /sessions — session summaries.
pub async fn sessions_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SessionsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(50).min(200);
    match state.storage.list_session_summaries(limit).await {
        Ok(summaries) => {
            let items: Vec<serde_json::Value> = summaries
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "session_id": s.session_id,
                        "agent_id": s.agent_id,
                        "summary": s.summary,
                        "observations_count": s.observations_count,
                        "created_at": s.created_at,
                    })
                })
                .collect();
            Ok(Json(items))
        }
        Err(e) => {
            tracing::warn!(error = %e, "sessions list failed");
            Err(storage_error())
        }
    }
}
