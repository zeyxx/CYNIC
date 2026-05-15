//! REST handlers for multi-cortex dispatch coordination.
//! POST /agent-dispatch — create a new dispatch record
//! GET /agent-dispatch?scope=... — get active dispatch for scope
//! GET /agent-dispatch?claimed_by=... — get active dispatches for agent

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::storage::AgentDispatch;

#[derive(Debug, Deserialize)]
pub struct CreateDispatchRequest {
    pub scope: String,
    pub zone: String,
    pub claimed_by: String,
    pub branch: String,
}

#[derive(Debug, Deserialize)]
pub struct QueryDispatchRequest {
    pub scope: Option<String>,
    pub claimed_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DispatchResponse {
    pub dispatch: AgentDispatch,
}

#[derive(Debug, Serialize)]
pub struct DispatchListResponse {
    pub dispatches: Vec<AgentDispatch>,
}

/// POST /agent-dispatch — create a new dispatch record for work coordination.
pub async fn create_dispatch_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDispatchRequest>,
) -> Result<Json<DispatchResponse>, (StatusCode, Json<ErrorResponse>)> {
    if req.scope.is_empty() || req.scope.len() > 128 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "scope must be 1-128 characters".into(),
            }),
        ));
    }
    if req.zone.is_empty() || req.zone.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "zone must be 1-64 characters".into(),
            }),
        ));
    }
    if req.claimed_by.is_empty() || req.claimed_by.len() > 128 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "claimed_by must be 1-128 characters".into(),
            }),
        ));
    }
    if req.branch.is_empty() || req.branch.len() > 255 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "branch must be 1-255 characters".into(),
            }),
        ));
    }

    let dispatch_id = format!("dispatch:{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();
    let dispatch = AgentDispatch {
        id: dispatch_id,
        scope: req.scope,
        zone: req.zone,
        claimed_by: req.claimed_by,
        branch: req.branch,
        status: "CLAIMED".to_string(),
        created_at: now,
        completed_at: None,
        pr_number: None,
        hash: String::new(),      // Computed by storage layer
        prev_hash: String::new(), // Computed by storage layer
    };

    match state.storage.store_agent_dispatch(&dispatch).await {
        Ok(_) => Ok(Json(DispatchResponse { dispatch })),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to store dispatch: {err}"),
            }),
        )),
    }
}

/// GET /agent-dispatch — query active dispatches by scope or agent.
pub async fn get_dispatch_handler(
    State(state): State<Arc<AppState>>,
    Query(req): Query<QueryDispatchRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.scope.is_none() && req.claimed_by.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "must provide either scope or claimed_by query parameter".into(),
            }),
        ));
    }

    // Query by scope
    if let Some(scope) = req.scope {
        match state.storage.get_active_dispatch_for_scope(&scope).await {
            Ok(Some(dispatch)) => {
                return Ok(Json(serde_json::json!({
                    "dispatch": dispatch
                })));
            }
            Ok(None) => {
                return Ok(Json(serde_json::json!({
                    "dispatch": serde_json::Value::Null
                })));
            }
            Err(err) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to query dispatch: {err}"),
                    }),
                ));
            }
        }
    }

    // Query by claimed_by
    if let Some(claimed_by) = req.claimed_by {
        match state
            .storage
            .get_active_dispatches_for_agent(&claimed_by)
            .await
        {
            Ok(dispatches) => {
                return Ok(Json(serde_json::json!({
                    "dispatches": dispatches
                })));
            }
            Err(err) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to query dispatches: {err}"),
                    }),
                ));
            }
        }
    }

    Err((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "internal error: query parameters were validated but not handled".into(),
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct UpdateDispatchStatusRequest {
    pub status: String,
}

/// PUT /agent-dispatch/:id/status — update dispatch status.
pub async fn update_dispatch_status_handler(
    State(state): State<Arc<AppState>>,
    Path(dispatch_id): Path<String>,
    Json(req): Json<UpdateDispatchStatusRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if !matches!(
        req.status.as_str(),
        "CLAIMED" | "WORKING" | "PROPOSED" | "COMPLETED"
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "status must be one of: CLAIMED, WORKING, PROPOSED, COMPLETED".into(),
            }),
        ));
    }

    match state
        .storage
        .update_dispatch_status(&dispatch_id, &req.status)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "updated"
        }))),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to update dispatch: {err}"),
            }),
        )),
    }
}
