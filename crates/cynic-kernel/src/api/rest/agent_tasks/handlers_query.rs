//! Read-only Axum route handlers for polling and watching agent tasks.

use axum::{extract::State, http::StatusCode, response::Json};
use std::sync::Arc;

use crate::api::rest::agent_tasks::types::{AGENT_TASK_STORAGE_TIMEOUT, WatchTasksQuery};
use crate::api::rest::agent_tasks::validation::wait_for_matching_task_event;
use crate::api::rest::types::{AppState, ErrorResponse};
use crate::domain::storage::AgentTask;

pub async fn list_pending_tasks(
    state: &Arc<AppState>,
    kind: &str,
    domain: Option<&str>,
    limit: u32,
) -> Result<Vec<AgentTask>, (StatusCode, Json<ErrorResponse>)> {
    tokio::time::timeout(
        AGENT_TASK_STORAGE_TIMEOUT,
        state.storage.list_pending_agent_tasks(kind, domain, limit),
    )
    .await
    .map_err(|_e| {
        tracing::warn!(kind, domain, limit, "list_pending_agent_tasks timed out");
        (
            StatusCode::GATEWAY_TIMEOUT,
            Json(ErrorResponse {
                error: "Task storage timed out".into(),
            }),
        )
    })?
    .map_err(|e| {
        tracing::warn!(error = %e, kind, domain, limit, "list_pending_agent_tasks failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to list tasks".into(),
            }),
        )
    })
}

/// GET /agent-tasks?kind=hermes&domain=organ-anvil&limit=10 — poll pending tasks.
pub async fn list_tasks_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let kind = params.get("kind").map(|s| s.as_str()).unwrap_or("hermes");
    let domain = params
        .get("domain")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty());
    let limit: u32 = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let tasks = list_pending_tasks(&state, kind, domain, limit).await?;

    Ok(Json(serde_json::json!({
        "tasks": tasks,
        "count": tasks.len(),
    })))
}

/// GET /agent-tasks/watch?kind=hermes&domain=organ-anvil&timeout_secs=30 — long-poll pending tasks.
pub async fn watch_tasks_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<WatchTasksQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let kind = query.kind.trim();
    if kind.is_empty() || kind.chars().count() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "kind must be 1-64 characters".into(),
            }),
        ));
    }

    let domain = query.domain.as_deref().filter(|value| !value.is_empty());
    let limit = query.limit.clamp(1, 100);
    let initial = list_pending_tasks(&state, kind, domain, limit).await?;
    if !initial.is_empty() {
        return Ok(Json(serde_json::json!({
            "tasks": initial,
            "count": initial.len(),
            "watch": "immediate",
        })));
    }

    let mut rx = state.event_tx.subscribe();
    let woke = wait_for_matching_task_event(&mut rx, kind, domain, query.timeout_secs).await;
    let tasks = list_pending_tasks(&state, kind, domain, limit).await?;
    Ok(Json(serde_json::json!({
        "tasks": tasks,
        "count": tasks.len(),
        "watch": if woke { "event" } else { "timeout" },
    })))
}

/// GET /agent-tasks/completed?kind=hermes&limit=10 — poll resolved tasks.
pub async fn list_completed_tasks_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let kind = params.get("kind").map(|s| s.as_str()).unwrap_or("hermes");
    let domain = params
        .get("domain")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty());
    let limit: u32 = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let tasks = tokio::time::timeout(
        AGENT_TASK_STORAGE_TIMEOUT,
        state
            .storage
            .list_completed_agent_tasks(kind, domain, limit),
    )
    .await
    .map_err(|_e| {
        tracing::warn!(kind, domain, limit, "list_completed_tasks timed out");
        (
            StatusCode::GATEWAY_TIMEOUT,
            Json(ErrorResponse {
                error: "Completed task storage timed out".into(),
            }),
        )
    })?
    .map_err(|e| {
        tracing::warn!(error = %e, "list_completed_tasks failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to list completed tasks".into(),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "tasks": tasks,
        "count": tasks.len(),
    })))
}
