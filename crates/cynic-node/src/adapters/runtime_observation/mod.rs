use std::error::Error;
use std::process::Command;

use crate::ports::network_interfaces_port::{
    CanonicalMeshStatus, MeshRuntimeObservation, MeshRuntimeObservationPort, NetworkPolicySummary,
    RegistryNodeRecord,
};

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

    fn command_output(command: &str, args: &[&str]) -> Option<String> {
        let output = Command::new(command).args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

impl MeshRuntimeObservationPort for LocalMeshRuntimeObserver {
    fn observe_runtime(&self) -> Result<MeshRuntimeObservation, Box<dyn Error>> {
        let interface_exists = Self::command_output("ip", &["-j", "addr", "show", "dev", &self.interface])
            .is_some();
        let wg_quick_active = Self::command_output(
            "systemctl",
            &["is-active", &format!("wg-quick@{}", self.interface)],
        )
        .is_some_and(|value| value == "active");
        let wg_show_has_output =
            Self::command_output("wg", &["show", &self.interface, "dump"]).is_some_and(|value| !value.is_empty());
        let route_show = Self::command_output("ip", &["route", "show", &self.synapse_cidr]).unwrap_or_default();
        let route_get =
            Self::command_output("ip", &["route", "get", &self.synapse_probe_ip]).unwrap_or_default();
        let route_uses_interface = route_show.contains(&format!(" dev {}", self.interface))
            || route_get.contains(&format!(" dev {}", self.interface));

        let mut blockers = Vec::new();
        if !interface_exists {
            blockers.push("wireguard_interface".to_string());
        }
        if !wg_quick_active {
            blockers.push("wg_quick_systemd_active".to_string());
        }
        if !wg_show_has_output {
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

        Ok(MeshRuntimeObservation {
            schema: "cynic.network_mesh.runtime_observation.v1",
            status,
            blockers,
            interface_present: interface_exists,
            wg_quick_active,
            route_uses_interface,
            wg_show_has_output,
        })
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
