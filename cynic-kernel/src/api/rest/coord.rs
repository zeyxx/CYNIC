//! REST handlers for coordination — POST /coord/register, /coord/claim, /coord/release.
//! These expose the same CoordPort methods as MCP, enabling hooks and external
//! clients to coordinate without MCP stdio.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::coord::ClaimResult;

// ── Request types ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub agent_id: String,
    pub intent: String,
    pub agent_type: Option<String>,
}

#[derive(Deserialize)]
pub struct ClaimRequest {
    pub agent_id: String,
    pub target: String,
    pub claim_type: Option<String>,
}

#[derive(Deserialize)]
pub struct ReleaseRequest {
    pub agent_id: String,
    pub target: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────

pub async fn coord_register_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.agent_id.is_empty() || req.agent_id.len() > 64 {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "agent_id must be 1-64 characters".into(),
        })));
    }

    let agent_type = req.agent_type.unwrap_or_else(|| "unknown".into());

    state.coord.register_agent(&req.agent_id, &agent_type, &req.intent).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Register failed: {}", e),
        })))?;

    let _ = state.coord.store_audit("coord_register", &req.agent_id, &serde_json::json!({
        "intent": req.intent, "agent_type": agent_type, "source": "rest",
    })).await;

    Ok(Json(serde_json::json!({
        "status": "registered",
        "agent_id": req.agent_id,
    })))
}

pub async fn coord_claim_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.agent_id.is_empty() || req.target.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "agent_id and target are required".into(),
        })));
    }

    let claim_type = req.claim_type.unwrap_or_else(|| "file".into());

    match state.coord.claim(&req.agent_id, &req.target, &claim_type).await {
        Ok(ClaimResult::Claimed) => {
            let _ = state.coord.store_audit("coord_claim", &req.agent_id, &serde_json::json!({
                "target": req.target, "claim_type": claim_type, "source": "rest",
            })).await;
            Ok(Json(serde_json::json!({
                "status": "claimed",
                "agent_id": req.agent_id,
                "target": req.target,
            })))
        }
        Ok(ClaimResult::Conflict(conflicts)) => {
            Err((StatusCode::CONFLICT, Json(ErrorResponse {
                error: format!("CONFLICT: '{}' already claimed by: {}",
                    req.target,
                    conflicts.iter().map(|c| c.agent_id.as_str()).collect::<Vec<_>>().join(", ")),
            })))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Claim failed: {}", e),
        }))),
    }
}

pub async fn coord_release_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ReleaseRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.agent_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "agent_id is required".into(),
        })));
    }

    let desc = state.coord.release(&req.agent_id, req.target.as_deref()).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: format!("Release failed: {}", e),
        })))?;

    let _ = state.coord.store_audit("coord_release", &req.agent_id, &serde_json::json!({
        "target": req.target, "source": "rest",
    })).await;

    Ok(Json(serde_json::json!({
        "status": "released",
        "agent_id": req.agent_id,
        "detail": desc,
    })))
}
