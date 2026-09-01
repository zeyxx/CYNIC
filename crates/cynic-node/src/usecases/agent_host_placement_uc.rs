//! Agent Host Placement and Conformance Use Case.
//!
//! Orchestrates Proxmox preflight observation, desired state resolution,
//! and pure VM reconciliation planning for cynic-agent-host.

use cynic_host::agent_host::{AgentHostObservationPort, AgentHostVmPlan, plan_agent_host_vm};
use cynic_protocol::AgentHostDesired;
use serde::{Deserialize, Serialize};

pub const AGENT_HOST_PLACEMENT_REPORT_SCHEMA: &str = "cynic.node.agent_host_placement_report.v1";

/// Structured report emitted by the agent host placement use case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentHostPlacementReport {
    pub schema: String,
    pub status: String,
    pub plan: AgentHostVmPlan,
    pub cortex_independent: bool,
    pub observed_at_unix_ms: u64,
}

/// Executes the agent host placement use case using an injected observation port.
pub fn execute_agent_host_placement(
    desired: &AgentHostDesired,
    observer: &dyn AgentHostObservationPort,
    now_unix_ms: u64,
) -> Result<AgentHostPlacementReport, String> {
    let preflight = observer.observe_agent_host_preflight(desired)?;
    let plan = plan_agent_host_vm(desired, &preflight);
    let cortex_independent = desired.cortex_autonomous;

    Ok(AgentHostPlacementReport {
        schema: AGENT_HOST_PLACEMENT_REPORT_SCHEMA.to_string(),
        status: plan.status.clone(),
        plan,
        cortex_independent,
        observed_at_unix_ms: now_unix_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_host::agent_host::AgentHostPreflightReport;

    struct MockObserver {
        preflight: AgentHostPreflightReport,
    }

    impl AgentHostObservationPort for MockObserver {
        fn observe_agent_host_preflight(
            &self,
            _desired: &AgentHostDesired,
        ) -> Result<AgentHostPreflightReport, String> {
            Ok(self.preflight.clone())
        }
    }

    #[test]
    fn placement_use_case_returns_planned_report() {
        let desired = AgentHostDesired::default();
        let preflight = AgentHostPreflightReport {
            schema: cynic_host::agent_host::AGENT_HOST_PREFLIGHT_REPORT_SCHEMA.to_string(),
            hypervisor_reachable: true,
            hypervisor_version: Some("pve-manager/8.4.0".to_string()),
            available_memory_bytes: 14 * 1024 * 1024 * 1024,
            available_disk_bytes: 750 * 1024 * 1024 * 1024,
            vmid_available: true,
            vmid_existing_status: None,
            template_present: true,
            template_volid: Some("local:iso/noble-server-cloudimg-amd64.img".to_string()),
            storage_pool_available: true,
            network_bridge_present: true,
            blockers: vec![],
        };
        let observer = MockObserver { preflight };
        let report = execute_agent_host_placement(&desired, &observer, 1788267750000).unwrap();

        assert_eq!(report.status, "planned");
        assert!(report.cortex_independent);
        assert_eq!(report.plan.actions.len(), 6);
    }
}
