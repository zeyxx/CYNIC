//! Axum route handlers for operator agent proposals and reviews.

use axum::{extract::State, http::StatusCode, response::Json};
use std::sync::Arc;

use crate::api::rest::agent_tasks::types::{
    ReviewAgentProposalRequest, AGENT_PROPOSAL_REVIEW_SCHEMA, AGENT_TASK_STORAGE_TIMEOUT,
};
use crate::api::rest::agent_tasks::validation::{
    agent_proposal_review_value, filter_proposal_projection, task_proposal_projection,
    validate_agent_proposal_review_request,
};
use crate::api::rest::types::{AppState, ErrorResponse};

/// GET /agent-proposals?kind=hermes&status=pending — operator proposal projection.
pub async fn list_agent_proposals_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let kind = params.get("kind").map(|s| s.as_str()).unwrap_or("hermes");
    let domain = params
        .get("domain")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty());
    let status = params
        .get("status")
        .map(|s| s.as_str())
        .unwrap_or("pending");
    let visibility = params
        .get("visibility")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty());
    let review_state = params
        .get("review_state")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty());
    let limit: u32 = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(25)
        .clamp(1, 100);

    let mut tasks = match status {
        "pending" => tokio::time::timeout(
            AGENT_TASK_STORAGE_TIMEOUT,
            state.storage.list_pending_agent_tasks(kind, domain, limit),
        )
        .await
        .map_err(|_e| {
            tracing::warn!(
                kind,
                domain,
                limit,
                "list_agent_proposals pending timed out"
            );
            (
                StatusCode::GATEWAY_TIMEOUT,
                Json(ErrorResponse {
                    error: "Pending proposal storage timed out".into(),
                }),
            )
        })?
        .map_err(|e| {
            tracing::warn!(error = %e, "list_agent_proposals pending failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list agent proposals".into(),
                }),
            )
        })?,
        "completed" => tokio::time::timeout(
            AGENT_TASK_STORAGE_TIMEOUT,
            state
                .storage
                .list_completed_agent_tasks(kind, domain, limit),
        )
        .await
        .map_err(|_e| {
            tracing::warn!(
                kind,
                domain,
                limit,
                "list_agent_proposals completed timed out"
            );
            (
                StatusCode::GATEWAY_TIMEOUT,
                Json(ErrorResponse {
                    error: "Completed proposal storage timed out".into(),
                }),
            )
        })?
        .map_err(|e| {
            tracing::warn!(error = %e, "list_agent_proposals completed failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list agent proposals".into(),
                }),
            )
        })?,
        "all" => {
            let mut pending = tokio::time::timeout(
                AGENT_TASK_STORAGE_TIMEOUT,
                state.storage.list_pending_agent_tasks(kind, domain, limit),
            )
            .await
            .map_err(|_e| {
                tracing::warn!(
                    kind,
                    domain,
                    limit,
                    "list_agent_proposals all/pending timed out"
                );
                (
                    StatusCode::GATEWAY_TIMEOUT,
                    Json(ErrorResponse {
                        error: "Pending proposal storage timed out".into(),
                    }),
                )
            })?
            .map_err(|e| {
                tracing::warn!(error = %e, "list_agent_proposals all/pending failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to list agent proposals".into(),
                    }),
                )
            })?;
            let mut completed = tokio::time::timeout(
                AGENT_TASK_STORAGE_TIMEOUT,
                state
                    .storage
                    .list_completed_agent_tasks(kind, domain, limit),
            )
            .await
            .map_err(|_e| {
                tracing::warn!(
                    kind,
                    domain,
                    limit,
                    "list_agent_proposals all/completed timed out"
                );
                (
                    StatusCode::GATEWAY_TIMEOUT,
                    Json(ErrorResponse {
                        error: "Completed proposal storage timed out".into(),
                    }),
                )
            })?
            .map_err(|e| {
                tracing::warn!(error = %e, "list_agent_proposals all/completed failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to list agent proposals".into(),
                    }),
                )
            })?;
            pending.append(&mut completed);
            pending
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "status must be pending, completed, or all".into(),
                }),
            ));
        }
    };

    tasks.truncate(limit as usize);
    let proposals: Vec<_> = tasks
        .iter()
        .filter_map(task_proposal_projection)
        .filter(|projection| filter_proposal_projection(projection, visibility, review_state))
        .collect();

    Ok(Json(serde_json::json!({
        "schema": "cynic.agent_proposal_listing.v1",
        "status": status,
        "kind": kind,
        "domain": domain,
        "proposals": proposals,
        "count": proposals.len(),
    })))
}

/// POST /agent-proposals/{proposal_id}/review — append operator review evidence.
pub async fn review_agent_proposal_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(proposal_id): axum::extract::Path<String>,
    Json(req): Json<ReviewAgentProposalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if proposal_id.is_empty() || proposal_id.chars().count() > 160 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "proposal_id must be 1-160 characters".into(),
            }),
        ));
    }
    if let Err(message) = validate_agent_proposal_review_request(&req) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: message }),
        ));
    }

    let value = agent_proposal_review_value(&proposal_id, &req);
    let obs = crate::domain::ccm::build_observation_with_ledger(
        "agent_proposal_review".to_string(),
        Some(proposal_id.clone()),
        Some("agent-proposals".to_string()),
        Some(req.state.clone()),
        Some(value.to_string()),
        Some("CYNIC".to_string()),
        Some(
            req.reviewer
                .clone()
                .unwrap_or_else(|| "operator".to_string()),
        ),
        None,
        Some(vec!["agent-proposal-review".to_string(), req.state.clone()]),
        Some(value.clone()),
        Some("observed".to_string()),
        Some("operator-interface".to_string()),
        Some("review_agent_proposal".to_string()),
        Some(vec![format!("agent_proposals:{proposal_id}")]),
        Some(1.0),
    );
    state.storage.store_observation(&obs).await.map_err(|e| {
        tracing::warn!(error = %e, proposal_id = %proposal_id, "agent proposal review observation failed");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Failed to persist proposal review".into(),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "review_recorded",
        "schema": AGENT_PROPOSAL_REVIEW_SCHEMA,
        "proposal_id": proposal_id,
        "review": value,
    })))
}
