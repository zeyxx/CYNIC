//! Registry types — pure data structures, no I/O, no policy

use cynic_protocol::NodeSubstrateDesiredState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Raw registry document as loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryDocument {
    #[serde(default)]
    pub network_policy: Option<RegistryNetworkPolicy>,
    #[serde(default)]
    pub lifecycle: Option<RegistryLifecycle>,
    pub nodes: Vec<RegistryNode>,
    #[serde(default)]
    pub fleet_desired: Option<RegistryFleetDesired>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryFleetDesired {
    #[serde(default)]
    pub targets: Vec<RegistryFleetTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryFleetTarget {
    pub id: Option<String>,
    pub provider: Option<String>,
    pub resource_id: Option<String>,
    pub role: Option<String>,
    pub desired_power: Option<String>,
    #[serde(default)]
    pub provisioning: Option<cynic_protocol::NodeSubstrateProvisioningSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryNode {
    pub name: Option<String>,
    pub id: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub role: Option<String>,
    pub canonical_role: Option<String>,
    #[serde(default)]
    pub provider_controls: Vec<String>,
    #[serde(default)]
    pub transports: Option<RegistryNodeTransports>,
    #[serde(default)]
    pub host_family: Option<String>,
    #[serde(default)]
    pub substrate: Option<RegistryNodeSubstrate>,
    #[serde(default)]
    pub services: Vec<RegistryNodeService>,
    #[serde(default)]
    pub network_links: Vec<RegistryNetworkLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryNodeTransports {
    #[serde(default)]
    pub ssh: Option<RegistrySshTransport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySshTransport {
    pub target: Option<String>,
    pub trust: Option<String>,
    #[serde(default)]
    pub enrollment_authority: Option<RegistrySshEnrollmentAuthority>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySshEnrollmentAuthority {
    pub class: Option<String>,
    pub principal: Option<String>,
    pub credential_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryNetworkPolicy {
    #[serde(default)]
    pub planes: BTreeMap<String, RegistryNetworkPlane>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryNetworkPlane {
    #[serde(default)]
    pub cidr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryNetworkLink {
    #[serde(default)]
    pub plane: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub interface: Option<String>,
    #[serde(default)]
    pub connection: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub gateway: Option<String>,
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub default_route: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryNodeSubstrate {
    pub family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryNodeService {
    pub name: Option<String>,
    pub artifact: Option<String>,
    pub port: Option<u16>,
    pub unit: Option<String>,
    #[serde(default)]
    pub supervisor: Option<RegistryServiceSupervisor>,
    #[serde(default)]
    pub health: Option<RegistryServiceHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryServiceSupervisor {
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryServiceHealth {
    pub kind: Option<String>,
    pub port: Option<u16>,
    pub process: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryLifecycle {
    #[serde(default)]
    pub kernel_release: Option<RegistryKernelRelease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryKernelRelease {
    #[serde(default)]
    pub build_runner_role: Option<String>,
    #[serde(default)]
    pub host_bootstrap: Option<RegistryHostBootstrap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryHostBootstrap {
    #[serde(default)]
    pub role_profiles: BTreeMap<String, RegistryHostBootstrapRoleProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryHostBootstrapRoleProfile {
    pub target_stage: String,
    #[serde(default)]
    pub required_domains: Vec<String>,
}

/// Resolved types — output of registry resolution (serialized, no I/O)

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedDirectManagementLink {
    pub interface: String,
    pub connection: String,
    pub address: String,
    pub gateway: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservedManagementLink {
    pub interface: String,
    pub connection: String,
    pub address: String,
    pub gateway: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedHostIdentity {
    pub node: String,
    pub role: Option<String>,
    pub match_source: String,
    pub hostname_match: bool,
    pub network_match: bool,
    pub metadata: Vec<String>,
    pub link: ResolvedDirectManagementLink,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedBuildRunnerTransport {
    pub node_id: String,
    pub role: String,
    pub transport_kind: String,
    pub ssh_target: String,
    pub trust: String,
    pub enrollment_authority_class: String,
    pub enrollment_principal: String,
    pub enrollment_credential_ref: String,
    pub registry_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedProviderControlTransport {
    pub provider: String,
    pub node_id: String,
    pub ssh_target: String,
    pub trust: String,
    pub registry_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolvedNodeSubstrateTarget {
    pub target_id: String,
    pub role: String,
    pub desired: NodeSubstrateDesiredState,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostProfileResolution {
    pub hostname_observed: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_role: Option<String>,
    pub profile_applied: &'static str,
    pub match_source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_bootstrap_target_stage: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub host_bootstrap_required_domains: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_path: Option<String>,
}

impl RegistryNetworkLink {
    fn to_direct_management_link(&self) -> Option<ResolvedDirectManagementLink> {
        let interface = self.interface.as_ref()?.trim();
        let connection = self.connection.as_ref()?.trim();
        let address = self.address.as_ref()?.trim();
        let gateway = self.gateway.as_ref()?.trim();
        if interface.is_empty() || connection.is_empty() || address.is_empty() || gateway.is_empty()
        {
            return None;
        }
        Some(ResolvedDirectManagementLink {
            interface: interface.to_string(),
            connection: connection.to_string(),
            address: address.to_string(),
            gateway: gateway.to_string(),
        })
    }
}

impl RegistryDocument {
    pub fn synapse_cidr(&self) -> Option<&str> {
        self.network_policy.as_ref().and_then(|policy| {
            policy
                .planes
                .get("synapse")
                .and_then(|plane| plane.cidr.as_deref())
        })
    }
}

impl RegistryNode {
    pub fn matching_direct_management_link(
        &self,
        synapse_cidr: &str,
    ) -> Option<ResolvedDirectManagementLink> {
        self.network_links.iter().find_map(|link| {
            let Some(plane) = link.plane.as_deref() else {
                return None;
            };
            if plane != "management" {
                return None;
            }
            if let Some(provider) = link.provider.as_deref() {
                if provider != "network-manager" {
                    return None;
                }
            }
            let route_match = link.routes.iter().any(|route| route.trim() == synapse_cidr);
            if !route_match {
                return None;
            }
            link.to_direct_management_link()
        })
    }
}
