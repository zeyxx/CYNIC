mod admission;
mod capability_status;
mod delivery;
mod durability;
mod lifecycle;
mod shared;
mod types;
mod workspace;
mod workspace_checks;
mod workspace_claim;
mod workspace_lifecycle;
mod workspace_types;

pub use admission::{
    admit_agent_workload, admit_agent_workload_dispatch, admit_workload_mandate_binding,
};
pub use capability_status::{
    AGENT_CONSTITUTION_STATUS_SCHEMA, AGENT_EXECUTION_REQUIRED_RUNTIME_CAPABILITIES,
    AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA, AgentConstitutionRuntimeStatus,
    AgentConstitutionStatus, AgentExecutionRuntimeObservation, LegacyAgentTaskStatus,
    reconcile_agent_execution_capabilities,
};
pub use delivery::{
    AGENT_WORKLOAD_COMPLETION_PUBLICATION_SCHEMA, AGENT_WORKLOAD_DELIVERY_LEASE_MAX_TTL_MS,
    AGENT_WORKLOAD_DELIVERY_LEASE_SCHEMA, AGENT_WORKLOAD_DELIVERY_SCHEMA,
    AgentWorkloadCompletionPublication, AgentWorkloadDelivery, AgentWorkloadDeliveryAcceptance,
    AgentWorkloadDeliveryLease, AgentWorkloadDeliveryStatus, KernelDeliveryEnvelope,
    accept_agent_workload_delivery, agent_workload_delivery_available,
    complete_agent_workload_delivery, parse_agent_workload_observation_read_back,
    validate_agent_workload_completion_publication, validate_agent_workload_delivery,
    validate_agent_workload_delivery_lease, validate_agent_workload_submission_read_back,
};
pub use durability::{
    AGENT_WORKLOAD_TRANSITION_REQUEST_SCHEMA, AgentWorkloadTransition,
    AgentWorkloadTransitionRequest, DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA,
    DurableAgentWorkloadRecord, admit_aggregate_sibling_budget, validate_agent_workload_record,
    validate_durable_agent_workload_record,
};
pub use lifecycle::{
    acknowledge_agent_workload_cancellation, admit_workload_action, claim_agent_workload,
    complete_agent_workload, consume_agent_workload_action,
    finalize_unclaimed_agent_workload_cancellation, renew_agent_workload_claim,
    request_agent_workload_cancellation,
};
pub use types::{
    AGENT_WORKLOAD_ADMISSION_SCHEMA, AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_INPUT_SCHEMA,
    AGENT_WORKLOAD_RECORD_SCHEMA, AgentBoundedAction, AgentWorkloadAdmission, AgentWorkloadClaim,
    AgentWorkloadDecision, AgentWorkloadEnvelope, AgentWorkloadInput, AgentWorkloadRecord,
    AgentWorkloadState, AgentWorkloadViolation, WorkloadMandateBinding,
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
