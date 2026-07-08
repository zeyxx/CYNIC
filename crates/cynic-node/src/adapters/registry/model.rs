use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub(super) struct Registry {
    pub(super) network_policy: Option<RegistryNetworkPolicy>,
    pub(super) nodes: Option<Vec<RegistryNode>>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RegistryNetworkPolicy {
    pub(super) canonical_service_plane: Option<String>,
    pub(super) planes: Option<RegistryPlanes>,
    pub(super) reservations: Option<Vec<RegistryReservation>>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RegistryPlanes {
    pub(super) synapse: Option<RegistryPlane>,
    pub(super) proxmox_guest: Option<RegistryPlane>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RegistryPlane {
    pub(super) cidr: Option<String>,
    pub(super) dns_zone: Option<String>,
    pub(super) canonical: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RegistryReservation {
    pub(super) name: Option<String>,
    pub(super) dns: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RegistryNode {
    pub(super) name: Option<String>,
    pub(super) role: Option<String>,
    pub(super) status: Option<String>,
    pub(super) aliases: Option<Vec<String>>,
    pub(super) network: Option<RegistryNodeNetwork>,
    pub(super) services: Option<Vec<RegistryService>>,
    pub(super) vms: Option<Vec<RegistryVm>>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RegistryVm {
    pub(super) name: Option<String>,
    pub(super) aliases: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct RegistryNodeNetwork {
    pub(super) synapse: Option<String>,
    pub(super) local: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryService {
    pub(super) name: Option<String>,
    pub(super) port: Option<u16>,
}
