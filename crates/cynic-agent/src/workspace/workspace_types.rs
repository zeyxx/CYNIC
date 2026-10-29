use serde::{Deserialize, Serialize};

use cynic_protocol::{AgentEvidenceRef, LeaseGuard};

pub const AGENT_WORKSPACE_REQUEST_SCHEMA: &str = "cynic.agent_workspace_request.v1";
pub const AGENT_WORKSPACE_OBSERVATION_SCHEMA: &str = "cynic.agent_workspace_observation.v1";
pub const AGENT_WORKSPACE_RECONCILIATION_SCHEMA: &str = "cynic.agent_workspace_reconciliation.v1";
pub const AGENT_WORKSPACE_CAPABILITY_ID: &str = "source_change";
pub const AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA: &str = "cynic.agent_workspace_claim_intent.v1";
pub const AGENT_WORKSPACE_CLAIM_ADMISSION_SCHEMA: &str = "cynic.agent_workspace_claim_admission.v1";
pub const AGENT_WORKSPACE_APPLY_REPORT_SCHEMA: &str = "cynic.agent_workspace_apply_report.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentWorkspaceZoneKind {
    Exact,
    Tree,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AgentWorkspaceZone {
    pub kind: AgentWorkspaceZoneKind,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceChange {
    pub path: String,
    #[serde(default)]
    pub preimage_digest: Option<String>,
    #[serde(default)]
    pub result_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceRequest {
    pub schema: String,
    pub workload_id: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub node_id: String,
    pub agent_id: String,
    pub base_commit: String,
    pub change_set_digest: String,
    pub changes: Vec<AgentWorkspaceChange>,
    pub zones: Vec<AgentWorkspaceZone>,
    pub guard: LeaseGuard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceClaimIntent {
    pub schema: String,
    pub workload_id: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub node_id: String,
    pub agent_id: String,
    pub zones: Vec<AgentWorkspaceZone>,
    pub requested_ttl_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceClaimObservation {
    pub workload_id: String,
    pub workspace_id: String,
    pub node_id: String,
    pub agent_id: String,
    pub zones: Vec<AgentWorkspaceZone>,
    pub guard: LeaseGuard,
    pub expires_at_ms: u64,
    pub hard_expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspacePathObservation {
    pub path: String,
    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentWorkspaceMutationKind {
    Create,
    Replace,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceReadBack {
    pub path: String,
    pub mutation: AgentWorkspaceMutationKind,
    pub expected_preimage_digest: Option<String>,
    pub observed_preimage_digest: Option<String>,
    pub expected_result_digest: Option<String>,
    pub observed_result_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentWorkspaceApplyStatus {
    Applied,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceApplyReport {
    pub schema: String,
    pub workload_id: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub change_set_digest: String,
    pub fencing_token: u64,
    pub status: AgentWorkspaceApplyStatus,
    pub read_back: Option<AgentWorkspaceReadBack>,
    #[serde(default)]
    pub violations: Vec<AgentWorkspaceViolation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceObservation {
    pub schema: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub node_id: String,
    pub exists: bool,
    pub isolated: bool,
    pub head_commit: Option<String>,
    #[serde(default)]
    pub path_preimages: Vec<AgentWorkspacePathObservation>,
    #[serde(default)]
    pub repository_claims: Vec<AgentWorkspaceClaimObservation>,
    #[serde(default)]
    pub evidence_refs: Vec<AgentEvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceViolation {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AgentWorkspaceClaimDecision {
    Allow,
    Deny {
        violations: Vec<AgentWorkspaceViolation>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceClaimAdmission {
    pub schema: String,
    pub workload_id: String,
    pub intent_digest: String,
    pub max_lifetime_ms: u64,
    pub decision: AgentWorkspaceClaimDecision,
}

impl AgentWorkspaceClaimAdmission {
    pub fn admitted(&self) -> bool {
        self.schema == AGENT_WORKSPACE_CLAIM_ADMISSION_SCHEMA
            && matches!(self.decision, AgentWorkspaceClaimDecision::Allow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AgentWorkspaceClaimOutcome {
    Granted {
        claim: AgentWorkspaceClaimObservation,
    },
    Conflict {
        conflicts: Vec<AgentWorkspaceClaimObservation>,
    },
    Rejected {
        violations: Vec<AgentWorkspaceViolation>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AgentWorkspaceDecision {
    Ready {
        workspace_target: String,
        change_set_digest: String,
        zones: Vec<AgentWorkspaceZone>,
    },
    Deny {
        violations: Vec<AgentWorkspaceViolation>,
    },
    Unknown {
        missing_evidence: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkspaceReconciliation {
    pub schema: String,
    pub workload_id: String,
    pub decision: AgentWorkspaceDecision,
}

impl AgentWorkspaceReconciliation {
    pub fn ready(&self) -> bool {
        self.schema == AGENT_WORKSPACE_RECONCILIATION_SCHEMA
            && matches!(self.decision, AgentWorkspaceDecision::Ready { .. })
    }
}
