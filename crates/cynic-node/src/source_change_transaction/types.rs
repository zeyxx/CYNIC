use cynic_workload::{
    AgentWorkloadEnvelope,
    AgentWorkspaceApplyReport,
    AgentWorkspaceChange,
    AgentWorkspaceReconciliation,
    AgentWorkspaceRetentionRoot,
    AgentWorkspaceZone,
};
use serde::{Deserialize, Serialize};

pub(crate) const AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA: &str =
    "cynic.agent_source_change_transaction_result.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AgentSourceChangeTransactionRequest {
    pub workload: AgentWorkloadEnvelope,
    pub repository_id: String,
    pub workspace_id: String,
    pub base_commit: String,
    pub changes: Vec<AgentWorkspaceChange>,
    pub zones: Vec<AgentWorkspaceZone>,
    pub requested_claim_ttl_ms: u64,
    pub retention_ms: u64,
    #[serde(default)]
    pub protected_roots: Vec<AgentWorkspaceRetentionRoot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentSourceChangeTransactionStatus {
    Completed,
    Blocked,
    RolledBack,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AgentSourceChangeTransactionResult {
    pub schema: String,
    pub workload_id: String,
    pub status: AgentSourceChangeTransactionStatus,
    pub reconciliation: Option<AgentWorkspaceReconciliation>,
    pub apply_report: Option<AgentWorkspaceApplyReport>,
    pub proof_ref: Option<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

impl AgentSourceChangeTransactionResult {
    pub(super) fn blocked(workload_id: String, blockers: Vec<String>) -> Self {
        Self {
            schema: AGENT_SOURCE_CHANGE_TRANSACTION_RESULT_SCHEMA.to_string(),
            workload_id,
            status: AgentSourceChangeTransactionStatus::Blocked,
            reconciliation: None,
            apply_report: None,
            proof_ref: None,
            blockers,
            unknowns: Vec::new(),
        }
    }
}
