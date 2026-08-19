//! Fleet discovery adapter — reads declared nodes from `infra/registry.json`
//! and probes them via TCP connect.
//!
//! Split per structural-invariant #1: types live in `cynic_protocol`, this
//! module is the adapter (I/O). The CLI/MCP surfaces project from it.

use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cynic_protocol::fleet_discovery::{
    classify_node, FleetDiscoveryReport, FleetNodeDiscovery, FleetNodeStatus,
    FleetServiceHealth, FleetServiceStatus, FLEET_DISCOVERY_SCHEMA,
};

/// Reads declared nodes from the registry and probes them.
pub struct FleetDiscoveryAdapter {
    registry_path: String,
    timeout_ms: u64,
}

impl FleetDiscoveryAdapter {
    pub fn new(registry_path: impl Into<String>) -> Self {
        Self {
            registry_path: registry_path.into(),
            timeout_ms: 3000,
        }
    }

    /// Run the full discovery: read registry → probe nodes → assemble report.
    pub fn discover(&self) -> FleetDiscoveryReport {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let declared = match self.read_declared_nodes() {
            Ok(nodes) => nodes,
            Err(e) => {
                return FleetDiscoveryReport {
                    schema: FLEET_DISCOVERY_SCHEMA.to_string(),
                    corpus_unix_s: now,
                    total_nodes: 0,
                    alive: 0,
                    degraded: 0,
                    unreachable: 0,
                    nodes: vec![],
                    residual_unknowns: vec![format!("registry_read_failed: {e}")],
                };
            }
        };

        let mut nodes = Vec::new();
        let mut residual_unknowns = Vec::new();
        let mut alive = 0;
        let mut degraded = 0;
        let mut unreachable = 0;

        for declared_node in declared {
            let ip = declared_node.synapse_ip.unwrap_or_else(|| declared_node.management_ip.clone());
            let services = probe_services(&ip, declared_node.services.as_slice(), self.timeout_ms);
            let status = classify_node(&services);

            match status {
                FleetNodeStatus::Alive => alive += 1,
                FleetNodeStatus::Degraded => degraded += 1,
                FleetNodeStatus::Unreachable => unreachable += 1,
            }

            nodes.push(FleetNodeDiscovery {
                node_id: declared_node.id,
                role: declared_node.role,
                ip,
                status,
                services,
                last_seen_unix_s: now,
            });
        }

        if !residual_unknowns.is_empty() {
            // no-op for now, reserved for future unknowns
        }

        FleetDiscoveryReport {
            schema: FLEET_DISCOVERY_SCHEMA.to_string(),
            corpus_unix_s: now,
            total_nodes: nodes.len(),
            alive,
            degraded,
            unreachable,
            nodes,
            residual_unknowns,
        }
    }

    fn read_declared_nodes(&self) -> Result<Vec<DeclaredNode>, String> {
        let bytes = std::fs::read(&self.registry_path)
            .map_err(|e| format!("read {}: {}", self.registry_path, e))?;
        parse_declared_nodes(&bytes)
    }
}

/// One declared node (parsed from registry JSON).
struct DeclaredNode {
    id: String,
    role: String,
    synapse_ip: Option<String>,
    management_ip: String,
    services: Vec<DeclaredService>,
}

struct DeclaredService {
    name: String,
    port: u16,
}

/// Parse declared nodes from registry JSON.
fn parse_declared_nodes(bytes: &[u8]) -> Result<Vec<DeclaredNode>, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| format!("parse registry JSON: {e}"))?;

    let mut nodes = Vec::new();

    // Parse from network_policy.reservations + nodes
    if let Some(reservations) = value.get("network_policy").and_then(|v| v.get("reservations")).and_then(|v| v.as_array()) {
        for reservation in reservations {
            let name = reservation.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let synapse_ip = reservation.get("synapse").and_then(|v| v.as_str()).map(String::from);
            let management_ip = reservation.get("management").and_then(|v| v.as_str()).map(String::from);

            if let Some(management) = management_ip {
                // Find matching node for role and services
                let (role, services) = find_node_details(&value, &name);
                nodes.push(DeclaredNode {
                    id: name.clone(),
                    role,
                    synapse_ip,
                    management_ip: management,
                    services,
                });
            } else if let Some(synapse) = synapse_ip {
                let (role, services) = find_node_details(&value, &name);
                nodes.push(DeclaredNode {
                    id: name.clone(),
                    role,
                    synapse_ip: Some(synapse),
                    management_ip: synapse,
                    services,
                });
            }
        }
    }

    Ok(nodes)
}

fn find_node_details(value: &serde_json::Value, name: &str) -> (String, Vec<DeclaredService>) {
    if let Some(nodes) = value.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if node_id == name {
                let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                let services = node.get("services").and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| {
                                let name = s.get("name").and_then(|v| v.as_str())?.to_string();
                                let port = s.get("port").and_then(|v| v.as_u64())? as u16;
                                Some(DeclaredService { name, port })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return (role, services);
            }
        }
    }
    ("unknown".to_string(), vec![])
}

/// Probe a service via TCP connect.
fn probe_service(ip: &str, port: u16, timeout_ms: u64) -> FleetServiceHealth {
    let addr = format!("{ip}:{port}");
    let start = Instant::now();
    match TcpStream::connect_timeout(
        &addr.parse().unwrap_or_else(|| "0.0.0.0:0".parse().unwrap()),
        Duration::from_millis(timeout_ms),
    ) {
        Ok(mut stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(timeout_ms)));
            let mut buf = [0u8; 1];
            let _ = stream.read(&mut buf);
            FleetServiceHealth::Healthy
        }
        Err(_) => FleetServiceHealth::Unhealthy,
    }
}

fn probe_services(ip: &str, services: &[DeclaredService], timeout_ms: u64) -> Vec<FleetServiceStatus> {
    services
        .iter()
        .map(|svc| {
            let start = Instant::now();
            let status = probe_service(ip, svc.port, timeout_ms);
            let latency_ms = start.elapsed().as_millis() as u64;
            FleetServiceStatus {
                name: svc.name.clone(),
                status,
                latency_ms,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_declared_nodes_from_minimal_json() {
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
        let nodes = parse_declared_nodes(json.as_bytes()).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, "test-node");
        assert_eq!(nodes[0].role, "test");
        assert_eq!(nodes[0].services.len(), 1);
    }

    #[test]
    fn parse_declared_nodes_missing_registry_is_error() {
        let result = parse_declared_nodes(b"not json");
        assert!(result.is_err());
    }
}
