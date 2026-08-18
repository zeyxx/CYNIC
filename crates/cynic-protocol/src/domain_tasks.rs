//! Domain-Partitioned Task Envelope & Queue Schemas.
//!
//! Enables agents (OMP, Hermes, Codex, Pi, Nightshift) to push and consume
//! structured asynchronous tasks partitioned by domain with strict priority and monotonic budgets.

use serde::{Deserialize, Serialize};

pub const DOMAIN_TASK_SCHEMA: &str = "cynic.domain_task.v1";
pub const DOMAIN_TASK_LIST_SCHEMA: &str = "cynic.domain_task_list.v1";

/// Operating domain for tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDomain {
    /// LLM inference, parametric sweeps, prompt engineering, quantization.
    Inference,
    /// Host infrastructure, DNS, networking, storage, Proxmox, LXC.
    Infra,
    /// Code analysis, compilation gates, architecture boundaries, refactoring.
    CodeAudit,
    /// Epistemic judgments, dog jury evaluation, Askesis causal memory.
    Cortex,
    /// Solana settlement, verification proofs, human governance.
    Governance,
    /// Automated background maintenance and dataset crystallization.
    Nightshift,
}

/// Execution priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    /// Emergency / immediate operator gesture.
    Critical = 4,
    /// Interactive user session.
    User = 3,
    /// Interactive agent workload (Hermes / OMP / Pi).
    Agent = 2,
    /// Autonomous scheduled nightshift.
    Nightshift = 1,
    /// Opportunistic background processing.
    Background = 0,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Agent
    }
}

/// Lifecycle status of a domain task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainTaskStatus {
    Pending,
    Claimed,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Default for DomainTaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// A structured task allocated to a specific operational domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainTask {
    #[serde(default = "default_domain_task_schema")]
    pub schema: String,
    pub id: String,
    pub domain: TaskDomain,
    pub priority: TaskPriority,
    pub title: String,
    pub payload: serde_json::Value,
    pub created_by_agent: String,
    pub created_at_unix_s: u64,
    pub timeout_secs: u64,
    pub status: DomainTaskStatus,
    pub claimed_by_agent: Option<String>,
    pub result_evidence: Option<serde_json::Value>,
    pub error_details: Option<String>,
}

fn default_domain_task_schema() -> String {
    DOMAIN_TASK_SCHEMA.to_string()
}

impl DomainTask {
    pub fn new(
        id: impl Into<String>,
        domain: TaskDomain,
        priority: TaskPriority,
        title: impl Into<String>,
        payload: serde_json::Value,
        created_by_agent: impl Into<String>,
    ) -> Self {
        Self {
            schema: DOMAIN_TASK_SCHEMA.to_string(),
            id: id.into(),
            domain,
            priority,
            title: title.into(),
            payload,
            created_by_agent: created_by_agent.into(),
            created_at_unix_s: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            timeout_secs: 300,
            status: DomainTaskStatus::Pending,
            claimed_by_agent: None,
            result_evidence: None,
            error_details: None,
        }
    }
}
