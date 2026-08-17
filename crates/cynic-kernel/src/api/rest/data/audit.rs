//! Audit trail, proof of history, and verdicts REST handlers.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::rest::response::{coordination_error, storage_error};
use crate::api::rest::types::{AppState, ErrorResponse};

#[derive(Debug, Deserialize, Default)]
pub struct AuditQuery {
    pub limit: Option<u32>,
    pub tool: Option<String>,
    pub agent_id: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

/// GET /audit — MCP audit trail.
pub async fn audit_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<crate::domain::coord::AuditEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(50).min(100);
    match state
        .coord
        .query_audit(
            q.tool.as_deref(),
            q.agent_id.as_deref(),
            q.since.as_deref(),
            q.until.as_deref(),
            limit,
        )
        .await
    {
        Ok(rows) => Ok(Json(rows)),
        Err(e) => {
            tracing::warn!(error = %e, "audit query failed");
            Err(coordination_error())
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct ProofOfHistoryQuery {
    pub limit: Option<u32>,
}

/// GET /proof-of-history — The PoH ledger.
pub async fn proof_of_history_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ProofOfHistoryQuery>,
) -> Result<Json<Vec<crate::domain::storage::ProofOfHistory>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(50).min(100);
    match state.storage.list_proof_of_history(limit).await {
        Ok(rows) => Ok(Json(rows)),
        Err(e) => {
            tracing::warn!(error = %e, "proof of history query failed");
            Err(storage_error())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct VerdictsQuery {
    pub limit: Option<u32>,
}

/// GET /verdicts — list historical verdicts.
pub async fn verdicts_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<VerdictsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(100).min(100);
    match state.storage.list_verdicts(limit).await {
        Ok(rows) => {
            let views: Vec<serde_json::Value> = rows
                .iter()
                .map(|v| serde_json::json!(crate::api::rest::response::verdict_to_response(v)))
                .collect();
            Ok(Json(views))
        }
        Err(e) => {
            tracing::warn!(error = %e, "verdicts query failed");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    }
}
