use std::collections::BTreeMap;

<<<<<<< HEAD
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
=======
use serde::Deserialize;

#[derive(Debug, Deserialize)]
>>>>>>> github/main
pub(crate) struct MeshRegistry {
    #[serde(default)]
    pub(crate) network_policy: Option<NetworkPolicy>,
    #[serde(default)]
    pub(crate) nodes: Vec<RegistryNode>,
}

<<<<<<< HEAD
#[derive(Debug, Deserialize, Serialize)]
=======
#[derive(Debug, Deserialize)]
>>>>>>> github/main
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

<<<<<<< HEAD
#[derive(Debug, Deserialize, Serialize)]
=======
#[derive(Debug, Deserialize)]
>>>>>>> github/main
pub(crate) struct MeshPolicy {
    #[serde(default)]
    pub(crate) provider: Option<String>,
}

<<<<<<< HEAD
#[derive(Debug, Deserialize, Serialize)]
=======
#[derive(Debug, Deserialize)]
>>>>>>> github/main
pub(crate) struct NetworkPlane {
    #[serde(default)]
    pub(crate) cidr: Option<String>,
    #[serde(default)]
    pub(crate) dns_zone: Option<String>,
    #[serde(default)]
    pub(crate) canonical: Option<bool>,
}

<<<<<<< HEAD
#[derive(Debug, Deserialize, Serialize)]
=======
#[derive(Debug, Deserialize)]
>>>>>>> github/main
pub(crate) struct NetworkReservation {
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) dns: Vec<String>,
}

<<<<<<< HEAD
#[derive(Debug, Deserialize, Serialize)]
=======
#[derive(Debug, Deserialize)]
>>>>>>> github/main
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

<<<<<<< HEAD
#[derive(Debug, Deserialize, Serialize)]
=======
#[derive(Debug, Deserialize)]
>>>>>>> github/main
pub(crate) struct RegistryService {
    pub(crate) name: Option<String>,
    pub(crate) port: Option<u16>,
}
