use cynic_host::process_observation::command_stdout;
use cynic_host::runtime_layout::current_runtime_layout;
use std::error::Error;

use crate::ports::network_interfaces_port::{
    CanonicalMeshStatus, MeshRuntimeObservation, MeshRuntimeObservationPort, NetworkPolicySummary,
    RegistryNodeRecord,
};

#[derive(Debug, Clone)]
struct MeshRuntimeCommandOutputs {
    interface_exists: bool,
    wg_quick_active: bool,
    wg_show_has_output: bool,
    route_show: String,
    route_get: String,
}

#[derive(Debug, Clone)]
pub struct LocalMeshRuntimeObserver {
    interface: String,
    synapse_cidr: String,
    synapse_probe_ip: String,
}

impl LocalMeshRuntimeObserver {
    pub fn new(interface: &str, synapse_cidr: &str, synapse_probe_ip: &str) -> Self {
        Self {
            interface: interface.to_string(),
            synapse_cidr: synapse_cidr.to_string(),
            synapse_probe_ip: synapse_probe_ip.to_string(),
        }
    }

    fn runtime_observation_from_outputs(
        interface: &str,
        outputs: MeshRuntimeCommandOutputs,
    ) -> MeshRuntimeObservation {
        let route_uses_interface = outputs.route_show.contains(&format!(" dev {}", interface))
            || outputs.route_get.contains(&format!(" dev {}", interface));

        let mut blockers = Vec::new();
        if !outputs.interface_exists {
            blockers.push("wireguard_interface".to_string());
        }
        if !outputs.wg_quick_active {
            blockers.push("wg_quick_systemd_active".to_string());
        }
        if !outputs.wg_show_has_output {
            blockers.push("wg_runtime_state".to_string());
        }
        if !route_uses_interface {
            blockers.push("synapse_route_via_wireguard_interface".to_string());
        }

        let status = if blockers.is_empty() {
            "mesh_active".to_string()
        } else {
            "blocked".to_string()
        };

        MeshRuntimeObservation {
            schema: "cynic.network_mesh.runtime_observation.v1",
            status,
            blockers,
            interface_present: outputs.interface_exists,
            wg_quick_active: outputs.wg_quick_active,
            route_uses_interface,
            wg_show_has_output: outputs.wg_show_has_output,
        }
    }
}

impl MeshRuntimeObservationPort for LocalMeshRuntimeObserver {
    fn observe_runtime(&self) -> Result<MeshRuntimeObservation, Box<dyn Error>> {
        let layout = current_runtime_layout();
        let capabilities = layout.capabilities();
        let ip_bin = capabilities.route_tool.command;
        let systemctl_bin = capabilities.service_manager.command;
        let wg_bin = capabilities.wireguard_tool.command;
        Ok(Self::runtime_observation_from_outputs(
            &self.interface,
            MeshRuntimeCommandOutputs {
                interface_exists: command_stdout(
                    &ip_bin,
                    &["-j", "addr", "show", "dev", &self.interface],
                )
                .is_some(),
                wg_quick_active: command_stdout(
                    &systemctl_bin,
                    &["is-active", &format!("wg-quick@{}", self.interface)],
                )
                .is_some_and(|value| value == "active"),
                wg_show_has_output: command_stdout(&wg_bin, &["show", &self.interface, "dump"])
                    .is_some_and(|value| !value.is_empty()),
                route_show: command_stdout(&ip_bin, &["route", "show", &self.synapse_cidr])
                    .unwrap_or_default(),
                route_get: command_stdout(&ip_bin, &["route", "get", &self.synapse_probe_ip])
                    .unwrap_or_default(),
            },
        ))
    }

    fn project_mesh_status(
        &self,
        topology: &[RegistryNodeRecord],
        _policy: &NetworkPolicySummary,
        runtime: &MeshRuntimeObservation,
    ) -> Result<CanonicalMeshStatus, Box<dyn Error>> {
        Ok(CanonicalMeshStatus {
            schema: "cynic.infrastructure_mcp.network_mesh_status.v1",
            runtime_status: runtime.status.clone(),
            dns_status: "not_checked".to_string(),
            peers: topology.len(),
            verdict: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalMeshRuntimeObserver, MeshRuntimeCommandOutputs};

    #[test]
    fn mesh_runtime_builder_reports_clear_active_state() {
        let observation = LocalMeshRuntimeObserver::runtime_observation_from_outputs(
            "wg0",
            MeshRuntimeCommandOutputs {
                interface_exists: true,
                wg_quick_active: true,
                wg_show_has_output: true,
                route_show: "10.20.0.0/24 dev wg0 scope link".to_string(),
                route_get: "10.20.0.1 dev wg0 src 10.20.0.2".to_string(),
            },
        );

        assert_eq!(observation.status, "mesh_active");
        assert!(observation.blockers.is_empty());
        assert!(observation.route_uses_interface);
    }

    #[test]
    fn mesh_runtime_builder_reports_blockers_when_mesh_is_absent() {
        let observation = LocalMeshRuntimeObserver::runtime_observation_from_outputs(
            "wg0",
            MeshRuntimeCommandOutputs {
                interface_exists: false,
                wg_quick_active: false,
                wg_show_has_output: false,
                route_show: String::new(),
                route_get: String::new(),
            },
        );

        assert_eq!(observation.status, "blocked");
        assert_eq!(observation.blockers.len(), 4);
        assert!(!observation.route_uses_interface);
    }
}
