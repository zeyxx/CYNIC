//! Types and constants for agent tasks REST endpoints.

use cynic_protocol::agent_recursion::AgentEvidenceRef;
use serde::Deserialize;
use std::time::Duration;

pub const AGENT_TASK_STORAGE_TIMEOUT: Duration = Duration::from_secs(5);
pub const AGENT_PROPOSAL_SCHEMA: &str = "cynic.agent_proposal.v1";
pub const AGENT_PROPOSAL_REVIEW_SCHEMA: &str = "cynic.agent_proposal_review.v1";
pub const VALID_AGENT_PROPOSAL_REVIEW_STATES: &[&str] =
    &["approved", "rejected", "needs_redaction", "needs_evidence"];

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

pub fn default_task_kind() -> String {
    "hermes".to_string()
}

pub fn default_task_limit() -> u32 {
    10
}

pub fn default_watch_timeout_secs() -> u64 {
    30
}
