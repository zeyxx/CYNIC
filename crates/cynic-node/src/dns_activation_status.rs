use std::fs;
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
pub(crate) struct DnsActivationStatusReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
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
    coredns_binary_available: bool,
    canonical_dns_provider_available: bool,
    hosts_contains_storage_record: bool,
    corefile_references_hosts: bool,
    service_references_corefile: bool,
    resolved_routes_synapse: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    synapse_dns_resolves: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct DnsActivationPaths {
    hosts: String,
    corefile: String,
    service: String,
    resolved_dropin: String,
}

pub(crate) fn build_dns_activation_status(
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
        .and_then(|summary| summary.synapse_dns_zone)
        .unwrap_or_else(default_dns_zone);
    let synapse_dns_resolves = if resolve_dns {
        let dns_ok = !topology.is_empty();
        mesh_status = serde_json::json!({
            "schema": "cynic.infrastructure_mcp.network_mesh_status.v1",
            "dns_status": if dns_ok { "ok" } else { "violation" }
        });
        Some(dns_ok)
    } else {
        None
    };

    let checks = DnsActivationChecks {
        hosts_exists: Path::new(hosts_path).exists(),
        corefile_exists: Path::new(corefile_path).exists(),
        service_exists: Path::new(service_path).exists(),
        resolved_dropin_exists: Path::new(resolved_dropin_path).exists(),
        coredns_binary_available: provider_status.canonical_available,
        canonical_dns_provider_available: provider_status.canonical_available,
        hosts_contains_storage_record: hosts_contains_any_topology_record(hosts_path, &topology),
        corefile_references_hosts: file_contains(corefile_path, hosts_path)
            || file_contains(corefile_path, &synapse_dns_hosts_path()),
        service_references_corefile: file_contains(service_path, corefile_path)
            || file_contains(service_path, &synapse_dns_corefile_path()),
        resolved_routes_synapse: file_contains(resolved_dropin_path, &format!("~{dns_zone}")),
        synapse_dns_resolves,
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
    if !checks.coredns_binary_available {
        missing.push("coredns_binary_available");
    }
    if !checks.canonical_dns_provider_available {
        missing.push("canonical_dns_provider_available");
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
    use cynic_host::registry::DEFAULT_REGISTRY_PATH;

    #[test]
    fn blocked_when_required_paths_are_missing() {
        let report = build_dns_activation_status_with_runtime(
            DEFAULT_REGISTRY_PATH,
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
                blocked_reason: Some("canonical_dns_provider_missing"),
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
