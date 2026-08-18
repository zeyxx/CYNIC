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
    #[serde(default)]
    pub(crate) wireguard: Option<WireGuardPolicy>,
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
    pub(crate) network_membership: Option<NetworkMembership>,
    #[serde(default)]
    pub(crate) services: Vec<RegistryService>,
}

/// Membership is an explicit topology authority.  Possession of an address on
/// a service plane must not silently enlist a management host into the mesh.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NetworkMembership {
    ManagementOnly,
    SynapseMember,
    MeshDnsGateway,
}

impl NetworkMembership {
    pub(crate) fn participates_in_mesh(&self) -> bool {
        matches!(self, Self::SynapseMember | Self::MeshDnsGateway)
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegistryService {
    pub(crate) name: Option<String>,
    pub(crate) port: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct WireGuardPolicy {
    pub(crate) schema: Option<String>,
    pub(crate) overlay_cidr: Option<String>,
    pub(crate) listen_port: Option<u16>,
    pub(crate) topology: Option<String>,
    #[serde(default)]
    pub(crate) keepalive_s: Option<u32>,
    #[serde(default)]
    pub(crate) watchdog_stale_s: Option<u64>,
    #[serde(default)]
    pub(crate) watchdog_dead_s: Option<u64>,
    #[serde(default)]
    pub(crate) nodes: Vec<WireGuardNode>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub(crate) struct WireGuardNode {
    pub(crate) node_id: String,
    pub(crate) overlay_ip: String,
    pub(crate) public_key: String,
    pub(crate) endpoint: String,
    #[serde(default)]
    pub(crate) watchdog: bool,
}

impl MeshRegistry {
    pub(crate) fn wireguard_nodes(&self) -> &[WireGuardNode] {
        self.network_policy
            .as_ref()
            .and_then(|p| p.wireguard.as_ref())
            .map(|wg| wg.nodes.as_slice())
            .unwrap_or(&[])
    }

    pub(crate) fn wireguard_policy(&self) -> Option<&WireGuardPolicy> {
        self.network_policy.as_ref()?.wireguard.as_ref()
    }
}
