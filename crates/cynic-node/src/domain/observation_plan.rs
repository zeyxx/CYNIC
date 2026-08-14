use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use crate::adapters::registry::LocalRegistryAdapter;
use crate::ports::network_interfaces_port::RegistryTopologyPort;

#[derive(Debug, Clone)]
pub(crate) struct TcpCheck {
    pub(crate) host: String,
    pub(crate) port: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct ObservationPlan {
    pub(crate) source: String,
    pub(crate) registry_path: Option<String>,
    pub(crate) canonical_service_plane: String,
    pub(crate) synapse_cidr: String,
    pub(crate) provider_fallback_cidr: String,
    pub(crate) provider_fallback_gateway: Option<String>,
    pub(crate) dns_zone: String,
    pub(crate) critical_hosts: Vec<String>,
    pub(crate) expected_synapse_ips: BTreeMap<String, String>,
    pub(crate) tcp_checks: Vec<TcpCheck>,
}

impl ObservationPlan {
    fn bootstrap_fallback() -> Self {
        Self {
            source: "bootstrap_fallback".to_string(),
            registry_path: None,
            canonical_service_plane: "synapse".to_string(),
            synapse_cidr: "10.20.0.0/24".to_string(),
            provider_fallback_cidr: "10.10.10.0/24".to_string(),
            provider_fallback_gateway: None,
            dns_zone: "synapse.cynic".to_string(),
            critical_hosts: Vec::new(),
            expected_synapse_ips: BTreeMap::new(),
            tcp_checks: Vec::new(),
        }
    }
}

pub(crate) fn build_observation_plan(registry_path: Option<&str>) -> ObservationPlan {
    let Some(path) = registry_path else {
        return ObservationPlan::bootstrap_fallback();
    };
    if fs::read_to_string(path).is_err() {
        return ObservationPlan::bootstrap_fallback();
    }
    let adapter = LocalRegistryAdapter::new(path);

    let mut plan = ObservationPlan::bootstrap_fallback();
    plan.source = "registry".to_string();
    plan.registry_path = Some(path.to_string());

    if let Ok(summary) = adapter.fetch_network_policy_summary() {
        plan.canonical_service_plane = summary.canonical_service_plane;
        if let Some(value) = summary.synapse_cidr {
            plan.synapse_cidr = value;
        }
        if let Some(value) = summary.synapse_dns_zone {
            plan.dns_zone = value;
        }
    }
    if let Ok(route) = adapter.fetch_provider_fallback_route() {
        plan.provider_fallback_cidr = route.cidr;
        plan.provider_fallback_gateway = route.gateway;
    }
    if let Ok(topology) = adapter.fetch_static_topology() {
        for record in &topology {
            for host in &record.names {
                if host.ends_with(&plan.dns_zone) {
                    plan.expected_synapse_ips
                        .insert(host.clone(), record.ip.clone());
                }
            }
        }
    }
    if let Ok(service_checks) = adapter.fetch_provider_fallback_service_checks() {
        let mut tcp_checks = service_checks
            .into_iter()
            .map(|check| TcpCheck {
                host: check.host,
                port: check.port,
            })
            .collect::<Vec<_>>();
        tcp_checks.sort_by(|left, right| (&left.host, left.port).cmp(&(&right.host, right.port)));
        tcp_checks.dedup_by(|left, right| left.host == right.host && left.port == right.port);
        if !tcp_checks.is_empty() {
            plan.critical_hosts = tcp_checks
                .iter()
                .map(|check| check.host.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            plan.tcp_checks = tcp_checks;
        } else if !plan.expected_synapse_ips.is_empty() {
            plan.critical_hosts = plan
                .expected_synapse_ips
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
        }
    }

    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn registry_plan_keeps_l0_network_checks_on_critical_active_roles() {
        let mut file = tempfile::NamedTempFile::new().expect("temp registry");
        write!(
            file,
            r#"{{
              "network_policy": {{
                "canonical_service_plane": "synapse",
                "planes": {{
                  "synapse": {{"cidr": "10.20.0.0/24", "dns_zone": "synapse.cynic"}},
                  "proxmox_guest": {{"cidr": "10.10.10.0/24"}}
                }},
                "reservations": [
                  {{"name": "kernel", "dns": ["kernel.synapse.cynic"]}},
                  {{"name": "staging", "dns": ["staging.synapse.cynic"]}},
                  {{"name": "builder", "dns": ["builder.synapse.cynic"]}}
                ]
              }},
              "nodes": [
                {{
                  "name": "kernel",
                  "role": "kernel_primary",
                  "status": "active",
                  "aliases": [],
                  "network": {{"local": "10.10.10.5"}},
                  "services": [
                    {{"name": "cynic-kernel", "port": 3030}}
                  ]
                }},
                {{
                  "name": "staging",
                  "role": "kernel_staging",
                  "status": "planned",
                  "aliases": [],
                  "network": {{"local": "10.10.10.6"}},
                  "services": [{{"name": "cynic-kernel", "port": 3030}}]
                }},
                {{
                  "name": "builder",
                  "role": "build_runner",
                  "status": "active",
                  "aliases": [],
                  "network": {{"local": "10.10.10.4"}},
                  "services": [
                    {{"name": "ssh", "port": 22}},
                    {{"name": "gate-2"}}
                  ]
                }}
              ]
            }}"#
        )
        .expect("write registry");

        let path = file.path().to_string_lossy().to_string();
        let plan = build_observation_plan(Some(&path));

        assert_eq!(plan.source, "registry");
        assert_eq!(
            plan.critical_hosts,
            vec![
                "builder.synapse.cynic".to_string(),
                "kernel.synapse.cynic".to_string(),
            ]
        );
        assert_eq!(
            plan.tcp_checks
                .iter()
                .map(|check| (check.host.as_str(), check.port))
                .collect::<Vec<_>>(),
            vec![
                ("builder.synapse.cynic", 22),
                ("kernel.synapse.cynic", 3030),
            ]
        );
    }
}
