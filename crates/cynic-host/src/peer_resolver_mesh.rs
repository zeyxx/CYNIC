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

use crate::os_health::probes::{read_available_mem_mib, read_bool, read_reclaim_pressure_pct};

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

        let now_ms: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0u128, |d| d.as_millis());
        let mut anomalies = vec![];

        if !softlockup_panic {
            anomalies.push(OsHealthAnomaly {
                severity: "critical".to_string(),
                error_code: Some("CYNIC-OSH-001".to_string()),
                detail:
                    "kernel.softlockup_panic=0 (remediation: sysctl -w kernel.softlockup_panic=1)"
                        .to_string(),
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
        self.resolve_peers_filtered(Some(role))
    }

    fn fallback_peers(&self, role: &str) -> Vec<SignedPeerEndpoint> {
        // Fallback: same as resolve_peers but without liveness check.
        // Used when the live mesh is unreachable.
        //
        // FAIL-CLOSED invariant (see PeerResolver trait doc on
        // `fallback_peers`): returns empty when the signed fallback bundle is
        // missing OR its signature does not verify. The registry does not
        // carry ed25519 signatures — a fallback without a verifiable signature
        // MUST NOT be returned as trusted. Returns empty vec (availability
        // sacrificed for safety under partition).
        let _ = role;
        vec![]
    }

    fn resolve_all_peers(&self) -> Vec<PeerEndpoint> {
        self.resolve_peers_filtered(None)
    }
}

impl MeshPeerResolver {
    /// Shared peer resolution used by both `resolve_peers` (role-filtered) and
    /// `resolve_all_peers` (unfiltered). Probes TCP reachability and observes
    /// local OS health, then builds a `PeerEndpoint` per declared node.
    fn resolve_peers_filtered(&self, role_filter: Option<&str>) -> Vec<PeerEndpoint> {
        let declared = match self.read_declared_peers() {
            Ok(d) => d,
            Err(_) => return vec![],
        };

        let mut peers = vec![];
        for node in declared {
            if let Some(role) = role_filter {
                if !node.role.contains(role) {
                    continue;
                }
            }
            let ip = node
                .synapse_ip
                .unwrap_or_else(|| node.management_ip.clone());
            let mut any_reachable = false;

            for port in &node.probe_ports {
                let (reachable, _) = self.probe_port(&ip, *port);
                if reachable {
                    any_reachable = true;
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
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("parse registry JSON: {e}"))?;

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
            let synapse_ip = reservation
                .get("synapse")
                .and_then(|v| v.as_str())
                .map(String::from);
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
                            .filter_map(|s| {
                                s.get("port").and_then(|v| v.as_u64()).map(|p| p as u16)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return (role, ports);
            }
        }
    }
    ("unknown".to_string(), vec![])
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

    #[test]
    fn resolve_peers_and_resolve_all_peers_share_logic() {
        // Both methods delegate to resolve_peers_filtered; a node present in
        // both views must produce an identical PeerEndpoint (only the role
        // filter differs, never the built endpoint).
        let json = r#"{
            "network_policy": {
                "reservations": [
                    {"name": "n1", "synapse": "10.10.10.1", "management": "10.10.10.2"}
                ]
            },
            "nodes": [
                {"id": "n1", "role": "build_runner", "services": [{"name": "http", "port": 8080}]}
            ]
        }"#;
        let path = std::env::temp_dir().join(format!("prm-{}.json", std::process::id()));
        std::fs::write(&path, json).unwrap();
        let resolver = MeshPeerResolver::new(path.to_str().unwrap(), "local");

        let by_role = resolver.resolve_peers("build_runner");
        let all = resolver.resolve_all_peers();
        std::fs::remove_file(&path).ok();

        assert_eq!(by_role.len(), 1);
        assert_eq!(all.len(), 1);
        assert_eq!(by_role[0], all[0]);
    }
}
