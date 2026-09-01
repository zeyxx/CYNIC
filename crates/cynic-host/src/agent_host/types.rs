//! Types for the agent host VM conformance and placement plan.

use cynic_protocol::AgentHostDesired;
use serde::{Deserialize, Serialize};

pub const AGENT_HOST_VM_PLAN_SCHEMA: &str = "cynic.agent_host.vm_plan.v1";
pub const AGENT_HOST_PREFLIGHT_REPORT_SCHEMA: &str = "cynic.agent_host.preflight_report.v1";

/// Preflight observation result on the target Proxmox hypervisor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentHostPreflightReport {
    pub schema: String,
    pub hypervisor_reachable: bool,
    pub hypervisor_version: Option<String>,
    pub available_memory_bytes: u64,
    pub available_disk_bytes: u64,
    pub vmid_available: bool,
    pub vmid_existing_status: Option<String>,
    pub template_present: bool,
    pub template_volid: Option<String>,
    pub storage_pool_available: bool,
    pub network_bridge_present: bool,
    pub blockers: Vec<String>,
}

impl Default for AgentHostPreflightReport {
    fn default() -> Self {
        Self {
            schema: AGENT_HOST_PREFLIGHT_REPORT_SCHEMA.to_string(),
            hypervisor_reachable: false,
            hypervisor_version: None,
            available_memory_bytes: 0,
            available_disk_bytes: 0,
            vmid_available: false,
            vmid_existing_status: None,
            template_present: false,
            template_volid: None,
            storage_pool_available: false,
            network_bridge_present: false,
            blockers: vec!["unobserved_preflight".to_string()],
        }
    }
}

/// A discrete bounded action for provisioning or configuring the VM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentHostVmAction {
    pub step: u32,
    pub kind: String,
    pub purpose: String,
    pub argv: Vec<String>,
    pub privileged: bool,
}

/// Complete reconciliation plan for creating or validating the agent host VM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentHostVmPlan {
    pub schema: String,
    pub status: String,
    pub desired: AgentHostDesired,
    pub preflight: AgentHostPreflightReport,
    pub actions: Vec<AgentHostVmAction>,
    pub rollback_intent: String,
    pub mutates: bool,
    pub blockers: Vec<String>,
}
