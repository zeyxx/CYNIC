use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct MeshRegistry {
    #[serde(default)]
    pub(crate) network_policy: Option<NetworkPolicy>,
    #[serde(default)]
    pub(crate) nodes: Vec<RegistryNode>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NetworkPolicy {
    #[serde(default)]
    pub(crate) canonical_service_plane: Option<String>,
    #[serde(default)]
    pub(crate) planes: BTreeMap<String, NetworkPlane>,
    #[serde(default)]
    pub(crate) reservations: Vec<NetworkReservation>,
    #[serde(default)]
    pub(crate) mesh: Option<MeshPolicy>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MeshPolicy {
    #[serde(default)]
    pub(crate) provider: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NetworkPlane {
    #[serde(default)]
    pub(crate) cidr: Option<String>,
    #[serde(default)]
    pub(crate) dns_zone: Option<String>,
    #[serde(default)]
    pub(crate) canonical: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NetworkReservation {
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) dns: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegistryNode {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) role: Option<String>,
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) network: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) services: Vec<RegistryService>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegistryService {
    pub(crate) name: Option<String>,
    pub(crate) port: Option<u16>,
}
