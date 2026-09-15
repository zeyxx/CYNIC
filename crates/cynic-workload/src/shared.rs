use super::types::{
    AGENT_WORKLOAD_ADMISSION_SCHEMA, AgentWorkloadAdmission, AgentWorkloadDecision,
    AgentWorkloadViolation,
};

pub(super) fn admission(
    workload_id: String,
    violations: Vec<AgentWorkloadViolation>,
    missing_evidence: Vec<String>,
) -> AgentWorkloadAdmission {
    let decision = if !violations.is_empty() {
        AgentWorkloadDecision::Deny { violations }
    } else if !missing_evidence.is_empty() {
        AgentWorkloadDecision::Unknown { missing_evidence }
    } else {
        AgentWorkloadDecision::Allow
    };
    AgentWorkloadAdmission {
        schema: AGENT_WORKLOAD_ADMISSION_SCHEMA.to_string(),
        workload_id,
        decision,
    }
}

pub(super) fn violation(
    code: impl Into<String>,
    message: impl Into<String>,
) -> AgentWorkloadViolation {
    AgentWorkloadViolation {
        code: code.into(),
        message: message.into(),
    }
}
