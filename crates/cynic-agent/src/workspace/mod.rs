pub mod local_observer;
pub mod ports;
pub mod workspace;
pub mod workspace_checks;
pub mod workspace_claim;
pub mod workspace_lifecycle;
pub mod workspace_types;

pub use local_observer::LocalAgentWorkspaceObserver;
pub use ports::*;
pub use workspace::reconcile_agent_workspace;
pub use workspace_checks::{agent_workspace_target, zones_overlap};
pub use workspace_claim::{admit_agent_workspace_claim, workspace_claim_admission_matches};
pub use workspace_lifecycle::{
    reconcile_agent_workspace_lifecycle, validate_agent_workspace_lifecycle_record,
    AgentWorkspaceLifecycleDecision, AgentWorkspaceLifecycleObservation,
    AgentWorkspaceLifecyclePlan, AgentWorkspaceLifecycleRecord, AgentWorkspaceRetentionRoot,
    AGENT_WORKSPACE_LIFECYCLE_OBSERVATION_SCHEMA, AGENT_WORKSPACE_LIFECYCLE_PLAN_SCHEMA,
    AGENT_WORKSPACE_LIFECYCLE_RECORD_SCHEMA,
};
pub use workspace_types::{
    AgentWorkspaceApplyReport, AgentWorkspaceApplyStatus, AgentWorkspaceChange,
    AgentWorkspaceClaimAdmission, AgentWorkspaceClaimDecision, AgentWorkspaceClaimIntent,
    AgentWorkspaceClaimObservation, AgentWorkspaceClaimOutcome, AgentWorkspaceDecision,
    AgentWorkspaceMutationKind, AgentWorkspaceObservation, AgentWorkspacePathObservation,
    AgentWorkspaceReadBack, AgentWorkspaceReconciliation, AgentWorkspaceRequest,
    AgentWorkspaceViolation, AgentWorkspaceZone, AgentWorkspaceZoneKind,
    AGENT_WORKSPACE_APPLY_REPORT_SCHEMA, AGENT_WORKSPACE_CAPABILITY_ID,
    AGENT_WORKSPACE_CLAIM_ADMISSION_SCHEMA, AGENT_WORKSPACE_CLAIM_INTENT_SCHEMA,
    AGENT_WORKSPACE_OBSERVATION_SCHEMA, AGENT_WORKSPACE_RECONCILIATION_SCHEMA,
    AGENT_WORKSPACE_REQUEST_SCHEMA,
};

#[cfg(test)]
mod workspace_tests;
