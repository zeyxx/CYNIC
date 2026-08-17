//! Workflow compliance REST handlers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::rest::response::storage_error;
use crate::api::rest::types::{AppState, ErrorResponse};
use crate::domain::compliance;

/// GET /sessions/{agent_id}/compliance — Score a session's workflow compliance.
pub async fn compliance_handler(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<compliance::SessionCompliance>, (StatusCode, Json<ErrorResponse>)> {
    let observations = state
        .storage
        .list_observations_raw(None, Some(&agent_id), 500)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, agent_id = %agent_id, "compliance: observation query failed");
            storage_error()
        })?;

    let result = compliance::score_session(&agent_id, &agent_id, &observations);

    if let Err(e) = state.storage.store_session_compliance(&result).await {
        tracing::warn!(error = %e, "compliance: failed to persist score");
    }

    if result.score < crate::domain::dog::PHI_INV3 {
        tracing::error!(agent_id = %agent_id, score = %result.score, "compliance: rejected");
        return Err((
            StatusCode::PRECONDITION_FAILED,
            Json(ErrorResponse {
                error: format!(
                    "CRITICAL NON-COMPLIANCE (score={:.3}). Action REJECTED. Warnings: {}",
                    result.score,
                    result.warnings.join("; ")
                ),
            }),
        ));
    }

    if result.score < crate::domain::dog::PHI_INV2 {
        let _ = state
            .event_tx
            .send(crate::domain::events::KernelEvent::Anomaly {
                kind: "low_compliance".to_string(),
                message: format!(
                    "Agent {} compliance={:.3} (threshold={:.3}): {}",
                    result.agent_id,
                    result.score,
                    crate::domain::dog::PHI_INV2,
                    result.warnings.join("; "),
                ),
                severity: "warning".to_string(),
            });
    }

    Ok(Json(result))
}

#[derive(Debug, Deserialize, Default)]
pub struct ComplianceTrendQuery {
    pub limit: Option<u32>,
}

/// GET /compliance — List recent compliance scores (trend tracking).
pub async fn compliance_trend_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ComplianceTrendQuery>,
) -> Result<Json<Vec<compliance::SessionCompliance>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = q.limit.unwrap_or(20).min(100);
    match state.storage.list_session_compliance(limit).await {
        Ok(reports) => Ok(Json(reports)),
        Err(e) => {
            tracing::warn!(error = %e, "compliance trend query failed");
            Err(storage_error())
        }
    }
}
