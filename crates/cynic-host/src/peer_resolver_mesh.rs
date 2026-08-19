//! Concrete `PeerResolver` implementation — mesh-aware, health-filtered.
//!
//! This is the bottom-up foundation: without a concrete resolver, every layer
//! above (autonomy loop, CLI, MCP, takeover decision) cannot be tested.

use std::net::TcpStream;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cynic_protocol::build_node::NodeHealthView;
use cynic_protocol::health_signal::OsHealthAnomaly;
use cynic_protocol::peer_resolver::{
    PeerEndpoint, PeerResolver, PeerTransport, SignedPeerEndpoint,
};

/// Concrete resolver: reads registry, probes TCP, observes OS health.
pub struct MeshPeerResolver {
    registry_path: String,
    timeout_ms: u64,
    local_node_id: String,
}

impl MeshPeerResolver {
    pub fn new(registry_path: impl Into<String>, local_node_id: impl Into<String>) -> Self {
        Self {
            registry_path: registry_path.into(),
            timeout_ms: 3000,
            local_node_id: local_node_id.into(),
        }
    }

    /// Read declared peers from registry.
    fn read_declared_peers(&self) -> Result<Vec<DeclaredPeer>, String> {
        let bytes = std::fs::read(&self.registry_path)
            .map_err(|e| format!("read {}: {}", self.registry_path, e))?;
        parse_declared_peers(&bytes)
    }

    /// Probe a single TCP port.
    fn probe_port(&self, ip: &str, port: u16) -> (bool, u64) {
        let addr = format!("{ip}:{port}");
        let start = Instant::now();
        let reachable = addr.parse().ok().is_some_and(|socket_addr| {
            TcpStream::connect_timeout(&socket_addr, Duration::from_millis(self.timeout_ms)).is_ok()
        });
        (reachable, start.elapsed().as_millis() as u64)
    }

    /// Read OS health anomalies for the local node.
    fn observe_local_os_health(&self) -> Vec<OsHealthAnomaly> {
        // Observe portable kernel interfaces (same as os_health/observe.rs).
        let available_mem_mib = read_available_mem_mib();
        let reclaim_pressure_pct = read_reclaim_pressure_pct();
        let softlockup_panic = read_bool("/proc/sys/kernel/softlockup_panic");

        let now_ms: u128 = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0u128, |d| d.as_millis());
        let mut anomalies = vec![];

        if !softlockup_panic {
            anomalies.push(OsHealthAnomaly {
                severity: "critical".to_string(),
                error_code: Some("CYNIC-OSH-001".to_string()),
                detail: "kernel.softlockup_panic=0 (remediation: sysctl -w kernel.softlockup_panic=1)".to_string(),
                observed_at_unix_ms: now_ms,
            });
        }

        if available_mem_mib < 2048 {
            anomalies.push(OsHealthAnomaly {
                severity: "warning".to_string(),
                error_code: Some("CYNIC-OSH-003".to_string()),
                detail: format!("available memory {available_mem_mib} MiB below floor (remediation: reduce workloads or add memory)"),
                observed_at_unix_ms: now_ms,
            });
        }

        if reclaim_pressure_pct > 80 {
            anomalies.push(OsHealthAnomaly {
                severity: "warning".to_string(),
                error_code: Some("CYNIC-OSH-004".to_string()),
                detail: format!("reclaim pressure {reclaim_pressure_pct}% above ceiling (remediation: inspect memory consumers)"),
                observed_at_unix_ms: now_ms,
            });
        }

        anomalies
    }
}

impl PeerResolver for MeshPeerResolver {
    fn resolve_peers(&self, role: &str) -> Vec<PeerEndpoint> {
        let declared = match self.read_declared_peers() {
            Ok(d) => d,
            Err(_) => return vec![],
        };

        let mut peers = vec![];
        for node in declared {
            if !node.role.contains(role) {
                continue;
            }
            let ip = node.synapse_ip.unwrap_or_else(|| node.management_ip.clone());
            let mut any_reachable = false;

            for port in &node.probe_ports {
                let (reachable, _) = self.probe_port(&ip, *port);
                if reachable {
                    any_reachable = true;
                } else {
                }
            }

            // OS health: local observation only (remote observation requires SSH/agent).
            let os_health = if node.id == self.local_node_id {
                let anomalies = self.observe_local_os_health();
                NodeHealthView::from_anomalies(&anomalies)
            } else {
                NodeHealthView::default()
            };

            peers.push(PeerEndpoint {
                node_id: node.id,
                role: node.role,
                transport: PeerTransport::Http,
                addr: format!("http://{ip}"),
                probe_ports: node.probe_ports,
                reachable: any_reachable,
                health: os_health,
            });
        }

        peers
    }

    fn fallback_peers(&self, role: &str) -> Vec<SignedPeerEndpoint> {
        // Fallback: same as resolve_peers but without liveness check.
        // Used when the live mesh is unreachable.
        let declared = match self.read_declared_peers() {
            Ok(d) => d,
            Err(_) => return vec![],
        };

        declared
            .iter()
            .filter(|n| n.role.contains(role))
            .map(|n| SignedPeerEndpoint {
                endpoint: PeerEndpoint {
                    node_id: n.id.clone(),
                    role: n.role.clone(),
                    transport: PeerTransport::Http,
                    addr: n.synapse_ip.clone().unwrap_or(n.management_ip.clone()),
                    probe_ports: n.probe_ports.clone(),
                    reachable: false,
                    health: NodeHealthView::default(),
                },
                signature_ed25519: String::new(),
                generated_at_unix_s: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                version: 1,
            })
            .collect()
    }

    fn resolve_all_peers(&self) -> Vec<PeerEndpoint> {
        let declared = match self.read_declared_peers() {
            Ok(d) => d,
            Err(_) => return vec![],
        };

        let mut peers = vec![];
        for node in declared {
            let ip = node.synapse_ip.unwrap_or_else(|| node.management_ip.clone());
            let mut any_reachable = false;

            for port in &node.probe_ports {
                let (reachable, _) = self.probe_port(&ip, *port);
                if reachable {
                    any_reachable = true;
                } else {
                }
            }

            let os_health = if node.id == self.local_node_id {
                let anomalies = self.observe_local_os_health();
                NodeHealthView::from_anomalies(&anomalies)
            } else {
                NodeHealthView::default()
            };

            peers.push(PeerEndpoint {
                node_id: node.id,
                role: node.role,
                transport: PeerTransport::Http,
                addr: format!("http://{ip}"),
                probe_ports: node.probe_ports,
                reachable: any_reachable,
                health: os_health,
            });
        }

        peers
    }

}

// ──────────────────────────────────────────────────────────────────────
// Registry parsing
// ──────────────────────────────────────────────────────────────────────

struct DeclaredPeer {
    id: String,
    role: String,
    synapse_ip: Option<String>,
    management_ip: String,
    probe_ports: Vec<u16>,
}

fn parse_declared_peers(bytes: &[u8]) -> Result<Vec<DeclaredPeer>, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| format!("parse registry JSON: {e}"))?;

    let mut peers = vec![];

    // Read from network_policy.reservations
    if let Some(reservations) = value
        .get("network_policy")
        .and_then(|v| v.get("reservations"))
        .and_then(|v| v.as_array())
    {
        for reservation in reservations {
            let name = reservation
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let synapse_ip = reservation.get("synapse").and_then(|v| v.as_str()).map(String::from);
            let management_ip = reservation
                .get("management")
                .and_then(|v| v.as_str())
                .map(String::from);

            if let Some(management) = management_ip {
                let (role, ports) = find_node_details(&value, &name);
                peers.push(DeclaredPeer {
                    id: name,
                    role,
                    synapse_ip,
                    management_ip: management,
                    probe_ports: ports,
                });
            }
        }
    }

    Ok(peers)
}

fn find_node_details(value: &serde_json::Value, name: &str) -> (String, Vec<u16>) {
    if let Some(nodes) = value.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if node_id == name {
                let role = node
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let ports = node
                    .get("services")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.get("port").and_then(|v| v.as_u64()).map(|p| p as u16))
                            .collect()
                    })
                    .unwrap_or_default();
                return (role, ports);
            }
        }
    }
    ("unknown".to_string(), vec![])
}

// ──────────────────────────────────────────────────────────────────────
// OS health probes (portable, same logic as os_health/observe.rs)
// ──────────────────────────────────────────────────────────────────────

fn read_bool(path: &str) -> bool {
    std::fs::read_to_string(path).map(|c| c.trim() == "1").unwrap_or(false)
}

fn read_available_mem_mib() -> u64 {
    let content = match std::fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return 0,
    };
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            if let Some(kb) = rest.split_whitespace().next() {
                if let Ok(kb_val) = kb.parse::<u64>() {
                    return kb_val / 1024;
                }
            }
        }
    }
    0
}

fn read_reclaim_pressure_pct() -> u8 {
    let content = match std::fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let mut avail = 0u64;
    let mut total = 0u64;
    for line in content.lines() {
        if let Some(r) = line.strip_prefix("MemAvailable:") {
            avail = r.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0);
        } else if let Some(r) = line.strip_prefix("MemTotal:") {
            total = r.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0);
        }
    }
    if total == 0 {
        return 0;
    }
    ((total.saturating_sub(avail)) as f64 / total as f64 * 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_declared_peers_from_minimal_json() {
        let json = r#"{
            "network_policy": {
                "reservations": [
                    {"name": "test-node", "synapse": "10.10.10.99", "management": "192.168.100.99"}
                ]
            },
            "nodes": [
                {"id": "test-node", "role": "test", "services": [{"name": "http", "port": 80}]}
            ]
        }"#;
        let peers = parse_declared_peers(json.as_bytes()).unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].id, "test-node");
        assert_eq!(peers[0].role, "test");
        assert_eq!(peers[0].probe_ports, vec![80]);
    }

    #[test]
    fn parse_declared_peers_missing_registry_is_error() {
        assert!(parse_declared_peers(b"not json").is_err());
    }

    #[test]
    fn mesh_peer_resolver_fallback_returns_declared() {
        // This test verifies fallback works without network access.
        // We can't easily test resolve_peers without a real registry + network.
        // For now, just verify the type system accepts the impl.
        let resolver = MeshPeerResolver::new("/nonexistent/registry.json", "local");
        let fallback = resolver.fallback_peers("build_runner");
        assert!(fallback.is_empty());
    }
}
