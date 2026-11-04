use serde::{Deserialize, Serialize};

use crate::detection::PackageManagerKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCapabilities {
    pub identity: IdentityCapability,
    pub name_service: NameServiceCapability,
    pub service_manager: ServiceManagerCapability,
    pub package_manager: PackageManagerCapability,
    pub route_tool: RouteToolCapability,
    pub wireguard_tool: WireguardToolCapability,
    pub network_manager: NetworkManagerCapability,
    pub dns_provider: DnsProviderCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityCapability {
    pub hostname_path: String,
    pub hostname_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameServiceCapability {
    pub resolver_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceManagerCapability {
    pub kind: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManagerCapability {
    pub kind: PackageManagerKind,
    pub installer: String,
    pub query: String,
    pub rpm_query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteToolCapability {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireguardToolCapability {
    pub command: String,
    pub mesh_interface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkManagerCapability {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsProviderCapability {
    pub provider: String,
    pub command: String,
    pub service_unit: String,
    pub resolved_unit: String,
}
