//! Network security audit use case (DDD/FCIS hexagonal).
//!
//! Orchestrates reading telemetry from `cynic_host`, resolving the allowed-ports
//! policy from the registry, and delegating pure evaluation to
//! `cynic_intel::soc::evaluate_network_security`.

use cynic_host::adapters::local_network::LocalNetworkAdapter;
use cynic_host::registry::resolve_host_profile_resolution;
use cynic_host::runtime_layout::observed_hostname;
use cynic_intel::soc::evaluate_network_security;
use cynic_network::extract_allowed_ports_from_registry;

pub fn execute_network_security_audit(
    registry: &str,
    max_handshake_age_secs: u64,
) -> cynic_intel::soc::NetworkSecurityAuditReport {
    let hostname = observed_hostname();
    let node_id = resolve_host_profile_resolution(&hostname, Some(registry))
        .ok()
        .and_then(|r| r.matched_node)
        .unwrap_or(hostname);

    let adapter = LocalNetworkAdapter::new();
    let snapshot = adapter.observe_live_telemetry(&node_id, Some("wg0"));

    let registry_json = std::fs::read_to_string(registry)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
    let allowed_ports = registry_json
        .as_ref()
        .map(|reg| extract_allowed_ports_from_registry(reg, Some(&node_id)))
        .unwrap_or_else(|| vec![22, 3030, 44196, 51820, 9090]);

    evaluate_network_security(&snapshot, &allowed_ports, max_handshake_age_secs * 1000)
}
