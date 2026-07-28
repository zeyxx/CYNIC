use serde::{Deserialize, Serialize};

use crate::{AgentDelegationEnvelope, ExecutionAuthorization, LeaseGuard, PolicyAdmissionReport};

pub const AGENT_WORKLOAD_ENVELOPE_SCHEMA: &str = "cynic.agent_workload_envelope.v1";
pub const AGENT_WORKLOAD_ADMISSION_SCHEMA: &str = "cynic.agent_workload_admission.v1";
pub const AGENT_WORKLOAD_RECORD_SCHEMA: &str = "cynic.agent_workload_record.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBoundedAction {
    pub argv: Vec<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub input_digest: Option<String>,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub stop_condition: String,
    pub rollback_intent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadEnvelope {
    pub schema: String,
    pub workload_id: String,
    pub idempotency_key: String,
    pub plan_digest: String,
    pub action: AgentBoundedAction,
    pub delegation: AgentDelegationEnvelope,
    #[serde(default)]
    pub parent_delegation: Option<AgentDelegationEnvelope>,
    pub policy_admission: PolicyAdmissionReport,
    pub execution_authorization: ExecutionAuthorization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentWorkloadState {
    Dispatched,
    Claimed,
    CancelRequested,
    Completed,
    Blocked,
    Failed,
}

impl AgentWorkloadState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Blocked | Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadClaim {
    pub guard: LeaseGuard,
    pub expires_at_ms: u64,
    pub cancellation_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadRecord {
    pub schema: String,
    pub envelope: AgentWorkloadEnvelope,
    pub state: AgentWorkloadState,
    #[serde(default)]
    pub claim: Option<AgentWorkloadClaim>,
    pub last_fencing_token: u64,
    pub cancellation_generation: u64,
    pub consumed_actions: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadViolation {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AgentWorkloadDecision {
    Allow,
    Deny {
        violations: Vec<AgentWorkloadViolation>,
    },
    Unknown {
        missing_evidence: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkloadAdmission {
    pub schema: String,
    pub workload_id: String,
    pub decision: AgentWorkloadDecision,
}

impl AgentWorkloadAdmission {
    pub fn admitted(&self) -> bool {
        self.schema == AGENT_WORKLOAD_ADMISSION_SCHEMA
            && matches!(self.decision, AgentWorkloadDecision::Allow)
    }
}
