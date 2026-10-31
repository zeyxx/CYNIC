//! Dynamic Peer & Service Discovery Domain for CYNIC Network
//! Follows CYNIC Coherence Pattern: Pure Observation Model -> Drift Detection -> Sealed Readback Proof.

use cynic_protocol::plane::Plane;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NETWORK_DISCOVERY_REPORT_SCHEMA: &str = "cynic.network.discovery_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeReachability {
    Reachable { latency_ms: u64, protocol: String },
    Degraded { reason: String },
    Unreachable { last_seen_unix_ms: Option<u64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub name: String,
    pub endpoint: String,
    pub port: u16,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredNode {
    pub node_id: String,
    pub direct_ip: Option<String>,
    pub synapse_ip: Option<String>,
    pub wireguard_ip: Option<String>,
    pub reachability: NodeReachability,
    pub services: Vec<DiscoveredService>,
    pub observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDiscoveryReport {
    pub schema: String,
    pub observer_node_id: String,
    pub nodes: BTreeMap<String, DiscoveredNode>,
    pub drift_detected: bool,
    pub drift_summary: Vec<String>,
    pub observed_at_unix_ms: u64,
}

pub trait NetworkDiscoveryProbePort: Send + Sync {
    fn probe_tcp(&self, host: &str, port: u16, timeout_ms: u64) -> Result<u64, String>;
    fn probe_wireguard_handshakes(&self, iface: &str) -> Result<BTreeMap<String, u64>, String>;
    fn probe_http_health(&self, _url: &str, _timeout_ms: u64) -> Result<(), String> {
        Err("probe_http_health not implemented".to_string())
    }
}

/// Pure evaluation of network cluster state against desired registry nodes.
pub fn evaluate_network_drift(
    observed_nodes: &[DiscoveredNode],
    desired_nodes: &BTreeMap<String, Vec<String>>,
) -> (bool, Vec<String>) {
    let mut drifts = Vec::new();
    for (desired_id, expected_ips) in desired_nodes {
        if let Some(obs) = observed_nodes.iter().find(|n| &n.node_id == desired_id) {
            match &obs.reachability {
                NodeReachability::Unreachable { .. } => {
                    drifts.push(format!(
                        "Node {} is declared in registry but unreachable",
                        desired_id
                    ));
                }
                NodeReachability::Degraded { reason } => {
                    drifts.push(format!("Node {} is degraded: {}", desired_id, reason));
                }
                NodeReachability::Reachable { .. } => {
                    if !expected_ips.is_empty() {
                        let matches_any = expected_ips.iter().any(|exp| {
                            obs.direct_ip.as_deref() == Some(exp.as_str())
                                || obs.synapse_ip.as_deref() == Some(exp.as_str())
                                || obs.wireguard_ip.as_deref() == Some(exp.as_str())
                        });
                        if !matches_any
                            && (obs.direct_ip.is_some()
                                || obs.synapse_ip.is_some()
                                || obs.wireguard_ip.is_some())
                        {
                            drifts.push(format!(
                                "Node {} IP mismatch: expected one of {:?}",
                                desired_id, expected_ips
                            ));
                        }
                    }
                }
            }
        } else {
            drifts.push(format!(
                "Node {} is absent from live observations",
                desired_id
            ));
        }
    }
    let has_drift = !drifts.is_empty();
    (has_drift, drifts)
}

/// Node network descriptor extracted from desired registry state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredNodeNetworkSpec {
    pub node_id: String,
    pub direct_ip: Option<String>,
    pub synapse_ip: Option<String>,
    pub wireguard_ip: Option<String>,
    pub wireguard_public_key: Option<String>,
    pub probe_ports: Vec<u16>,
}

/// Pure cluster node discovery over the injected probe port.
pub fn discover_cluster_nodes(
    observer_node_id: &str,
    specs: &[DesiredNodeNetworkSpec],
    probe: &dyn NetworkDiscoveryProbePort,
    timeout_ms: u64,
    now_unix_ms: u64,
) -> NetworkDiscoveryReport {
    let mut nodes = BTreeMap::new();
    let wg_handshakes = probe.probe_wireguard_handshakes("wg0").unwrap_or_default();

    for spec in specs {
        let is_self = spec.node_id == observer_node_id;
        let reachability = if is_self {
            NodeReachability::Reachable {
                latency_ms: 0,
                protocol: "local".to_string(),
            }
        } else {
            let mut reachable_via_wg = false;
            if let Some(ref pk) = spec.wireguard_public_key {
                if let Some(hs_ms) = wg_handshakes.get(pk) {
                    let age_ms = now_unix_ms.saturating_sub(*hs_ms);
                    if age_ms < 180_000 {
                        reachable_via_wg = true;
                    }
                }
            }

            let mut tested_ok = None;
            let target_ips: Vec<&str> = [
                spec.synapse_ip.as_deref(),
                spec.wireguard_ip.as_deref(),
                spec.direct_ip.as_deref(),
            ]
            .into_iter()
            .flatten()
            .collect();

            let ports_to_try = if spec.probe_ports.is_empty() {
                vec![22]
            } else {
                spec.probe_ports.clone()
            };

            'probe_loop: for ip in target_ips {
                for &port in &ports_to_try {
                    if let Ok(lat) = probe.probe_tcp(ip, port, timeout_ms) {
                        tested_ok = Some((lat, format!("tcp:{}", port)));
                        break 'probe_loop;
                    }
                }
            }

            match (tested_ok, reachable_via_wg) {
                (Some((latency_ms, protocol)), _) => NodeReachability::Reachable {
                    latency_ms,
                    protocol,
                },
                (None, true) => NodeReachability::Reachable {
                    latency_ms: 5,
                    protocol: "wireguard_handshake".to_string(),
                },
                (None, false) => NodeReachability::Unreachable {
                    last_seen_unix_ms: None,
                },
            }
        };

        nodes.insert(
            spec.node_id.clone(),
            DiscoveredNode {
                node_id: spec.node_id.clone(),
                direct_ip: spec.direct_ip.clone(),
                synapse_ip: spec.synapse_ip.clone(),
                wireguard_ip: spec.wireguard_ip.clone(),
                reachability,
                services: vec![],
                observed_at_unix_ms: now_unix_ms,
            },
        );
    }

    let desired_map: BTreeMap<String, Vec<String>> = specs
        .iter()
        .map(|s| {
            let mut ips = Vec::new();
            if let Some(ref ip) = s.synapse_ip {
                ips.push(ip.clone());
            }
            if let Some(ref ip) = s.wireguard_ip {
                ips.push(ip.clone());
            }
            if let Some(ref ip) = s.direct_ip {
                ips.push(ip.clone());
            }
            (s.node_id.clone(), ips)
        })
        .collect();

    let observed_vec: Vec<DiscoveredNode> = nodes.values().cloned().collect();
    let (drift_detected, drift_summary) = evaluate_network_drift(&observed_vec, &desired_map);

    NetworkDiscoveryReport {
        schema: NETWORK_DISCOVERY_REPORT_SCHEMA.to_string(),
        observer_node_id: observer_node_id.to_string(),
        nodes,
        drift_detected,
        drift_summary,
        observed_at_unix_ms: now_unix_ms,
    }
}

/// Pure extractor of desired node network specs from a registry JSON Value.
pub fn extract_desired_network_specs(registry: &serde_json::Value) -> Vec<DesiredNodeNetworkSpec> {
    let mut specs = Vec::new();
    let canonical_service_plane = registry
        .pointer("/network_policy/canonical_service_plane")
        .and_then(|v| v.as_str())
        .unwrap_or("synapse");

    let mut reservations_service: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut reservations_direct: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    if let Some(reservations) = registry
        .pointer("/network_policy/reservations")
        .and_then(|v| v.as_array())
    {
        for r in reservations {
            if let Some(name) = r.get("name").and_then(|v| v.as_str()) {
                if let Some(syn) = r
                    .get(canonical_service_plane)
                    .or_else(|| r.get("synapse"))
                    .and_then(|v| v.as_str())
                {
                    reservations_service.insert(name.to_string(), syn.to_string());
                }
                if let Some(mgmt) = r
                    .get(Plane::Admin.name())
                    .or_else(|| r.get("management"))
                    .or_else(|| r.get("direct"))
                    .and_then(|v| v.as_str())
                {
                    reservations_direct.insert(name.to_string(), mgmt.to_string());
                }
            }
        }
    }

    let mut wg_nodes: std::collections::HashMap<String, (String, Option<String>)> =
        std::collections::HashMap::new();
    if let Some(wg) = registry
        .pointer("/network_policy/wireguard/nodes")
        .and_then(|v| v.as_array())
    {
        for w in wg {
            if let Some(nid) = w.get("node_id").and_then(|v| v.as_str()) {
                let overlay_ip = w
                    .get("overlay_ip")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let pubkey = w
                    .get("public_key")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                wg_nodes.insert(nid.to_string(), (overlay_ip, pubkey));
            }
        }
    }

    if let Some(nodes) = registry.get("nodes").and_then(|v| v.as_array()) {
        for n in nodes {
            let node_id = n
                .get("id")
                .or_else(|| n.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if node_id.is_empty() {
                continue;
            }
            let name = n.get("name").and_then(|v| v.as_str()).unwrap_or(node_id);

            let node_net = n.get("network");
            let synapse_ip = node_net
                .and_then(|net| {
                    net.get(canonical_service_plane)
                        .or_else(|| net.get("synapse"))
                })
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| reservations_service.get(name).cloned())
                .or_else(|| reservations_service.get(node_id).cloned());

            let direct_ip = node_net
                .and_then(|net| {
                    net.get(Plane::Admin.name())
                        .or_else(|| net.get("management"))
                        .or_else(|| net.get("direct"))
                        .or_else(|| net.get("local"))
                })
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| reservations_direct.get(name).cloned())
                .or_else(|| reservations_direct.get(node_id).cloned());

            let (wireguard_ip, wireguard_public_key) = match wg_nodes.get(node_id) {
                Some((ip, pk)) => (
                    if ip.is_empty() {
                        None
                    } else {
                        Some(ip.clone())
                    },
                    pk.clone(),
                ),
                None => (None, None),
            };

            let mut probe_ports = Vec::new();
            if let Some(services) = n.get("services").and_then(|v| v.as_array()) {
                for s in services {
                    if let Some(port) = s.get("port").and_then(|v| v.as_u64()) {
                        probe_ports.push(port as u16);
                    }
                }
            }

            specs.push(DesiredNodeNetworkSpec {
                node_id: node_id.to_string(),
                direct_ip,
                synapse_ip,
                wireguard_ip,
                wireguard_public_key,
                probe_ports,
            });
        }
    }

    specs
}

/// Pure extraction of service endpoints declared in a registry JSON value.
///
/// Inspects `/nodes`, each node's `services` array, matches `service.name == service_name`,
/// and extracts a `DiscoveredService` using `bench_endpoint` or `endpoint` (preferring
/// `bench_endpoint`), defaulting the port from the URL when no explicit `port` is present.
pub fn service_endpoints_from_registry(
    registry_val: &serde_json::Value,
    service_name: &str,
) -> Vec<DiscoveredService> {
    let mut out = Vec::new();
    let nodes = match registry_val.get("nodes").and_then(|v| v.as_array()) {
        Some(n) => n,
        None => return out,
    };
    for node in nodes {
        let services = match node.get("services").and_then(|v| v.as_array()) {
            Some(s) => s,
            None => continue,
        };
        for svc in services {
            if svc.get("name").and_then(|v| v.as_str()) != Some(service_name) {
                continue;
            }
            let endpoint_raw = svc
                .get("bench_endpoint")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    svc.get("endpoint")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                });
            let endpoint = match endpoint_raw {
                Some(e) => e.to_string(),
                None => continue,
            };
            let port = svc
                .get("port")
                .and_then(|v| v.as_u64())
                .map(|p| p as u16)
                .unwrap_or_else(|| port_from_url(&endpoint));
            out.push(DiscoveredService {
                name: svc
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(service_name)
                    .to_string(),
                endpoint,
                port,
                status: "declared".to_string(),
            });
        }
    }
    out
}

fn port_from_url(url: &str) -> u16 {
    let no_frag = url.split('#').next().unwrap_or(url);
    let no_query = no_frag.split('?').next().unwrap_or(no_frag);
    let after_scheme = no_query.split("://").nth(1).unwrap_or(no_query);
    let host_port = after_scheme.split('/').next().unwrap_or(after_scheme);
    if let Some(idx) = host_port.rfind(':') {
        let port_str = &host_port[idx + 1..];
        if let Ok(p) = port_str.parse::<u16>() {
            return p;
        }
    }
    0
}

/// Resolve service endpoints from the workspace registry.
///
/// Walks up from the current directory to find `infra/registry.json`, parses it,
/// calls `service_endpoints_from_registry`, and returns the endpoint strings.
/// Returns `None` when no registry resolves or no endpoints are found.
pub fn endpoints_from_workspace_registry(service_name: &str) -> Option<Vec<String>> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("infra/registry.json");
        if candidate.is_file() {
            let data = std::fs::read_to_string(&candidate).ok()?;
            let registry: serde_json::Value = serde_json::from_str(&data).ok()?;
            let services = service_endpoints_from_registry(&registry, service_name);
            let endpoints: Vec<String> = services.into_iter().map(|s| s.endpoint).collect();
            if endpoints.is_empty() {
                return None;
            }
            return Some(endpoints);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Helper to populate `services` for a discovered node from a registry value.
pub fn services_for_node(
    registry_val: &serde_json::Value,
    node_id: &str,
) -> Vec<DiscoveredService> {
    if let Some(nodes) = registry_val.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            let id = node
                .get("id")
                .or_else(|| node.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if id != node_id {
                continue;
            }
            if let Some(services) = node.get("services").and_then(|v| v.as_array()) {
                let mut out = Vec::new();
                for svc in services {
                    let name = svc
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let endpoint_raw = svc
                        .get("bench_endpoint")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .or_else(|| {
                            svc.get("endpoint")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                        });
                    let endpoint = endpoint_raw.map(str::to_string).unwrap_or_default();
                    let port = svc
                        .get("port")
                        .and_then(|v| v.as_u64())
                        .map(|p| p as u16)
                        .unwrap_or_else(|| port_from_url(&endpoint));
                    out.push(DiscoveredService {
                        name,
                        endpoint,
                        port,
                        status: "declared".to_string(),
                    });
                }
                return out;
            }
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_unreachable_node_drift() {
        let desired: BTreeMap<String, Vec<String>> = [
            (
                "cynic-builder".to_string(),
                vec!["10.10.10.119".to_string()],
            ),
            ("local-kernel".to_string(), vec!["10.10.10.100".to_string()]),
        ]
        .into_iter()
        .collect();

        let observed = vec![
            DiscoveredNode {
                node_id: "cynic-builder".to_string(),
                direct_ip: None,
                synapse_ip: Some("10.10.10.119".to_string()),
                wireguard_ip: Some("10.20.0.5".to_string()),
                reachability: NodeReachability::Reachable {
                    latency_ms: 2,
                    protocol: "tcp".to_string(),
                },
                services: vec![],
                observed_at_unix_ms: 1000,
            },
            DiscoveredNode {
                node_id: "local-kernel".to_string(),
                direct_ip: None,
                synapse_ip: Some("10.10.10.100".to_string()),
                wireguard_ip: None,
                reachability: NodeReachability::Unreachable {
                    last_seen_unix_ms: None,
                },
                services: vec![],
                observed_at_unix_ms: 1000,
            },
        ];

        let (drift, reasons) = evaluate_network_drift(&observed, &desired);
        assert!(drift);
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("local-kernel"));
    }

    struct MockDiscoveryProbe {
        reachable_ports: Vec<(String, u16)>,
        wg_peers: BTreeMap<String, u64>,
    }

    impl NetworkDiscoveryProbePort for MockDiscoveryProbe {
        fn probe_tcp(&self, host: &str, port: u16, _timeout_ms: u64) -> Result<u64, String> {
            if self
                .reachable_ports
                .iter()
                .any(|(h, p)| h == host && *p == port)
            {
                Ok(12)
            } else {
                Err("connection refused".to_string())
            }
        }

        fn probe_wireguard_handshakes(
            &self,
            _iface: &str,
        ) -> Result<BTreeMap<String, u64>, String> {
            Ok(self.wg_peers.clone())
        }
    }

    #[test]
    fn discovers_nodes_with_real_spec_and_mock_port() {
        let specs = vec![
            DesiredNodeNetworkSpec {
                node_id: "cortex".to_string(),
                direct_ip: Some("192.168.100.2".to_string()),
                synapse_ip: None,
                wireguard_ip: Some("10.20.0.3".to_string()),
                wireguard_public_key: Some("pk_cortex".to_string()),
                probe_ports: vec![22],
            },
            DesiredNodeNetworkSpec {
                node_id: "cynic-builder".to_string(),
                direct_ip: None,
                synapse_ip: Some("10.10.10.119".to_string()),
                wireguard_ip: Some("10.20.0.5".to_string()),
                wireguard_public_key: Some("pk_builder".to_string()),
                probe_ports: vec![9090, 22],
            },
        ];

        let probe = MockDiscoveryProbe {
            reachable_ports: vec![("10.10.10.119".to_string(), 9090)],
            wg_peers: BTreeMap::new(),
        };

        let report = discover_cluster_nodes("cortex", &specs, &probe, 500, 1000);
        assert_eq!(report.nodes.len(), 2);
        assert_eq!(
            report.nodes["cortex"].reachability,
            NodeReachability::Reachable {
                latency_ms: 0,
                protocol: "local".to_string(),
            }
        );
        assert_eq!(
            report.nodes["cynic-builder"].reachability,
            NodeReachability::Reachable {
                latency_ms: 12,
                protocol: "tcp:9090".to_string(),
            }
        );
        assert!(!report.drift_detected);
    }

    #[test]
    fn extracts_specs_dynamically_from_nodes_and_reservations() {
        let registry = serde_json::json!({
            "schema": "cynic.network.registry.v1",
            "network_policy": {
                "canonical_service_plane": "custom_mesh",
                "reservations": [
                    {
                        "name": "node-b",
                        "custom_mesh": "10.42.0.2",
                        "management": "192.168.1.2"
                    }
                ],
                "wireguard": {
                    "nodes": [
                        {
                            "node_id": "node-a",
                            "overlay_ip": "10.99.0.1",
                            "public_key": "pk_a"
                        }
                    ]
                }
            },
            "nodes": [
                {
                    "id": "node-a",
                    "network": {
                        "custom_mesh": "10.42.0.1",
                        "direct": "192.168.1.1"
                    },
                    "services": [
                        {"name": "rpc", "port": 8080}
                    ]
                },
                {
                    "id": "node-b",
                    "name": "node-b"
                }
            ]
        });

        let specs = extract_desired_network_specs(&registry);
        assert_eq!(specs.len(), 2);

        let spec_a = specs.iter().find(|s| s.node_id == "node-a").unwrap();
        assert_eq!(spec_a.synapse_ip.as_deref(), Some("10.42.0.1"));
        assert_eq!(spec_a.direct_ip.as_deref(), Some("192.168.1.1"));
        assert_eq!(spec_a.wireguard_ip.as_deref(), Some("10.99.0.1"));
        assert_eq!(spec_a.wireguard_public_key.as_deref(), Some("pk_a"));
        assert_eq!(spec_a.probe_ports, vec![8080]);

        let spec_b = specs.iter().find(|s| s.node_id == "node-b").unwrap();
        assert_eq!(spec_b.synapse_ip.as_deref(), Some("10.42.0.2"));
        assert_eq!(spec_b.direct_ip.as_deref(), Some("192.168.1.2"));
    }

    #[test]
    fn service_endpoints_from_registry_extracts_bench_and_endpoint() {
        let registry = serde_json::json!({
            "nodes": [
                {
                    "id": "node-a",
                    "services": [
                        {"name": "embedding-server", "bench_endpoint": "http://10.0.0.1:8080/v1#slots=4", "port": 8080},
                        {"name": "other", "endpoint": "http://x", "port": 90}
                    ]
                },
                {
                    "id": "node-b",
                    "services": [
                        {"name": "embedding-server", "endpoint": "http://10.0.0.2:9000", "port": 9000}
                    ]
                }
            ]
        });
        let svcs = service_endpoints_from_registry(&registry, "embedding-server");
        assert_eq!(svcs.len(), 2);
        assert_eq!(svcs[0].endpoint, "http://10.0.0.1:8080/v1#slots=4");
        assert_eq!(svcs[0].port, 8080);
        assert_eq!(svcs[0].status, "declared");
        assert_eq!(svcs[1].endpoint, "http://10.0.0.2:9000");
        assert_eq!(svcs[1].port, 9000);
    }

    #[test]
    fn service_endpoints_from_registry_defaults_port_from_url() {
        let registry = serde_json::json!({
            "nodes": [
                {
                    "id": "node-a",
                    "services": [
                        {"name": "svc", "bench_endpoint": "http://10.0.0.1:7777/path"}
                    ]
                }
            ]
        });
        let svcs = service_endpoints_from_registry(&registry, "svc");
        assert_eq!(svcs.len(), 1);
        assert_eq!(svcs[0].port, 7777);
    }

    #[test]
    fn service_endpoints_from_registry_skips_missing_endpoint() {
        let registry = serde_json::json!({
            "nodes": [
                {
                    "id": "node-a",
                    "services": [
                        {"name": "svc", "port": 80}
                    ]
                }
            ]
        });
        let svcs = service_endpoints_from_registry(&registry, "svc");
        assert!(svcs.is_empty());
    }

    #[test]
    fn endpoints_from_workspace_registry_returns_none_when_no_registry() {
        let tmp = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        let res = endpoints_from_workspace_registry("nope");
        std::env::set_current_dir(prev).unwrap();
        assert!(res.is_none());
    }
}
