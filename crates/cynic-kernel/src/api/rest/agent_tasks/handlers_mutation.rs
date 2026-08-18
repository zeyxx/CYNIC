//! Axum route handlers for task dispatch, claim, and completion.

use axum::{extract::State, http::StatusCode, response::Json};
use cynic_protocol::validate_evidence_refs;
use std::sync::Arc;

use crate::api::rest::agent_tasks::types::{
    AGENT_TASK_STORAGE_TIMEOUT, ClaimTaskRequest, DispatchTaskRequest, UpdateTaskResultRequest,
};
use crate::api::rest::agent_tasks::validation::validate_agent_task_content;
use crate::api::rest::types::{AppState, ErrorResponse};
use crate::domain::storage::AgentTask;

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
    if let Err(message) = validate_agent_task_content(&req.kind, &req.domain, &req.content) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: message }),
        ));
    }

    let task_id = format!("agent-task:{}", crate::infra::crypto::generate_secure_id());
    let now = chrono::Utc::now().timestamp_millis();
    let task = AgentTask {
        id: task_id.clone(),
        kind: req.kind,
        domain: req.domain,
        content: req.content,
        status: "pending".to_string(),
        result: None,
        created_at: now,
        completed_at: None,
        agent_id: req.agent_id.clone(),
        claimed_by: None,
        error: None,
        due_at: req.due_at,
        priority: req.priority,
        consumer: req.consumer,
        escalation_policy: req.escalation_policy,
        overdue: false,
        age_seconds: None,
    };

    let task_id = tokio::time::timeout(
        AGENT_TASK_STORAGE_TIMEOUT,
        state.storage.store_agent_task(&task),
    )
    .await
    .map_err(|_e| {
        tracing::warn!(
            "dispatch_task timed out. Storage remediation requires explicit runtime authority."
        );
        (
            StatusCode::GATEWAY_TIMEOUT,
            Json(ErrorResponse {
                error: "Storage timed out while creating agent task. No automatic host remediation was triggered; use an explicit cynic-host runtime authority surface to recover storage_primary.".into(),
            }),
        )
    })?
    .map_err(|e| {
        tracing::warn!(
            error = %e,
            "dispatch_task failed. Storage remediation requires explicit runtime authority."
        );
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Storage unreachable. No automatic host remediation was triggered; use an explicit cynic-host runtime authority surface to recover storage_primary.".into(),
            }),
        )
    })?;

    let _ = state
        .event_tx
        .send(crate::domain::events::KernelEvent::AgentTaskCreated {
            task_id: task_id.clone(),
            kind: task.kind.clone(),
            domain: task.domain.clone(),
        });

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": "pending",
    })))
}

/// POST /agent-tasks/{id}/claim — atomically claim a pending task.
pub async fn claim_task_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
    Json(req): Json<ClaimTaskRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.agent_id.is_empty() || req.agent_id.chars().count() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "agent_id must be 1-64 characters".into(),
            }),
        ));
    }

    let claimed = tokio::time::timeout(
        AGENT_TASK_STORAGE_TIMEOUT,
        state.storage.claim_agent_task(&task_id, &req.agent_id),
    )
    .await
    .map_err(|_e| {
        tracing::warn!(task_id = %task_id, agent_id = %req.agent_id, "claim_task timed out");
        (
            StatusCode::GATEWAY_TIMEOUT,
            Json(ErrorResponse {
                error: "Task claim timed out".into(),
            }),
        )
    })?
    .map_err(|e| {
        tracing::warn!(error = %e, "claim_task failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to claim task".into(),
            }),
        )
    })?;

    if claimed {
        let _ = state
            .event_tx
            .send(crate::domain::events::KernelEvent::AgentTaskClaimed {
                task_id: task_id.clone(),
                agent_id: req.agent_id.clone(),
            });
    }

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "claimed": claimed,
    })))
}

/// POST /agent-tasks/{id}/result — mark task complete with result or failed with error.
pub async fn complete_task_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
    Json(req): Json<UpdateTaskResultRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let _ = tokio::time::timeout(
        AGENT_TASK_STORAGE_TIMEOUT,
        state.storage.claim_agent_task(&task_id, "rest_api"),
    )
    .await;

    let has_error = req.error.is_some();
    let completion_status = if has_error { "failed" } else { "completed" };
    let result_value = req.result.clone();
    let error_value = req.error.clone();
    let evidence_refs = req.evidence_refs.clone();
    if evidence_refs.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "evidence_refs must contain at most 64 entries".into(),
            }),
        ));
    }
    let evidence_violations = validate_evidence_refs(&evidence_refs);
    if !evidence_violations.is_empty() {
        let codes = evidence_violations
            .iter()
            .map(|violation| violation.code.as_str())
            .collect::<Vec<_>>();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("invalid evidence_refs: {}", codes.join(",")),
            }),
        ));
    }
    let completion_payload = serde_json::json!({
        "task_id": task_id.clone(),
        "status": completion_status,
        "result": result_value.clone(),
        "error": error_value.clone(),
        "evidence_refs": evidence_refs,
    });

    tokio::time::timeout(
        AGENT_TASK_STORAGE_TIMEOUT,
        state
            .storage
            .update_agent_task_result(&task_id, "rest_api", result_value, error_value),
    )
    .await
    .map_err(|_e| {
        tracing::warn!(task_id = %task_id, "complete_task timed out");
        (
            StatusCode::GATEWAY_TIMEOUT,
            Json(ErrorResponse {
                error: "Failed to update task before timeout".into(),
            }),
        )
    })?
    .map_err(|e| {
        tracing::warn!(error = %e, "complete_task failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to update task".into(),
            }),
        )
    })?;

    let mut completion_observation = crate::domain::ccm::build_observation_with_ledger(
        "agent_task_result".to_string(),
        Some(task_id.clone()),
        Some("agent-tasks".to_string()),
        Some(completion_status.to_string()),
        Some(completion_payload.to_string()),
        Some("CYNIC".to_string()),
        Some("rest_api".to_string()),
        None,
        Some(vec![format!("agent-task:{task_id}")]),
        Some(completion_payload),
        Some(if has_error {
            "inferred".to_string()
        } else {
            "observed".to_string()
        }),
        Some("agent-task-completion-loop".to_string()),
        Some("record_agent_task_result".to_string()),
        Some(vec![format!("agent-task:{task_id}")]),
        None,
    );
    completion_observation.source_tier =
        crate::introspection::classify_source_tier("rest_api").to_string();

    match state.bg_semaphore.clone().try_acquire_owned() {
        Ok(permit) => {
            let storage = Arc::clone(&state.storage);
            let task_id_clone = task_id.clone();
            let obs_clone = completion_observation;
            state.metrics.inc_observation_ingested();
            state.bg_tasks.spawn(async move {
                let _permit = permit;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    storage.store_observation(&obs_clone),
                )
                .await
                {
                    Ok(Err(e)) => {
                        tracing::warn!(error = %e, task_id = task_id_clone, "agent task completion observation store failed")
                    }
                    Err(_) => tracing::warn!(
                        task_id = task_id_clone,
                        "agent task completion observation store timed out (5s)"
                    ),
                    Ok(Ok(_)) => {
                        tracing::debug!(task_id = task_id_clone, "agent task completion observation stored")
                    }
                }
            });
        }
        Err(_) => {
            state.metrics.inc_observation_dropped();
            tracing::warn!(
                task_id = task_id,
                "agent task completion observation dropped: background limit reached"
            );
            state.metrics.inc_observation_post_failed();
        }
    }

    let _ = state
        .event_tx
        .send(crate::domain::events::KernelEvent::AgentTaskCompleted {
            task_id: task_id.clone(),
        });

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": if has_error { "failed" } else { "completed" },
    })))
}
