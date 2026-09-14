use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::PathBuf;

mod agent_workspace_layout;
mod model;
mod projections;

pub use agent_workspace_layout::RegistryAgentWorkspaceLayoutAdapter;

use model::Registry;
use projections::{
    canonical_names_for, preferred_node_host, reservation_hosts_by_name,
    reservation_synapse_by_name, vm_aliases_by_name,
};

use crate::domain::fleet_truth::{Node, ObservedResource};
use crate::ports::network_interfaces_port::{
    NetworkPolicySummary, ProviderFallbackHostRecord, ProviderFallbackRoute, ProviderServiceCheck,
    RegistryNodeRecord, RegistryTopologyPort,
};

#[derive(Debug, Clone)]
pub struct LocalRegistryAdapter {
    registry_path: PathBuf,
}

impl LocalRegistryAdapter {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            registry_path: path.into(),
        }
    }

    fn load_registry(&self) -> Result<Registry, RegistryAdapterError> {
        let raw =
            fs::read_to_string(&self.registry_path).map_err(|source| RegistryAdapterError::Io {
                path: self.registry_path.clone(),
                source,
            })?;
        serde_json::from_str(&raw).map_err(|source| RegistryAdapterError::InvalidJson {
            path: self.registry_path.clone(),
            source,
        })
    }

    pub fn repository_transport_uri(
        &self,
        node_id: &str,
        repository_id: &str,
    ) -> Result<Option<String>, RegistryAdapterError> {
        let registry = self.load_registry()?;
        let nodes = registry.nodes.unwrap_or_default();
        let target_node = nodes
            .into_iter()
            .find(|n| n.id.as_deref() == Some(node_id) || n.name.as_deref() == Some(node_id));
        if let Some(node) = target_node {
            if let Some(layout) = node.source_workspace_layout {
                for repo in layout.repositories {
                    if repo.id == repository_id {
                        return Ok(repo.transport_uri);
                    }
                }
            }
        }
        Ok(None)
    }

    pub fn declared_nodes(&self) -> Result<Vec<Node>, RegistryAdapterError> {
        let registry = self.load_registry()?;
        Ok(registry
            .nodes
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|node| self.node_from_registry(&registry, &node))
            .collect())
    }

    pub fn registry_organs(&self) -> Result<BTreeSet<String>, RegistryAdapterError> {
        let registry = self.load_registry()?;
        Ok(registry
            .organs
            .unwrap_or_default()
            .into_iter()
            .filter_map(|organ| organ.name.filter(|value| !value.is_empty()))
            .collect())
    }

    pub fn observed_provider_resources(
        &self,
    ) -> Result<Vec<ObservedResource>, RegistryAdapterError> {
        let registry = self.load_registry()?;
        let mut resources = Vec::new();
        let mut seen = BTreeSet::new();

        for node in registry.nodes.clone().unwrap_or_default() {
            let Some(resource) = self.node_resource(&registry, &node) else {
                continue;
            };
            let key = (
                resource.provider.clone(),
                resource.name.clone(),
                resource.resource_id.clone(),
            );
            if seen.insert(key) {
                resources.push(resource);
            }
        }

        for parent in registry.nodes.unwrap_or_default() {
            let constellation = parent.constellation.clone();
            for vm in parent.vms.unwrap_or_default() {
                let Some(resource) = self.vm_resource(&vm, constellation.clone()) else {
                    continue;
                };
                let key = (
                    resource.provider.clone(),
                    resource.name.clone(),
                    resource.resource_id.clone(),
                );
                if seen.insert(key) {
                    resources.push(resource);
                }
            }
        }

        Ok(resources)
    }

    fn node_from_registry(&self, registry: &Registry, node: &model::RegistryNode) -> Option<Node> {
        let name = node_name(node)?;
        Some(Node {
            name,
            role: node.role.clone().unwrap_or_else(|| "unknown".to_string()),
            status: node.status.clone().unwrap_or_else(|| "unknown".to_string()),
            constellation: node.constellation.clone(),
            aliases: node.aliases.clone().unwrap_or_default(),
            host: self.local_ip(node),
            proxmox_id: self.node_proxmox_id(registry, node),
            energy_profile: node.energy_profile.clone(),
        })
    }

    fn node_proxmox_id(&self, registry: &Registry, node: &model::RegistryNode) -> Option<String> {
        if let Some(value) = node.proxmox_id.clone().filter(|value| !value.is_empty()) {
            return Some(value);
        }
        if let Some(value) = node
            .proxmox
            .as_ref()
            .and_then(|proxmox| proxmox.id.clone())
            .filter(|value| !value.is_empty())
        {
            return Some(value);
        }
        let name = node_name(node)?;
        for parent in registry.nodes.as_deref().unwrap_or(&[]) {
            for vm in parent.vms.as_deref().unwrap_or(&[]) {
                if vm.name.as_deref() == Some(name.as_str()) {
                    if let Some(value) = vm.id.clone().filter(|value| !value.is_empty()) {
                        return Some(value);
                    }
                }
            }
        }
        None
    }

    fn node_resource(
        &self,
        registry: &Registry,
        node: &model::RegistryNode,
    ) -> Option<ObservedResource> {
        let name = node_name(node)?;
        let local_ip = self.local_ip(node)?;
        if local_ip == "dhcp" {
            return None;
        }
        let declared_status = node.status.as_deref().unwrap_or("unknown");
        let status = if matches!(
            declared_status,
            "active" | "declared" | "planned_obsolescence"
        ) {
            "running"
        } else {
            "planned"
        };
        Some(ObservedResource {
            provider: "operator_observation".to_string(),
            name,
            status: status.to_string(),
            resource_id: self.node_proxmox_id(registry, node),
            kind: self.resource_kind_for_role(node.role.as_deref()),
            constellation: node.constellation.clone(),
        })
    }

    fn vm_resource(
        &self,
        vm: &model::RegistryVm,
        constellation: Option<String>,
    ) -> Option<ObservedResource> {
        let name = vm.name.clone().filter(|value| !value.is_empty())?;
        let ip = vm.ip.clone().filter(|value| !value.is_empty())?;
        if ip == "dhcp" {
            return None;
        }
        let resource_id = vm.id.clone().filter(|value| !value.is_empty());
        let kind = resource_id.as_ref().map(|value| {
            if value.starts_with("lxc/") {
                "lxc".to_string()
            } else {
                "vm".to_string()
            }
        });
        Some(ObservedResource {
            provider: "operator_observation".to_string(),
            name,
            status: "running".to_string(),
            resource_id,
            kind,
            constellation,
        })
    }

    fn resource_kind_for_role(&self, role: Option<&str>) -> Option<String> {
        match role {
            Some("hypervisor") => Some("hypervisor".to_string()),
            Some("storage_primary" | "forge_primary") => Some("lxc".to_string()),
            Some(value) if !value.is_empty() => Some("vm".to_string()),
            _ => None,
        }
    }

    fn local_ip(&self, node: &model::RegistryNode) -> Option<String> {
        node.network
            .as_ref()
            .and_then(|network| network.best_ip())
            .map(|value| value.to_string())
    }
}

fn node_name(node: &model::RegistryNode) -> Option<String> {
    node.name
        .clone()
        .or_else(|| node.id.clone())
        .filter(|value| !value.is_empty())
}

fn registry_dns_zone(registry: &Registry) -> String {
    registry
        .network_policy
        .as_ref()
        .and_then(|policy| policy.planes.as_ref())
        .and_then(|planes| planes.synapse.as_ref())
        .and_then(|plane| plane.dns_zone.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "synapse.cynic".to_string())
}

fn provider_fallback_cidr(registry: &Registry) -> String {
    registry
        .network_policy
        .as_ref()
        .and_then(|policy| policy.planes.as_ref())
        .and_then(|planes| planes.proxmox_guest.as_ref())
        .and_then(|plane| plane.cidr.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "10.10.10.0/24".to_string())
}

fn status_allows_service_checks(status: Option<&str>) -> bool {
    !matches!(
        status,
        Some("planned") | Some("archived") | Some("retired") | Some("planned_obsolescence")
    )
}

fn ipv4_in_cidr(ip: &str, cidr: &str) -> bool {
    let Some((network, prefix)) = cidr.split_once('/') else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u32>() else {
        return false;
    };
    if prefix > 32 {
        return false;
    }
    let Some(ip) = parse_ipv4(ip) else {
        return false;
    };
    let Some(network) = parse_ipv4(network) else {
        return false;
    };
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (ip & mask) == (network & mask)
}

fn parse_ipv4(value: &str) -> Option<u32> {
    let mut output = 0u32;
    let mut parts = 0usize;
    for part in value.split('.') {
        let octet = part.parse::<u8>().ok()?;
        output = (output << 8) | u32::from(octet);
        parts += 1;
    }
    (parts == 4).then_some(output)
}

impl RegistryTopologyPort for LocalRegistryAdapter {
    fn fetch_static_topology(&self) -> Result<Vec<RegistryNodeRecord>, Box<dyn Error>> {
        let registry = self.load_registry()?;
        let dns_zone = registry_dns_zone(&registry);
        let reservation_hosts = reservation_hosts_by_name(&registry, &dns_zone);
        let reservation_synapse = reservation_synapse_by_name(&registry);
        let mut records = Vec::new();
        let vm_aliases = vm_aliases_by_name(registry.nodes.as_deref().unwrap_or(&[]));
        for node in registry.nodes.unwrap_or_default() {
            let Some(synapse_ip) = node
                .network
                .as_ref()
                .and_then(|network| network.synapse.as_deref())
            else {
                continue;
            };
            if synapse_ip.starts_with('<') {
                continue;
            }
            let Some(primary_name) = node.name.clone().filter(|value| !value.is_empty()) else {
                continue;
            };
            if reservation_synapse.get(&primary_name).map(String::as_str) != Some(synapse_ip) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Synapse reservation missing or mismatched for {primary_name}: node={synapse_ip} reservation={:?}",
                        reservation_synapse.get(&primary_name)
                    ),
                )
                .into());
            }
            records.push(RegistryNodeRecord {
                node: primary_name.clone(),
                ip: synapse_ip.to_string(),
                names: canonical_names_for(
                    &dns_zone,
                    &primary_name,
                    node.aliases.as_deref().unwrap_or(&[]),
                    reservation_hosts.get(&primary_name).cloned(),
                ),
            });
            let aliases = node
                .aliases
                .unwrap_or_default()
                .into_iter()
                .chain(vm_aliases.get(&primary_name).into_iter().flatten().cloned())
                .filter(|value| !value.is_empty())
                .collect::<BTreeSet<_>>();
            for alias in aliases {
                records.push(RegistryNodeRecord {
                    node: alias.clone(),
                    ip: synapse_ip.to_string(),
                    names: canonical_names_for(&dns_zone, &alias, &[], None),
                });
            }
        }
        Ok(records)
    }

    fn fetch_network_policy_summary(&self) -> Result<NetworkPolicySummary, Box<dyn Error>> {
        let registry = self.load_registry()?;
        let policy = registry.network_policy.unwrap_or_default();
        let provider_fallback_planes = policy
            .planes
            .as_ref()
            .map(|planes| {
                let mut values = Vec::new();
                if planes
                    .proxmox_guest
                    .as_ref()
                    .is_some_and(|plane| plane.canonical == Some(false))
                {
                    values.push("proxmox_guest".to_string());
                }
                values
            })
            .unwrap_or_default();
        Ok(NetworkPolicySummary {
            canonical_service_plane: policy
                .canonical_service_plane
                .unwrap_or_else(|| "synapse".to_string()),
            synapse_cidr: policy
                .planes
                .as_ref()
                .and_then(|planes| planes.synapse.as_ref().and_then(|plane| plane.cidr.clone())),
            synapse_dns_zone: policy.planes.as_ref().and_then(|planes| {
                planes
                    .synapse
                    .as_ref()
                    .and_then(|plane| plane.dns_zone.clone())
            }),
            provider_fallback_planes,
        })
    }

    fn fetch_provider_fallback_records(
        &self,
    ) -> Result<Vec<ProviderFallbackHostRecord>, Box<dyn Error>> {
        let registry = self.load_registry()?;
        let dns_zone = registry_dns_zone(&registry);
        let provider_fallback_cidr = provider_fallback_cidr(&registry);
        let reservation_dns = registry
            .network_policy
            .as_ref()
            .and_then(|policy| policy.reservations.as_ref())
            .map(|reservations| {
                let mut values = std::collections::BTreeMap::new();
                for reservation in reservations {
                    let Some(name) = reservation.name.clone().filter(|value| !value.is_empty())
                    else {
                        continue;
                    };
                    let dns = reservation
                        .dns
                        .clone()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|value| value.ends_with(&dns_zone))
                        .collect::<Vec<_>>();
                    values.insert(name, dns);
                }
                values
            })
            .unwrap_or_default();
        let mut records = Vec::new();
        for node in registry.nodes.unwrap_or_default() {
            let Some(name) = node.name.clone().filter(|value| !value.is_empty()) else {
                continue;
            };
            let Some(local_ip) = node
                .network
                .as_ref()
                .and_then(|network| network.best_ip())
                .map(|value| value.to_string())
            else {
                continue;
            };
            let mut names = node.aliases.clone().unwrap_or_default();
            names.push(name.clone());
            if let Some(hosts) = reservation_dns.get(&name) {
                for host in hosts {
                    if !host.is_empty() && !names.contains(host) {
                        names.push(host.clone());
                    }
                }
            }
            names.sort();
            names.dedup();
            records.push(ProviderFallbackHostRecord {
                node: name,
                role: node.role.clone(),
                provider_plane: if ipv4_in_cidr(&local_ip, &provider_fallback_cidr) {
                    "proxmox_guest".to_string()
                } else {
                    "lan".to_string()
                },
                ip: local_ip,
                names,
                canonical: false,
            });
        }
        records.sort_by(|left, right| left.node.cmp(&right.node));
        Ok(records)
    }

    fn fetch_provider_fallback_route(&self) -> Result<ProviderFallbackRoute, Box<dyn Error>> {
        let registry = self.load_registry()?;
        let gateway = registry.nodes.as_ref().and_then(|nodes| {
            nodes
                .iter()
                .find(|node| node.role.as_deref() == Some("hypervisor"))
                .and_then(|node| node.network.as_ref())
                .and_then(|network| network.best_ip())
                .map(|value| value.to_string())
        });
        let cidr = provider_fallback_cidr(&registry);
        Ok(ProviderFallbackRoute {
            cidr,
            gateway,
            provider_plane: "proxmox_guest".to_string(),
        })
    }

    fn fetch_provider_fallback_service_checks(
        &self,
    ) -> Result<Vec<ProviderServiceCheck>, Box<dyn Error>> {
        let registry = self.load_registry()?;
        let dns_zone = registry_dns_zone(&registry);
        let fallback_records = self.fetch_provider_fallback_records()?;
        let mut checks = Vec::new();
        for node in registry.nodes.unwrap_or_default() {
            let Some(name) = node.name.clone().filter(|value| !value.is_empty()) else {
                continue;
            };
            if !status_allows_service_checks(node.status.as_deref()) {
                continue;
            }
            let Some(network) = node.network.as_ref() else {
                continue;
            };
            let Some(local_ip) = network.best_ip().map(|value| value.to_string()) else {
                continue;
            };
            let fallback_host = fallback_records
                .iter()
                .find(|record| record.node == name)
                .and_then(|record| {
                    record
                        .names
                        .iter()
                        .find(|value| value.ends_with(&dns_zone))
                        .cloned()
                })
                .unwrap_or_else(|| preferred_node_host(&dns_zone, &node, None, &local_ip));
            let mut node_checks = Vec::new();
            for service in node.services.clone().unwrap_or_default() {
                if matches!(
                    service.status.as_deref(),
                    Some("planned")
                        | Some("archived")
                        | Some("retired")
                        | Some("disabled")
                        | Some("optional")
                ) || service.optional.unwrap_or(false)
                {
                    continue;
                }
                let Some(port) = service.port else {
                    continue;
                };
                node_checks.push(ProviderServiceCheck {
                    node: name.clone(),
                    service: service.name.unwrap_or_else(|| "unknown".to_string()),
                    host: fallback_host.clone(),
                    ip: local_ip.clone(),
                    port,
                });
            }
            checks.extend(node_checks);
        }
        checks.sort_by(|left, right| {
            (&left.node, &left.service, left.port).cmp(&(&right.node, &right.service, right.port))
        });
        checks.dedup_by(|left, right| {
            left.node == right.node
                && left.host == right.host
                && left.port == right.port
                && left.service == right.service
        });
        Ok(checks)
    }
}

#[derive(Debug)]
pub enum RegistryAdapterError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidJson {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl Display for RegistryAdapterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed to read registry {}: {source}", path.display())
            }
            Self::InvalidJson { path, source } => {
                write!(f, "failed to parse registry {}: {source}", path.display())
            }
        }
    }
}

impl Error for RegistryAdapterError {}

#[cfg(test)]
mod tests;
