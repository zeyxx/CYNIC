//! REST handlers for agent task queue — dispatch, poll, complete.
//! Mirrors MCP agent_tools for HTTP clients (Python daemons, curl, future brain agent).

use axum::{extract::State, http::StatusCode, response::Json};
use cynic_protocol::agent_recursion::AgentEvidenceRef;
use cynic_protocol::validate_evidence_refs;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

use super::types::{AppState, ErrorResponse};
use crate::api::forge_task_contract::validate_repo_affecting_hermes_task;
use crate::domain::events::KernelEvent;
use crate::domain::storage::AgentTask;

const AGENT_TASK_STORAGE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
pub struct DispatchTaskRequest {
    pub kind: String,
    pub domain: String,
    pub content: String,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub consumer: Option<String>,
    #[serde(default)]
    pub escalation_policy: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskResultRequest {
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<AgentEvidenceRef>,
}

#[derive(Debug, Deserialize)]
pub struct ClaimTaskRequest {
    pub agent_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewAgentProposalRequest {
    pub state: String,
    #[serde(default)]
    pub reviewer: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub public_eligible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct WatchTasksQuery {
    #[serde(default = "default_task_kind")]
    pub kind: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default = "default_task_limit")]
    pub limit: u32,
    #[serde(default = "default_watch_timeout_secs")]
    pub timeout_secs: u64,
}

const AGENT_PROPOSAL_SCHEMA: &str = "cynic.agent_proposal.v1";
const AGENT_PROPOSAL_REVIEW_SCHEMA: &str = "cynic.agent_proposal_review.v1";
const VALID_AGENT_PROPOSAL_REVIEW_STATES: &[&str] =
    &["approved", "rejected", "needs_redaction", "needs_evidence"];

fn default_task_kind() -> String {
    "hermes".to_string()
}

fn default_task_limit() -> u32 {
    10
}

fn default_watch_timeout_secs() -> u64 {
    30
}

fn validate_agent_task_content(kind: &str, domain: &str, content: &str) -> Result<(), String> {
    validate_repo_affecting_hermes_task(kind, domain, content)
}

fn extract_agent_proposal(task: &AgentTask) -> Option<serde_json::Value> {
    let parsed: serde_json::Value = serde_json::from_str(&task.content).ok()?;
    if parsed.get("schema").and_then(|value| value.as_str()) == Some(AGENT_PROPOSAL_SCHEMA) {
        return Some(parsed);
    }
    parsed
        .get("operator_interface_proposal")
        .filter(|proposal| {
            proposal.get("schema").and_then(|value| value.as_str()) == Some(AGENT_PROPOSAL_SCHEMA)
        })
        .cloned()
}

fn task_proposal_projection(task: &AgentTask) -> Option<serde_json::Value> {
    let proposal = extract_agent_proposal(task)?;
    Some(serde_json::json!({
        "schema": "cynic.agent_proposal_task_projection.v1",
        "task": {
            "id": task.id.clone(),
            "kind": task.kind.clone(),
            "domain": task.domain.clone(),
            "status": task.status.clone(),
            "created_at": task.created_at,
            "completed_at": task.completed_at.clone(),
            "agent_id": task.agent_id.clone(),
            "claimed_by": task.claimed_by.clone(),
            "consumer": task.consumer.clone(),
            "overdue": task.overdue,
            "age_seconds": task.age_seconds,
        },
        "proposal": proposal,
    }))
}

fn filter_proposal_projection(
    projection: &serde_json::Value,
    visibility: Option<&str>,
    review_state: Option<&str>,
) -> bool {
    let proposal = projection
        .get("proposal")
        .unwrap_or(&serde_json::Value::Null);
    if let Some(visibility) = visibility
        && proposal.get("visibility").and_then(|value| value.as_str()) != Some(visibility)
    {
        return false;
    }
    if let Some(review_state) = review_state
        && proposal
            .get("operator_review")
            .and_then(|value| value.get("state"))
            .and_then(|value| value.as_str())
            != Some(review_state)
    {
        return false;
    }
    true
}

fn validate_agent_proposal_review_request(req: &ReviewAgentProposalRequest) -> Result<(), String> {
    if !VALID_AGENT_PROPOSAL_REVIEW_STATES.contains(&req.state.as_str()) {
        return Err(
            "state must be approved, rejected, needs_redaction, or needs_evidence".to_string(),
        );
    }
    if req
        .reviewer
        .as_deref()
        .unwrap_or("operator")
        .chars()
        .count()
        > 96
    {
        return Err("reviewer must be at most 96 characters".to_string());
    }
    if req.reason.as_deref().unwrap_or_default().chars().count() > 2000 {
        return Err("reason must be at most 2000 characters".to_string());
    }
    if req.public_eligible == Some(true) && req.state != "approved" {
        return Err("public_eligible=true requires state=approved".to_string());
    }
    Ok(())
}

fn agent_proposal_review_value(
    proposal_id: &str,
    req: &ReviewAgentProposalRequest,
) -> serde_json::Value {
    serde_json::json!({
        "schema": AGENT_PROPOSAL_REVIEW_SCHEMA,
        "proposal_id": proposal_id,
        "state": req.state.as_str(),
        "reviewer": req.reviewer.as_deref().unwrap_or("operator"),
        "reason": req.reason.as_deref().unwrap_or(""),
        "public_eligible": req.public_eligible.unwrap_or(false),
        "reviewed_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    })
}

async fn list_pending_tasks(
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

fn event_matches_pending_task_watch(event: &KernelEvent, kind: &str, domain: Option<&str>) -> bool {
    match event {
        KernelEvent::AgentTaskCreated {
            kind: event_kind,
            domain: event_domain,
            ..
        } => event_kind == kind && domain.is_none_or(|wanted| event_domain == wanted),
        _ => false,
    }
}

async fn wait_for_matching_task_event(
    rx: &mut tokio::sync::broadcast::Receiver<KernelEvent>,
    kind: &str,
    domain: Option<&str>,
    timeout_secs: u64,
) -> bool {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs.clamp(1, 300));
    loop {
        let recv = tokio::time::timeout_at(deadline, rx.recv()).await;
        let Ok(result) = recv else {
            return false;
        };
        match result {
            Ok(event) => {
                if event_matches_pending_task_watch(&event, kind, domain) {
                    return true;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                return true;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                return false;
            }
        }
    }
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

    let task_id = tokio::time::timeout(AGENT_TASK_STORAGE_TIMEOUT, state.storage.store_agent_task(&task))
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

/// POST /agent-tasks/{id}/result — mark task complete with result or failed with error.
pub async fn complete_task_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
    Json(req): Json<UpdateTaskResultRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Mark processing first
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

#[cfg(test)]
mod tests {
    use super::{
        ReviewAgentProposalRequest, agent_proposal_review_value, event_matches_pending_task_watch,
        extract_agent_proposal, filter_proposal_projection, task_proposal_projection,
        validate_agent_proposal_review_request, validate_agent_task_content,
        wait_for_matching_task_event,
    };
    use crate::domain::events::KernelEvent;
    use crate::domain::storage::AgentTask;

    fn activation_evidence_contract() -> serde_json::Value {
        serde_json::json!({
            "patch_field": "activation_evidence_patch",
            "node_field": "node",
            "runtime_paths_field": "activation_evidence.runtime_paths",
            "services_field": "activation_evidence.services",
            "verified_requires": ["marker_exists", "sentinels_verified"],
            "service_active_requires": ["declared", "active"],
            "ingestion_mode": "validated_merge",
        })
    }

    #[test]
    fn validates_forge_task_with_coord_targets() {
        let registry = serde_json::json!({
            "lifecycle": {
                "kernel_release": {
                    "orchestrator_organ": "organ-forge",
                    "build_runner_role": "build_runner",
                    "deploy_target_role": "kernel_primary",
                    "deploy_target_flag": "deploy_target",
                    "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
                    "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
                    "activation_evidence_contract": activation_evidence_contract()
                }
            },
            "nodes": [
                {"name": "cynic-vm-anvil", "role": "build_runner"},
                {"name": "cynic-vm-core", "role": "kernel_primary", "deploy_target": true}
            ]
        });
        let content = serde_json::json!({
            "lifecycle": "kernel_release",
            "objective": "build kernel release",
            "orchestrator_organ": "organ-forge",
            "build_target": "cynic-vm-anvil",
            "build_runner_role": "build_runner",
            "deploy_target": "cynic-vm-core",
            "deploy_target_role": "kernel_primary",
            "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
            "activation_evidence_contract": activation_evidence_contract(),
            "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
        });

        let result = cynic_host::lifecycle::forge_task_contract::validate_forge_lifecycle_contract(
            &content,
            Some(&registry),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_forge_task_without_declared_targets() {
        let content = serde_json::json!({
            "objective": "build kernel release"
        })
        .to_string();

        let result = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(
            result.is_err(),
            "repo-affecting forge tasks should declare coordination targets"
        );
    }

    fn task_with_content(content: &serde_json::Value) -> AgentTask {
        AgentTask {
            id: "agent_tasks:test".to_string(),
            kind: "hermes".to_string(),
            domain: "organ-lifecycle".to_string(),
            content: content.to_string(),
            status: "pending".to_string(),
            result: None,
            created_at: 1_772_480_000_000,
            completed_at: None,
            agent_id: Some("organ-lifecycle-convergence".to_string()),
            claimed_by: None,
            error: None,
            due_at: None,
            priority: Some(95),
            consumer: Some("organ-lifecycle-hermes-agent".to_string()),
            escalation_policy: None,
            overdue: false,
            age_seconds: Some(12),
        }
    }

    #[test]
    fn extracts_operator_interface_proposal_from_hermes_task_content() {
        let task = task_with_content(&serde_json::json!({
            "schema": "cynic.dependency_convergence_hermes_task.v1",
            "operator_interface_proposal": {
                "schema": "cynic.agent_proposal.v1",
                "proposal_id": "dependency-convergence-test",
                "visibility": "operator_private",
                "operator_review": {"state": "pending"},
                "public_projection": {"eligible": false, "redaction_status": "needs_review"}
            }
        }));

        let proposal = extract_agent_proposal(&task).expect("proposal should be extracted");

        assert_eq!(
            proposal.get("schema").and_then(|value| value.as_str()),
            Some("cynic.agent_proposal.v1")
        );
        assert_eq!(
            proposal.get("visibility").and_then(|value| value.as_str()),
            Some("operator_private")
        );
    }

    #[test]
    fn projects_agent_proposal_for_operator_listing() {
        let task = task_with_content(&serde_json::json!({
            "operator_interface_proposal": {
                "schema": "cynic.agent_proposal.v1",
                "proposal_id": "dependency-convergence-test",
                "visibility": "operator_private",
                "operator_review": {"state": "pending"},
                "public_projection": {"eligible": false, "redaction_status": "needs_review"}
            }
        }));

        let projection = task_proposal_projection(&task).expect("projection should exist");

        assert_eq!(
            projection.get("schema").and_then(|value| value.as_str()),
            Some("cynic.agent_proposal_task_projection.v1")
        );
        assert_eq!(
            projection
                .get("task")
                .and_then(|value| value.get("consumer"))
                .and_then(|value| value.as_str()),
            Some("organ-lifecycle-hermes-agent")
        );
        assert!(filter_proposal_projection(
            &projection,
            Some("operator_private"),
            Some("pending")
        ));
        assert!(!filter_proposal_projection(
            &projection,
            Some("public"),
            Some("pending")
        ));
    }

    #[test]
    fn validates_agent_proposal_review_request() {
        let request = ReviewAgentProposalRequest {
            state: "approved".to_string(),
            reviewer: Some("operator".to_string()),
            reason: Some("proof inspected".to_string()),
            public_eligible: Some(true),
        };

        let result = validate_agent_proposal_review_request(&request);

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_public_eligible_without_approval() {
        let request = ReviewAgentProposalRequest {
            state: "needs_redaction".to_string(),
            reviewer: Some("operator".to_string()),
            reason: None,
            public_eligible: Some(true),
        };

        let result = validate_agent_proposal_review_request(&request);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires state=approved"));
    }

    #[test]
    fn builds_agent_proposal_review_observation_payload() {
        let request = ReviewAgentProposalRequest {
            state: "rejected".to_string(),
            reviewer: None,
            reason: Some("insufficient evidence".to_string()),
            public_eligible: None,
        };

        let review = agent_proposal_review_value("proposal-1", &request);

        assert_eq!(
            review.get("schema").and_then(|value| value.as_str()),
            Some("cynic.agent_proposal_review.v1")
        );
        assert_eq!(
            review.get("proposal_id").and_then(|value| value.as_str()),
            Some("proposal-1")
        );
        assert_eq!(
            review.get("reviewer").and_then(|value| value.as_str()),
            Some("operator")
        );
    }

    #[test]
    fn rejects_forge_task_without_lifecycle_contract() {
        let content = serde_json::json!({
            "objective": "build kernel release",
            "coord_targets": ["crates/cynic-kernel", "infra/registry.json"]
        })
        .to_string();

        let result = validate_agent_task_content("hermes", "organ-forge", &content);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("declare lifecycle"));
    }

    #[test]
    fn validates_forge_task_against_dynamic_registry_contract() {
        let registry = serde_json::json!({
            "lifecycle": {
                "kernel_release": {
                    "orchestrator_organ": "organ-build",
                    "build_runner_role": "builder",
                    "deploy_target_role": "primary",
                    "deploy_target_flag": "ships_kernel",
                    "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
                    "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
                    "activation_evidence_contract": activation_evidence_contract()
                }
            },
            "nodes": [
                {"name": "node-builder", "role": "builder"},
                {"name": "node-primary", "role": "primary", "ships_kernel": true}
            ]
        });
        let content = serde_json::json!({
            "lifecycle": "kernel_release",
            "orchestrator_organ": "organ-build",
            "build_target": "node-builder",
            "build_runner_role": "builder",
            "deploy_target": "node-primary",
            "deploy_target_role": "primary",
            "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
            "activation_evidence_contract": activation_evidence_contract(),
            "coord_targets": ["infra/registry.json"]
        });

        let result = cynic_host::lifecycle::forge_task_contract::validate_forge_lifecycle_contract(
            &content,
            Some(&registry),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_forge_task_that_does_not_match_dynamic_registry_contract() {
        let registry = serde_json::json!({
            "lifecycle": {
                "kernel_release": {
                    "orchestrator_organ": "organ-build",
                    "build_runner_role": "builder",
                    "deploy_target_role": "primary",
                    "deploy_target_flag": "ships_kernel",
                    "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
                    "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
                    "activation_evidence_contract": activation_evidence_contract()
                }
            },
            "nodes": [
                {"name": "node-builder", "role": "builder"},
                {"name": "node-primary", "role": "primary", "ships_kernel": true}
            ]
        });
        let content = serde_json::json!({
            "lifecycle": "kernel_release",
            "orchestrator_organ": "organ-forge",
            "build_target": "node-builder",
            "build_runner_role": "builder",
            "deploy_target": "node-primary",
            "deploy_target_role": "primary",
            "runtime_paths": {"repo_root": "/var/lib/cynic/repos/CYNIC"},
            "runtime_path_proofs": {"repo_root": {"kind": "git_worktree", "marker": ".git", "sentinels": ["AGENTS.md", "infra/registry.json"]}},
            "activation_evidence_contract": activation_evidence_contract(),
            "coord_targets": ["infra/registry.json"]
        });

        let result = cynic_host::lifecycle::forge_task_contract::validate_forge_lifecycle_contract(
            &content,
            Some(&registry),
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("match registry orchestrator_organ")
        );
    }

    #[test]
    fn allows_non_forge_task_without_declared_targets() {
        let result =
            validate_agent_task_content("hermes", "telegram", r#"{"objective":"observe"}"#);

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_invalid_json_for_forge_task() {
        let result = validate_agent_task_content("hermes", "organ-forge", "not-json");

        assert!(result.is_err());
    }

    #[test]
    fn agent_task_created_event_matches_watch_filters() {
        let event = KernelEvent::AgentTaskCreated {
            task_id: "agent_tasks:test".to_string(),
            kind: "hermes".to_string(),
            domain: "forge".to_string(),
        };

        assert!(event_matches_pending_task_watch(
            &event,
            "hermes",
            Some("forge")
        ));
        assert!(!event_matches_pending_task_watch(
            &event,
            "judge",
            Some("forge")
        ));
        assert!(!event_matches_pending_task_watch(
            &event,
            "hermes",
            Some("token")
        ));
    }

    #[tokio::test]
    async fn task_watch_waiter_wakes_for_matching_event() {
        let (tx, mut rx) = tokio::sync::broadcast::channel(8);
        let waiter = tokio::spawn(async move {
            wait_for_matching_task_event(&mut rx, "hermes", Some("forge"), 1).await
        });

        let _ = tx.send(KernelEvent::AgentTaskCreated {
            task_id: "agent_tasks:test".to_string(),
            kind: "hermes".to_string(),
            domain: "forge".to_string(),
        });

        assert!(waiter.await.unwrap());
    }

    #[tokio::test]
    async fn task_watch_waiter_times_out_without_matching_event() {
        let (_tx, mut rx) = tokio::sync::broadcast::channel(8);
        assert!(!wait_for_matching_task_event(&mut rx, "hermes", Some("forge"), 1).await);
    }
}
