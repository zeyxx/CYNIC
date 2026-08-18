//! Validation, projection, and event matching logic for agent tasks and proposals.

use std::time::Duration;

use crate::api::forge_task_contract::validate_repo_affecting_hermes_task;
use crate::api::rest::agent_tasks::types::{
    AGENT_PROPOSAL_REVIEW_SCHEMA, AGENT_PROPOSAL_SCHEMA, ReviewAgentProposalRequest,
    VALID_AGENT_PROPOSAL_REVIEW_STATES,
};
use crate::domain::events::KernelEvent;
use crate::domain::storage::AgentTask;

pub fn validate_agent_task_content(kind: &str, domain: &str, content: &str) -> Result<(), String> {
    validate_repo_affecting_hermes_task(kind, domain, content)
}

pub fn extract_agent_proposal(task: &AgentTask) -> Option<serde_json::Value> {
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

pub fn task_proposal_projection(task: &AgentTask) -> Option<serde_json::Value> {
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

pub fn filter_proposal_projection(
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

pub fn validate_agent_proposal_review_request(
    req: &ReviewAgentProposalRequest,
) -> Result<(), String> {
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

pub fn agent_proposal_review_value(
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

pub fn event_matches_pending_task_watch(
    event: &KernelEvent,
    kind: &str,
    domain: Option<&str>,
) -> bool {
    match event {
        KernelEvent::AgentTaskCreated {
            kind: event_kind,
            domain: event_domain,
            ..
        } => event_kind == kind && domain.is_none_or(|wanted| event_domain == wanted),
        _ => false,
    }
}

pub async fn wait_for_matching_task_event(
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
