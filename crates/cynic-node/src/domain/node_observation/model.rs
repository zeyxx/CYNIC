use serde::Serialize;

pub const NODE_OBSERVATION_SCHEMA: &str = "cynic.node_observation.v1";
pub const SURFACE_NETWORK_MESH_STATUS: &str = "network.mesh.status";
pub const SURFACE_NETWORK_MESH_DNS_ACTIVATION_STATUS: &str =
    "network.mesh.dns.activation_status";
pub const SURFACE_NETWORK_MESH_PROVIDER_FALLBACK_HOSTS: &str =
    "network.mesh.provider_fallback_hosts";
pub const SURFACE_NETWORK_MESH_PROVIDER_FALLBACK_STATUS: &str =
    "network.mesh.provider_fallback_status";
pub const SURFACE_FLEET_NODE_STATUS: &str = "fleet.node.status";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRuntimeStatus {
    Blocked,
    ProviderFallbackActive,
    MeshActive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRuntimeSeverity {
    Clear,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRuntimeBlocker {
    ProviderFallbackRouteMissing,
    CriticalDnsResolutionFailed,
    CanonicalDnsProjectionDrift,
    CriticalTcpConnectivityFailed,
    SynapseRouteMissing,
    WireguardMeshAbsent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRuntimeAction {
    RestoreProviderFallbackRoute,
    RestoreCriticalDns,
    RestoreCanonicalDnsProjection,
    RestoreCriticalTcp,
    ActivateSynapseRoute,
    ActivateWireguardMesh,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeRuntimeRemediation {
    pub action: NodeRuntimeAction,
    pub mutates_host: bool,
    pub requires_root: bool,
    pub canonical_surface: &'static str,
    pub fallback_surface: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeRuntimeCapabilities {
    pub provider_fallback_active: bool,
    pub synapse_route_active: bool,
    pub wireguard_active: bool,
    pub critical_dns_ok: bool,
    pub critical_dns_canonical: bool,
    pub critical_tcp_open: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeRuntimeVerdict {
    pub status: NodeRuntimeStatus,
    pub severity: NodeRuntimeSeverity,
    pub capabilities: NodeRuntimeCapabilities,
    pub blockers: Vec<NodeRuntimeBlocker>,
    pub remediation: Vec<NodeRuntimeRemediation>,
}
