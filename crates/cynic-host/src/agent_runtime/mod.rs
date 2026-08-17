//! External-agent runtime inventory and read-only capability observation.
//!
//! The catalog is desired state. Probes only establish what this host can
//! currently observe; an observed CLI is never execution authority.

mod adoption;
mod probe;
mod process_identity;
mod specs;
mod types;

pub use adoption::{
    AGENT_RUNTIME_ADOPTION_PLAN_SCHEMA, AgentRuntimeAdoptionDecision,
    AgentRuntimeAdoptionDesiredState, AgentRuntimeAdoptionPlan, AgentRuntimeBoundedAction,
    apply_runtime_ownership, plan_runtime_adoption,
};
pub use probe::observe_agent_runtimes;
pub use process_identity::{
    AgentProcessIdentityObservation, AgentRuntimeOwnership, AgentRuntimeOwnershipState,
    ProcessIdentityState, classify_runtime_ownership,
    classify_runtime_ownership_with_external_gateway, observe_linux_process_identity,
};
pub use types::{
    AgentRuntimeActivation, AgentRuntimeInstallOwnership, AgentRuntimeInstallProvenance,
    AgentRuntimeInventoryReport, AgentRuntimeMode, AgentRuntimeObservation,
    AgentRuntimeSupervision, CapabilityNode, CapabilityState,
};
