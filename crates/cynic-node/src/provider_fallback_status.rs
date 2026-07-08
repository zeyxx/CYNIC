use std::fs;
use std::path::Path;

use chrono::Utc;
use serde::Serialize;

use crate::adapters::provider_fallback_probe::LocalProviderFallbackProbe;
use crate::adapters::registry::LocalRegistryAdapter;
use crate::ports::network_interfaces_port::{
    ProviderFallbackProbePort, ProviderServiceConnectivity, RegistryTopologyPort,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderFallbackStatusReport {
    pub(crate) schema: &'static str,
    generated_at: String,
    pub(crate) status: &'static str,
    canonical: bool,
    registry_ref: String,
    hosts_path: String,
    checks: ProviderFallbackChecks,
    missing: Vec<&'static str>,
    hosts_missing: Vec<String>,
    route: serde_json::Value,
    connectivity: Vec<ProviderServiceConnectivity>,
    warning: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderFallbackChecks {
    hosts_file_exists: bool,
    hosts_entries_present: bool,
    route_active: bool,
    route_persistent: bool,
    connectivity_checked: bool,
    network_connectivity_ok: Option<bool>,
}

pub(crate) fn build_provider_fallback_status(
    registry: &str,
    hosts_path: &str,
    check_connectivity: bool,
) -> Result<ProviderFallbackStatusReport, Box<dyn std::error::Error>> {
    let registry_adapter = LocalRegistryAdapter::new(registry);
    let route = registry_adapter.fetch_provider_fallback_route()?;
    let service_checks = registry_adapter.fetch_provider_fallback_service_checks()?;
    let probe = LocalProviderFallbackProbe;
    let route_status = probe.observe_route(&route)?;
    let connectivity = if check_connectivity {
        probe.observe_connectivity(&service_checks)?
    } else {
        Vec::new()
    };

    let hosts_text = fs::read_to_string(hosts_path).unwrap_or_default();
    let hosts_file_exists = Path::new(hosts_path).exists();
    let hosts_missing = service_checks
        .iter()
        .filter(|check| check.host.ends_with(".synapse.cynic") && !hosts_text.contains(&check.host))
        .map(|check| check.host.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let blocked_connectivity = connectivity
        .iter()
        .any(|item| item.network_status == "blocked");

    let mut missing = Vec::new();
    if !hosts_file_exists {
        missing.push("provider_fallback_hosts_file");
    }
    if !hosts_missing.is_empty() {
        missing.push("provider_fallback_hosts_entries");
    }
    if !route_status.active {
        missing.push("provider_fallback_route_active");
    }
    if !route_status.persistent {
        missing.push("provider_fallback_route_persistent");
    }
    if check_connectivity && blocked_connectivity {
        missing.push("provider_fallback_network_connectivity");
    }

    Ok(ProviderFallbackStatusReport {
        schema: "cynic.organ_fleet.provider_fallback_status.v1",
        generated_at: Utc::now().to_rfc3339(),
        status: if missing.is_empty() { "active" } else { "blocked" },
        canonical: false,
        registry_ref: registry.to_string(),
        hosts_path: hosts_path.to_string(),
        checks: ProviderFallbackChecks {
            hosts_file_exists,
            hosts_entries_present: hosts_missing.is_empty(),
            route_active: route_status.active,
            route_persistent: route_status.persistent,
            connectivity_checked: check_connectivity,
            network_connectivity_ok: check_connectivity.then_some(!blocked_connectivity),
        },
        missing,
        hosts_missing,
        route: serde_json::to_value(route_status)?,
        connectivity,
        warning: "Provider fallback is a resurrection path only while Synapse/WireGuard remains inactive.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn report_marks_missing_hosts_file_as_blocked() {
        let mut registry = tempfile::NamedTempFile::new().expect("temp registry");
        write!(
            registry,
            r#"{{
              "network_policy": {{
                "canonical_service_plane": "synapse",
                "planes": {{
                  "synapse": {{"cidr": "10.20.0.0/24", "dns_zone": "synapse.cynic"}},
                  "proxmox_guest": {{"cidr": "10.10.10.0/24", "canonical": false}}
                }},
                "reservations": [
                  {{"name": "forge", "synapse": "10.20.0.4", "dns": ["forge.synapse.cynic"]}}
                ]
              }},
              "nodes": [
                {{
                  "name": "cynic-host-proxmox",
                  "role": "hypervisor",
                  "network": {{"local": "192.168.0.37"}}
                }},
                {{
                  "name": "forge",
                  "role": "build_runner",
                  "status": "active",
                  "network": {{"local": "10.10.10.4", "synapse": "10.20.0.4"}},
                  "services": [{{"name": "forgejo", "port": 3000}}]
                }}
              ]
            }}"#
        )
        .expect("write registry");

        let report = build_provider_fallback_status(
            registry.path().to_str().expect("path"),
            "/definitely/missing/provider-fallback.hosts",
            false,
        )
        .expect("report");

        assert_eq!(report.schema, "cynic.organ_fleet.provider_fallback_status.v1");
        assert_eq!(report.status, "blocked");
        assert!(report.missing.contains(&"provider_fallback_hosts_file"));
    }
}
