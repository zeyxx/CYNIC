//! REST handlers for agent task queue — dispatch, poll, complete.
//! Mirrors MCP agent_tools for HTTP clients (Python daemons, curl, future brain agent).

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Deserialize;
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};
use crate::domain::storage::AgentTask;

#[derive(Debug, Deserialize)]
pub struct DispatchTaskRequest {
    pub kind: String,
    pub domain: String,
    pub content: String,
    #[serde(default)]
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskResultRequest {
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// POST /agent-tasks — dispatch a new task to the queue.
pub async fn dispatch_task_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DispatchTaskRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.kind.is_empty() || req.kind.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "kind must be 1-64 characters".into(),
            }),
        ));
    }
    if req.content.is_empty() || req.content.chars().count() > 10_000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "content must be 1-10000 characters".into(),
            }),
        ));
    }

    let task_id = format!("agent-task:{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();
    let task = AgentTask {
        id: task_id.clone(),
        kind: req.kind,
        domain: req.domain,
        content: req.content,
        status: "pending".to_string(),
        result: None,
        created_at: now,
        completed_at: None,
        agent_id: req.agent_id,
        error: None,
    };

    state.storage.store_agent_task(&task).await.map_err(|e| {
        tracing::warn!(error = %e, "dispatch_task failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to store task".into(),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": "pending",
    })))
}

/// GET /agent-tasks?kind=hermes&limit=10 — poll pending tasks.
pub async fn list_tasks_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let kind = params.get("kind").map(|s| s.as_str()).unwrap_or("hermes");
    let limit: u32 = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let tasks = state
        .storage
        .list_pending_agent_tasks(kind, limit)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "list_tasks failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list tasks".into(),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "tasks": tasks,
        "count": tasks.len(),
    })))
}

/// POST /agent-tasks/{id}/result — mark task complete with result or error.
pub async fn complete_task_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
    Json(req): Json<UpdateTaskResultRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Mark processing first
    let _ = state.storage.mark_agent_task_processing(&task_id).await;

    state
        .storage
        .update_agent_task_result(&task_id, req.result, req.error)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "complete_task failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update task".into(),
                }),
            )
        })?;

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": "completed",
    })))
}
