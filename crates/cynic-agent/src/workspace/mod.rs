pub mod workspace;
pub mod workspace_checks;
pub mod workspace_claim;
pub mod workspace_lifecycle;
pub mod workspace_types;

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
mod workspace_tests;
