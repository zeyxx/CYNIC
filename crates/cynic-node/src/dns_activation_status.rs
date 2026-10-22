use std::fs;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::Path;

use cynic_host::registry::HostProfileResolution;
use serde::Serialize;
use serde_json::Value;

use crate::adapters::registry::LocalRegistryAdapter;
use crate::dns_contract::{
    DnsProviderObservedStatus, DnsProviderPolicySurface, default_dns_zone,
    dns_provider_policy_surface, observe_dns_provider_status, synapse_dns_corefile_path,
    synapse_dns_hosts_path,
};
use crate::network_host_substrate;
use crate::ports::network_interfaces_port::RegistryTopologyPort;

#[derive(Debug, Clone, Serialize)]
pub struct DnsActivationStatusReport {
    pub(crate) schema: &'static str,
    pub status: &'static str,
    host_profile: &'static str,
    checks: DnsActivationChecks,
    provider_policy: DnsProviderPolicySurface,
    provider_status: DnsProviderObservedStatus,
    missing: Vec<&'static str>,
    paths: DnsActivationPaths,
    mesh_status: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_resolution: Option<HostProfileResolution>,
    host_substrate: Value,
    host_substrate_plan: Value,
}

#[derive(Debug, Clone, Serialize)]
struct DnsActivationChecks {
    hosts_exists: bool,
    corefile_exists: bool,
    service_exists: bool,
    resolved_dropin_exists: bool,
    selected_provider_available: bool,
    selected_provider_provenance_verified: bool,
    hosts_contains_storage_record: bool,
    corefile_references_hosts: bool,
    service_references_corefile: bool,
    resolved_routes_synapse: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    synapse_dns_resolves: Option<bool>,
    /// The resolver listener itself answers TCP. Unit state lies
    /// (inactive while the port answers); only a connect proves liveness.
    /// None when the drop-in names no listener (nothing to probe).
    #[serde(skip_serializing_if = "Option::is_none")]
    listener_accepts: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct DnsActivationPaths {
    hosts: String,
    corefile: String,
    service: String,
    resolved_dropin: String,
}

pub fn build_dns_activation_status(
    registry: &str,
    hosts_path: &str,
    corefile_path: &str,
    service_path: &str,
    resolved_dropin_path: &str,
    resolve_dns: bool,
) -> Result<DnsActivationStatusReport, Box<dyn std::error::Error>> {
    let provider_status = observe_dns_provider_status();
    let provider_policy = dns_provider_policy_surface();
    let host_context = network_host_substrate::observe_network_host_runtime_context();
    build_dns_activation_status_with_runtime(
        registry,
        hosts_path,
        corefile_path,
        service_path,
        resolved_dropin_path,
        resolve_dns,
        provider_status,
        provider_policy,
        host_context.host_profile,
        host_context.host_resolution.clone(),
        serde_json::to_value(&host_context.host_substrate)?,
        serde_json::to_value(&host_context.host_substrate_plan)?,
        network_host_substrate::blocked_domains(&host_context),
    )
}

fn build_dns_activation_status_with_runtime(
    registry: &str,
    hosts_path: &str,
    corefile_path: &str,
    service_path: &str,
    resolved_dropin_path: &str,
    resolve_dns: bool,
    provider_status: DnsProviderObservedStatus,
    provider_policy: DnsProviderPolicySurface,
    host_profile: &'static str,
    host_resolution: Option<HostProfileResolution>,
    host_substrate: Value,
    host_substrate_plan: Value,
    blocked_domains: Vec<&'static str>,
) -> Result<DnsActivationStatusReport, Box<dyn std::error::Error>> {
    let mut mesh_status = serde_json::json!({});
    let registry_adapter = LocalRegistryAdapter::new(registry);
    let topology = registry_adapter.fetch_static_topology().unwrap_or_default();
    let dns_zone = registry_adapter
        .fetch_network_policy_summary()
        .ok()
        .and_then(|summary| summary.service_dns_zone)
        .unwrap_or_else(default_dns_zone);
    let synapse_dns_resolves = if resolve_dns {
        let dns_ok = topology_resolves_to_declared_addresses(&topology, &dns_zone);
        mesh_status = serde_json::json!({
            "schema": "cynic.infrastructure_mcp.network_mesh_status.v1",
            "dns_status": if dns_ok { "ok" } else { "violation" }
        });
        Some(dns_ok)
    } else {
        None
    };

    let listener_accepts = dropin_listener_addr(resolved_dropin_path)
        .map(|addr| listener_accepts_tcp(&addr));

    let checks = DnsActivationChecks {
        hosts_exists: Path::new(hosts_path).exists(),
        corefile_exists: Path::new(corefile_path).exists(),
        service_exists: Path::new(service_path).exists(),
        resolved_dropin_exists: Path::new(resolved_dropin_path).exists(),
        selected_provider_available: provider_status.selected_provider.is_some(),
        selected_provider_provenance_verified: provider_status.package_provenance_verified,
        hosts_contains_storage_record: hosts_contains_any_topology_record(hosts_path, &topology),
        corefile_references_hosts: file_contains(corefile_path, hosts_path)
            || file_contains(corefile_path, &synapse_dns_hosts_path()),
        service_references_corefile: file_contains(service_path, corefile_path)
            || file_contains(service_path, &synapse_dns_corefile_path()),
        resolved_routes_synapse: file_contains(resolved_dropin_path, &format!("~{dns_zone}")),
        synapse_dns_resolves,
        listener_accepts,
    };

    let mut missing = Vec::new();
    if !checks.hosts_exists {
        missing.push("hosts_exists");
    }
    if !checks.corefile_exists {
        missing.push("corefile_exists");
    }
    if !checks.service_exists {
        missing.push("service_exists");
    }
    if !checks.resolved_dropin_exists {
        missing.push("resolved_dropin_exists");
    }
    if !checks.selected_provider_available {
        missing.push("selected_provider_available");
    }
    if !checks.selected_provider_provenance_verified {
        missing.push("selected_provider_provenance_verified");
    }
    if !checks.hosts_contains_storage_record {
        missing.push("hosts_contains_storage_record");
    }
    if !checks.corefile_references_hosts {
        missing.push("corefile_references_hosts");
    }
    if !checks.service_references_corefile {
        missing.push("service_references_corefile");
    }
    if !checks.resolved_routes_synapse {
        missing.push("resolved_routes_synapse");
    }
    if resolve_dns && checks.synapse_dns_resolves == Some(false) {
        missing.push("synapse_dns_resolves");
    }
    if checks.listener_accepts == Some(false) {
        missing.push("listener_accepts");
    }
    for blocker in blocked_domains {
        missing.push(blocker);
    }

    Ok(DnsActivationStatusReport {
        schema: "cynic.organ_fleet.dns_activation_status.v1",
        status: if missing.is_empty() {
            "active"
        } else {
            "blocked"
        },
        host_profile,
        checks,
        provider_policy,
        provider_status,
        missing,
        paths: DnsActivationPaths {
            hosts: hosts_path.to_string(),
            corefile: corefile_path.to_string(),
            service: service_path.to_string(),
            resolved_dropin: resolved_dropin_path.to_string(),
        },
        mesh_status,
        host_resolution,
        host_substrate,
        host_substrate_plan,
    })
}

/// Listener address named by the resolved drop-in (`DNS=127.0.0.1:1053`).
/// The resolver is whatever the drop-in routes to — never a hardcoded port.
fn dropin_listener_addr(dropin_path: &str) -> Option<String> {
    let raw = std::fs::read_to_string(dropin_path).ok()?;
    for line in raw.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("DNS=") {
            let addr = value.trim().split_whitespace().next()?;
            if addr.contains(':') {
                return Some(addr.to_string());
            }
        }
    }
    None
}

/// TCP connect with a short timeout. The unit can say anything;
/// an accepted connection is the only liveness that counts.
fn listener_accepts_tcp(addr: &str) -> bool {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;
    let Ok(socket) = addr.parse::<SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&socket, Duration::from_secs(2)).is_ok()
}

fn topology_resolves_to_declared_addresses(
    topology: &[crate::ports::network_interfaces_port::RegistryNodeRecord],
    dns_zone: &str,
) -> bool {
    !topology.is_empty()
        && topology.iter().all(|record| {
            let Ok(expected) = record.ip.parse::<IpAddr>() else {
                return false;
            };
            record
                .names
                .iter()
                .filter(|name| name.ends_with(dns_zone))
                .any(|name| {
                    (name.as_str(), 0)
                        .to_socket_addrs()
                        .map(|addresses| {
                            addresses
                                .into_iter()
                                .any(|address| address.ip() == expected)
                        })
                        .unwrap_or(false)
                })
        })
}

fn file_contains(path: &str, needle: &str) -> bool {
    fs::read_to_string(path)
        .map(|value| value.contains(needle))
        .unwrap_or(false)
}

fn hosts_contains_any_topology_record(
    hosts_path: &str,
    topology: &[crate::ports::network_interfaces_port::RegistryNodeRecord],
) -> bool {
    let Ok(hosts_text) = fs::read_to_string(hosts_path) else {
        return false;
    };
    topology.iter().any(|record| {
        hosts_text.contains(&record.ip) && record.names.iter().any(|name| hosts_text.contains(name))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns_contract::{CANONICAL_DNS_BINARY, CANONICAL_DNS_PROVIDER};
    use cynic_types::paths::CynicPaths;

    #[test]
    fn dropin_listener_addr_reads_dns_line() {
        let dir = std::env::temp_dir();
        let path = dir.join("cynic-test-resolved.conf");
        std::fs::write(&path, "[Resolve]\nDNS=127.0.0.1:1053\nDomains=~synapse.cynic\n").unwrap();
        assert_eq!(
            dropin_listener_addr(&path.display().to_string()),
            Some("127.0.0.1:1053".to_string())
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn dropin_without_dns_line_probes_nothing() {
        let dir = std::env::temp_dir();
        let path = dir.join("cynic-test-resolved-empty.conf");
        std::fs::write(&path, "[Resolve]\nDomains=~synapse.cynic\n").unwrap();
        assert_eq!(dropin_listener_addr(&path.display().to_string()), None);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn open_port_accepts_closed_port_refuses() {
        let open = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = open.local_addr().unwrap().to_string();
        assert!(listener_accepts_tcp(&addr));
        let port = open.local_addr().unwrap().port();
        drop(open);
        assert!(!listener_accepts_tcp(&format!("127.0.0.1:{port}")));
    }

    #[test]
    fn blocked_when_required_paths_are_missing() {
        let report = build_dns_activation_status_with_runtime(
            &CynicPaths::registry_path().display().to_string(),
            "/missing/hosts",
            "/missing/corefile",
            "/missing/service",
            "/missing/dropin",
            false,
            DnsProviderObservedStatus {
                canonical_provider: CANONICAL_DNS_PROVIDER,
                canonical_binary: CANONICAL_DNS_BINARY,
                canonical_available: false,
                usable_provider: None,
                observed_binaries: Default::default(),
                noncanonical_available: Vec::new(),
                blocked_reason: Some("supported_dns_provider_missing"),
                selected_provider: None,
                selected_package: None,
                package_provenance_verified: false,
            },
            dns_provider_policy_surface(),
            "test-host",
            None,
            serde_json::json!({}),
            serde_json::json!({}),
            Vec::new(),
        )
        .expect("report");
        assert_eq!(report.status, "blocked");
        assert!(report.missing.contains(&"hosts_exists"));
    }
}
