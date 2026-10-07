use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MeshRegistry {
    #[serde(default)]
    pub(crate) network_policy: Option<NetworkPolicy>,
    #[serde(default)]
    pub nodes: Vec<RegistryNode>,
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct MeshPolicy {
    #[serde(default)]
    pub(crate) provider: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct NetworkPlane {
    #[serde(default)]
    pub(crate) cidr: Option<String>,
    #[serde(default)]
    pub(crate) dns_zone: Option<String>,
    #[serde(default)]
    pub(crate) canonical: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct NetworkReservation {
    pub name: Option<String>,
    #[serde(default)]
    pub(crate) dns: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegistryNode {
    pub id: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub network: BTreeMap<String, String>,
    #[serde(default)]
    pub services: Vec<RegistryService>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegistryService {
    pub name: Option<String>,
    pub port: Option<u16>,
}
