use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::PathBuf;

mod model;
mod projections;

use model::Registry;
use projections::{
    canonical_names_for, is_l0_network_role, is_l0_network_service, preferred_node_host,
    reservation_hosts_by_name, vm_aliases_by_name,
};

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
        let raw = fs::read_to_string(&self.registry_path).map_err(|source| {
            RegistryAdapterError::Io {
                path: self.registry_path.clone(),
                source,
            }
        })?;
        serde_json::from_str(&raw).map_err(|source| RegistryAdapterError::InvalidJson {
            path: self.registry_path.clone(),
            source,
        })
    }
}

impl RegistryTopologyPort for LocalRegistryAdapter {
    fn fetch_static_topology(&self) -> Result<Vec<RegistryNodeRecord>, Box<dyn Error>> {
        let registry = self.load_registry()?;
        let reservation_hosts = reservation_hosts_by_name(&registry);
        let mut records = Vec::new();
        let vm_aliases = vm_aliases_by_name(registry.nodes.as_deref().unwrap_or(&[]));
        for node in registry.nodes.unwrap_or_default() {
            let Some(synapse_ip) = node.network.as_ref().and_then(|network| network.synapse.as_deref()) else {
                continue;
            };
            if synapse_ip.starts_with('<') {
                continue;
            }
            let Some(primary_name) = node.name.clone().filter(|value| !value.is_empty()) else {
                continue;
            };
            records.push(RegistryNodeRecord {
                node: primary_name.clone(),
                ip: synapse_ip.to_string(),
                names: canonical_names_for(
                    &primary_name,
                    node.aliases.as_deref().unwrap_or(&[]),
                    reservation_hosts.get(&primary_name).cloned(),
                ),
            });
            let aliases = node
                .aliases
                .unwrap_or_default()
                .into_iter()
                .chain(
                    vm_aliases
                        .get(&primary_name)
                        .into_iter()
                        .flatten()
                        .cloned(),
                )
                .filter(|value| !value.is_empty())
                .collect::<BTreeSet<_>>();
            for alias in aliases {
                records.push(RegistryNodeRecord {
                    node: alias.clone(),
                    ip: synapse_ip.to_string(),
                    names: canonical_names_for(&alias, &[], None),
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
                if planes.proxmox_guest.as_ref().is_some_and(|plane| plane.canonical == Some(false))
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
            synapse_cidr: policy.planes.as_ref().and_then(|planes| {
                planes
                    .synapse
                    .as_ref()
                    .and_then(|plane| plane.cidr.clone())
            }),
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
        let reservation_dns = registry
            .network_policy
            .as_ref()
            .and_then(|policy| policy.reservations.as_ref())
            .map(|reservations| {
                let mut values = std::collections::BTreeMap::new();
                for reservation in reservations {
                    let Some(name) = reservation.name.clone().filter(|value| !value.is_empty()) else {
                        continue;
                    };
                    let dns = reservation
                        .dns
                        .clone()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|value| value.ends_with(".synapse.cynic"))
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
                .and_then(|network| network.local.clone())
                .filter(|value| value != "dhcp")
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
                provider_plane: if local_ip.starts_with("10.10.10.") {
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
        let gateway = registry
            .nodes
            .as_ref()
            .and_then(|nodes| {
                nodes.iter()
                    .find(|node| node.role.as_deref() == Some("hypervisor"))
                    .and_then(|node| node.network.as_ref())
                    .and_then(|network| network.local.clone())
            });
        let cidr = registry
            .network_policy
            .as_ref()
            .and_then(|policy| policy.planes.as_ref())
            .and_then(|planes| planes.proxmox_guest.as_ref())
            .and_then(|plane| plane.cidr.clone())
            .unwrap_or_else(|| "10.10.10.0/24".to_string());
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
        let fallback_records = self.fetch_provider_fallback_records()?;
        let mut checks = Vec::new();
        for node in registry.nodes.unwrap_or_default() {
            let Some(name) = node.name.clone().filter(|value| !value.is_empty()) else {
                continue;
            };
            if node.status.as_deref() != Some("active") || !is_l0_network_role(node.role.as_deref()) {
                continue;
            }
            let Some(network) = node.network.as_ref() else {
                continue;
            };
            let Some(local_ip) = network.local.clone().filter(|value| value != "dhcp") else {
                continue;
            };
            let fallback_host = fallback_records
                .iter()
                .find(|record| record.node == name)
                .and_then(|record| {
                    record
                        .names
                        .iter()
                        .find(|value| value.ends_with(".synapse.cynic"))
                        .cloned()
                })
                .unwrap_or_else(|| preferred_node_host(&node, None, &local_ip));
            let mut node_checks = Vec::new();
            for service in node.services.clone().unwrap_or_default() {
                if !is_l0_network_service(service.name.as_deref()) {
                    continue;
                }
                let Some(port) = service.port else { continue; };
                node_checks.push(ProviderServiceCheck {
                    node: name.clone(),
                    service: service.name.unwrap_or_else(|| "unknown".to_string()),
                    host: fallback_host.clone(),
                    ip: local_ip.clone(),
                    port,
                });
            }
            if node.role.as_deref() == Some("build_runner") {
                node_checks.push(ProviderServiceCheck {
                    node: name.clone(),
                    service: "ssh".to_string(),
                    host: fallback_host.clone(),
                    ip: local_ip.clone(),
                    port: 22,
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
