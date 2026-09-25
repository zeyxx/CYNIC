use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct Registry {
    pub(super) network_policy: Option<RegistryNetworkPolicy>,
    pub(super) nodes: Option<Vec<RegistryNode>>,
    pub(super) organs: Option<Vec<RegistryOrgan>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryNetworkPolicy {
    pub(super) canonical_service_plane: Option<String>,
    pub(super) planes: Option<RegistryPlanes>,
    pub(super) reservations: Option<Vec<RegistryReservation>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryPlanes {
    pub(super) synapse: Option<RegistryPlane>,
    pub(super) proxmox_guest: Option<RegistryPlane>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryPlane {
    pub(super) cidr: Option<String>,
    pub(super) dns_zone: Option<String>,
    pub(super) canonical: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryReservation {
    pub(super) name: Option<String>,
    pub(super) synapse: Option<String>,
    pub(super) dns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryNode {
    pub(super) name: Option<String>,
    pub(super) id: Option<String>,
    pub(super) role: Option<String>,
    pub(super) status: Option<String>,
    pub(super) constellation: Option<String>,
    pub(super) aliases: Option<Vec<String>>,
    pub(super) network: Option<RegistryNodeNetwork>,
    pub(super) services: Option<Vec<RegistryService>>,
    pub(super) vms: Option<Vec<RegistryVm>>,
    pub(super) proxmox_id: Option<String>,
    pub(super) proxmox: Option<RegistryProxmox>,
    pub(super) source_workspace_layout: Option<RegistrySourceWorkspaceLayout>,
    #[serde(default)]
    pub(super) energy_profile: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(super) struct RegistrySourceWorkspaceLayout {
    pub(super) repository_store: String,
    pub(super) workspace_store: String,
    pub(super) principal: String,
    #[serde(default)]
    pub(super) dedicated_worktrees: bool,
    #[serde(default)]
    pub(super) repositories: Vec<RegistrySourceRepository>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(super) struct RegistrySourceRepository {
    pub(super) id: String,
    pub(super) relative_path: String,
    pub(super) transport_uri: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryVm {
    pub(super) name: Option<String>,
    pub(super) aliases: Option<Vec<String>>,
    pub(super) id: Option<String>,
    pub(super) ip: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryNodeNetwork {
    pub(super) synapse: Option<String>,
    pub(super) local: Option<String>,
    pub(super) management: Option<String>,
}

impl RegistryNodeNetwork {
    pub(super) fn best_ip(&self) -> Option<&str> {
        self.local
            .as_deref()
            .or(self.synapse.as_deref())
            .or(self.management.as_deref())
            .filter(|value| !value.is_empty() && *value != "dhcp")
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryProxmox {
    pub(super) id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryService {
    pub(super) name: Option<String>,
    pub(super) port: Option<u16>,
    pub(super) status: Option<String>,
    pub(super) optional: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryOrgan {
    pub(super) name: Option<String>,
}
