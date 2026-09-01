//! Agent host VM placement, preflight observation, and conformance planning.

pub mod decision;
pub mod observe;
pub mod types;

pub use decision::plan_agent_host_vm;
pub use observe::{
    AgentHostObservationPort, ProxmoxRestAgentHostObserver, ProxmoxSshAgentHostObserver,
};
pub use types::{
    AGENT_HOST_PREFLIGHT_REPORT_SCHEMA, AGENT_HOST_VM_PLAN_SCHEMA, AgentHostPreflightReport,
    AgentHostVmAction, AgentHostVmPlan,
};
