//! Agent host VM placement, preflight observation, and conformance planning.

pub mod decision;
pub mod types;

pub use crate::adapters::agent_host_observer::{
    AgentHostObservationPort, ProxmoxRestAgentHostObserver, ProxmoxSshAgentHostObserver,
};
pub use decision::plan_agent_host_vm;
pub use types::{
    AGENT_HOST_PREFLIGHT_REPORT_SCHEMA, AGENT_HOST_VM_PLAN_SCHEMA, AgentHostPreflightReport,
    AgentHostVmAction, AgentHostVmPlan,
};
