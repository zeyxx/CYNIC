use crate::domain::node_observation::NodeRuntimeVerdict;
use crate::domain::observation_runtime::DnsResolutionObservation;
use crate::{
    adapters::host_observation::{CommandObservation, TcpObservation},
    build_attestation::BinaryAttestation,
};
use serde::Serialize;
use std::error::Error;

/// Canonical registry-backed topology view for node-local network decisions.
pub trait RegistryTopologyPort {
    fn fetch_static_topology(&self) -> Result<Vec<RegistryNodeRecord>, Box<dyn Error>>;
    fn fetch_network_policy_summary(&self) -> Result<NetworkPolicySummary, Box<dyn Error>>;
    fn fetch_provider_fallback_records(
        &self,
    ) -> Result<Vec<ProviderFallbackHostRecord>, Box<dyn Error>>;
    fn fetch_provider_fallback_route(&self) -> Result<ProviderFallbackRoute, Box<dyn Error>>;
    fn fetch_provider_fallback_service_checks(
        &self,
    ) -> Result<Vec<ProviderServiceCheck>, Box<dyn Error>>;
}

/// Host/runtime observation port for local mesh truth.
pub trait MeshRuntimeObservationPort {
    fn observe_runtime(&self) -> Result<MeshRuntimeObservation, Box<dyn Error>>;
    fn project_mesh_status(
        &self,
        topology: &[RegistryNodeRecord],
        policy: &NetworkPolicySummary,
        runtime: &MeshRuntimeObservation,
    ) -> Result<CanonicalMeshStatus, Box<dyn Error>>;
}

/// Deterministic rendering of canonical Synapse DNS bundles.
pub trait DnsBundleRendererPort {
    fn activation_plan(&self) -> Result<DnsActivationPlan, Box<dyn Error>>;
    fn render_hosts(&self, topology: &[RegistryNodeRecord]) -> Result<String, Box<dyn Error>>;
    fn render_corefile(&self) -> Result<String, Box<dyn Error>>;
    fn render_resolved_dropin(&self) -> Result<String, Box<dyn Error>>;
}

/// Observation of provider-plane fallback health while canonical Synapse is degraded.
pub trait ProviderFallbackProbePort {
    fn observe_route(
        &self,
        route: &ProviderFallbackRoute,
    ) -> Result<ProviderFallbackRouteStatus, Box<dyn Error>>;
    fn observe_connectivity(
        &self,
        checks: &[ProviderServiceCheck],
    ) -> Result<Vec<ProviderServiceConnectivity>, Box<dyn Error>>;
}

/// Local node-runtime observation port used by node observation assembly.
pub(crate) trait NodeRuntimeObservationPort {
    fn observe_interfaces(&self) -> CommandObservation;
    fn observe_routes(&self) -> CommandObservation;
    fn observe_network_manager_connections(&self) -> CommandObservation;
    fn observe_policy_routes(&self) -> CommandObservation;
    fn observe_network_namespaces(&self) -> CommandObservation;
    fn observe_firewall(&self) -> CommandObservation;
    fn observe_provider_fallback_route(&self, cidr: &str) -> CommandObservation;
    fn observe_synapse_route(&self, cidr: &str) -> CommandObservation;
    fn observe_wireguard(&self) -> CommandObservation;
    fn observe_mesh_runtime(
        &self,
        synapse_cidr: &str,
        synapse_probe_ip: &str,
    ) -> MeshRuntimeObservation;
    fn observe_dns_checks(
        &self,
        critical_hosts: &[String],
        expected_synapse_ips: &std::collections::BTreeMap<String, String>,
    ) -> Vec<DnsResolutionObservation>;
    fn observe_tcp_checks(&self, checks: &[ProviderServiceCheck]) -> Vec<TcpObservation>;
    fn observe_wg_quick_status(&self) -> CommandObservation;
    fn observe_network_manager_status(&self) -> CommandObservation;
    fn observe_systemd_resolved_status(&self) -> CommandObservation;
    fn observe_hostname(&self) -> String;
    fn observe_binary_attestation(&self, registry_path: Option<&str>) -> BinaryAttestation;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegistryNodeRecord {
    pub node: String,
    pub ip: String,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetworkPolicySummary {
    pub canonical_service_plane: String,
    pub synapse_cidr: Option<String>,
    pub synapse_dns_zone: Option<String>,
    pub provider_fallback_planes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderFallbackRoute {
    pub cidr: String,
    pub gateway: Option<String>,
    pub provider_plane: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderFallbackHostRecord {
    pub node: String,
    pub role: Option<String>,
    pub provider_plane: String,
    pub ip: String,
    pub names: Vec<String>,
    pub canonical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderServiceCheck {
    pub node: String,
    pub service: String,
    pub host: String,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderFallbackRouteStatus {
    pub active: bool,
    pub persistent: bool,
    pub observed_route: String,
    pub observed_nmcli_routes: String,
    pub ip_route_error: Option<String>,
    pub nmcli_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderServiceConnectivity {
    pub node: String,
    pub service: String,
    pub host: String,
    pub ip: String,
    pub port: u16,
    pub status: String,
    pub network_status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DnsActivationPlan {
    pub zone: String,
    pub hosts_path: String,
    pub corefile_path: String,
    pub resolved_dropin_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeshRuntimeObservation {
    pub schema: &'static str,
    pub status: String,
    pub blockers: Vec<String>,
    pub interface_present: bool,
    pub wg_quick_active: bool,
    pub route_uses_interface: bool,
    pub wg_show_has_output: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalMeshStatus {
    pub schema: &'static str,
    pub runtime_status: String,
    pub dns_status: String,
    pub peers: usize,
    pub verdict: Option<NodeRuntimeVerdict>,
}
