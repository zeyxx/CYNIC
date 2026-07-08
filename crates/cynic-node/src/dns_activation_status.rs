use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::adapters::registry::LocalRegistryAdapter;
use crate::ports::network_interfaces_port::RegistryTopologyPort;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DnsActivationStatusReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    checks: DnsActivationChecks,
    provider_policy: ProviderPolicy,
    provider_status: ProviderStatus,
    missing: Vec<&'static str>,
    paths: DnsActivationPaths,
    mesh_status: serde_json::Value,
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
struct ProviderPolicy {
    canonical_provider: &'static str,
    canonical_binary: &'static str,
    selection_source: &'static str,
    service_contract: &'static str,
    approved_sources: Vec<&'static str>,
    known_noncanonical_candidates: Vec<&'static str>,
    promotion_rule: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderStatus {
    canonical_provider: &'static str,
    canonical_binary: &'static str,
    canonical_available: bool,
    usable_provider: Option<&'static str>,
    observed_binaries: std::collections::BTreeMap<&'static str, Option<String>>,
    noncanonical_available: Vec<&'static str>,
    blocked_reason: Option<&'static str>,
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
    let provider_status = dns_provider_status();
    let provider_policy = dns_provider_policy();
    let mut mesh_status = serde_json::json!({});
    let synapse_dns_resolves = if resolve_dns {
        let registry_adapter = LocalRegistryAdapter::new(registry);
        let topology = registry_adapter.fetch_static_topology()?;
        let dns_ok = topology.iter().any(|record| record.node == "organ-surrealdb");
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
        hosts_contains_storage_record: file_contains(hosts_path, "organ-surrealdb.synapse.cynic")
            && file_contains(hosts_path, "10.20.0.3"),
        corefile_references_hosts: file_contains(corefile_path, hosts_path)
            || file_contains(corefile_path, "/etc/coredns/synapse.hosts"),
        service_references_corefile: file_contains(service_path, corefile_path)
            || file_contains(service_path, "/etc/coredns/Corefile"),
        resolved_routes_synapse: file_contains(resolved_dropin_path, "~synapse.cynic"),
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

    Ok(DnsActivationStatusReport {
        schema: "cynic.organ_fleet.dns_activation_status.v1",
        status: if missing.is_empty() { "active" } else { "blocked" },
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
    })
}

fn file_contains(path: &str, needle: &str) -> bool {
    fs::read_to_string(path).map(|value| value.contains(needle)).unwrap_or(false)
}

fn dns_provider_policy() -> ProviderPolicy {
    ProviderPolicy {
        canonical_provider: "coredns",
        canonical_binary: "coredns",
        selection_source: "organ-fleet Synapse DNS activation contract",
        service_contract: "*.synapse.cynic",
        approved_sources: vec!["distribution_package", "approved_internal_package_mirror"],
        known_noncanonical_candidates: vec!["dnsmasq", "unbound", "bind9", "systemd-resolved"],
        promotion_rule: "A candidate provider becomes usable only after a dedicated adapter contract, bundle renderer, service template, and activation-status checks are added.",
    }
}

fn dns_provider_status() -> ProviderStatus {
    let mut observed_binaries = std::collections::BTreeMap::new();
    let mut noncanonical_available = Vec::new();
    for binary in ["coredns", "dnsmasq", "unbound", "named", "resolvectl"] {
        let path = find_on_path(binary);
        if binary != "coredns" && path.is_some() {
            noncanonical_available.push(match binary {
                "named" => "named",
                other => other,
            });
        }
        observed_binaries.insert(binary, path);
    }
    let canonical_available = observed_binaries
        .get("coredns")
        .and_then(|value| value.as_ref())
        .is_some();
    ProviderStatus {
        canonical_provider: "coredns",
        canonical_binary: "coredns",
        canonical_available,
        usable_provider: canonical_available.then_some("coredns"),
        observed_binaries,
        noncanonical_available,
        blocked_reason: (!canonical_available).then_some("canonical_dns_provider_missing"),
    }
}

fn find_on_path(binary: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&path) {
        let candidate = entry.join(binary);
        if candidate.is_file() {
            return Some(candidate.display().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_when_required_paths_are_missing() {
        let report = build_dns_activation_status(
            "infra/registry.json",
            "/missing/hosts",
            "/missing/corefile",
            "/missing/service",
            "/missing/dropin",
            false,
        )
        .expect("report");
        assert_eq!(report.status, "blocked");
        assert!(report.missing.contains(&"hosts_exists"));
    }
}
