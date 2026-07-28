mod admission;
mod lifecycle;
mod shared;
mod types;
mod workspace;
mod workspace_checks;
mod workspace_claim;
mod workspace_lifecycle;
mod workspace_types;

pub use admission::{admit_agent_workload, admit_agent_workload_dispatch};
pub use lifecycle::{
    admit_workload_action, claim_agent_workload, complete_agent_workload,
    consume_agent_workload_action, request_agent_workload_cancellation,
};
pub use types::{
    AGENT_WORKLOAD_ADMISSION_SCHEMA, AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA,
    AgentBoundedAction, AgentWorkloadAdmission, AgentWorkloadClaim, AgentWorkloadDecision,
    AgentWorkloadEnvelope, AgentWorkloadRecord, AgentWorkloadState, AgentWorkloadViolation,
};
pub use workspace::reconcile_agent_workspace;
pub use workspace_checks::{agent_workspace_target, zones_overlap};
pub use workspace_claim::{admit_agent_workspace_claim, workspace_claim_admission_matches};
pub use workspace_lifecycle::{
    AGENT_WORKSPACE_LIFECYCLE_OBSERVATION_SCHEMA, AGENT_WORKSPACE_LIFECYCLE_PLAN_SCHEMA,
    AGENT_WORKSPACE_LIFECYCLE_RECORD_SCHEMA, AgentWorkspaceLifecycleDecision,
    AgentWorkspaceLifecycleObservation, AgentWorkspaceLifecyclePlan, AgentWorkspaceLifecycleRecord,
    AgentWorkspaceRetentionRoot, reconcile_agent_workspace_lifecycle,
    validate_agent_workspace_lifecycle_record,
};
pub use workspace_types::{
    AGENT_WORKSPACE_APPLY_REPORT_SCHEMA, AGENT_WORKSPACE_CAPABILITY_ID,
    AGENT_WORKSPACE_CLAIM_ADMISSION_SCHEMA, AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA,
    AGENT_WORKSPACE_OBSERVATION_SCHEMA, AGENT_WORKSPACE_RECONCILIATION_SCHEMA,
    AGENT_WORKSPACE_REQUEST_SCHEMA, AgentWorkspaceApplyReport, AgentWorkspaceApplyStatus,
    AgentWorkspaceChange, AgentWorkspaceClaimAdmission, AgentWorkspaceClaimDecision,
    AgentWorkspaceClaimIntent, AgentWorkspaceClaimObservation, AgentWorkspaceClaimOutcome,
    AgentWorkspaceDecision, AgentWorkspaceMutationKind, AgentWorkspaceObservation,
    AgentWorkspacePathObservation, AgentWorkspaceReadBack, AgentWorkspaceReconciliation,
    AgentWorkspaceRequest, AgentWorkspaceViolation, AgentWorkspaceZone, AgentWorkspaceZoneKind,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod workspace_tests;
