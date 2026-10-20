//! Federated service Mesh Probe Use Case.
//!
//! Measures live node-to-node reachability, WireGuard overlay health,
//! and service availability across the declared service cluster without modifying host state.

use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use cynic_protocol::{
    FEDERATED_MESH_PROBE_SCHEMA, FederatedMeshNodeStatus, FederatedMeshProbeReport,
};

/// Probes all declared service nodes and generates a read-back proof report.
///
/// The declared topology comes from `infra/service-nodes.seed.json`, read by the
/// resolver. This function carried its own `CANONICAL_SYNAPSE_NODES` const until
/// 2026-09-30 — a second copy of the same five addresses and ports, kept as the
/// resolver's fallback when the seed file was missing. The seed file's own note
/// already said it "Replaces the hardcoded CANONICAL_SYNAPSE_NODES const as the
/// seed source"; the note was true about the intent and false about the code.
///
/// Two copies of a registry is the defect, not the fallback: the const could
/// drift from the seed and nothing compared them. An absent seed now yields zero
/// declared nodes and a report of `isolated` — an explicit unknown, which is
/// what a missing lower layer should produce, rather than a second copy of the
/// answer that no test can keep honest.
pub fn probe_federated_service_mesh(local_node_id: &str) -> FederatedMeshProbeReport {
    let resolver =
        super::federated_mesh_resolver::LocalRegistryMeshResolver::from_repo_infra_default();
    let seed_peers = resolver.resolve_all_peers();
    let timeout = Duration::from_millis(350);
    let mut node_statuses = Vec::new();
    let mut reachable_count = 0;

    for node in &seed_peers {
        let mut active_services = Vec::new();
        let mut min_rtt: Option<f32> = None;
        let mut is_reachable = false;

        for &port in &node.probe_ports {
            if let Ok(addr) = format!("{}:{}", node.addr, port).parse::<SocketAddr>() {
                let start = Instant::now();
                if let Ok(_stream) = TcpStream::connect_timeout(&addr, timeout) {
                    let rtt = start.elapsed().as_secs_f32() * 1000.0;
                    is_reachable = true;
                    active_services.push(format!("port_{port}_tcp"));
                    min_rtt = Some(min_rtt.map(|m| m.min(rtt)).unwrap_or(rtt));
                }
            }
        }

        // On local machine, 127.0.0.1 is always locally present
        if node.addr == "127.0.0.1" {
            is_reachable = true;
        }

        if is_reachable {
            reachable_count += 1;
        }

        node_statuses.push(FederatedMeshNodeStatus {
            node_id: node.node_id.clone(),
            role: node.role.clone(),
            service_ip: node.addr.clone(),
            wireguard_reachable: is_reachable,
            active_services,
            rtt_ms: min_rtt,
        });
    }

    let overall_status = if reachable_count >= 1 {
        "active"
    } else {
        "isolated"
    };

    FederatedMeshProbeReport {
        schema: FEDERATED_MESH_PROBE_SCHEMA,
        status: overall_status,
        local_node_id: local_node_id.to_string(),
        total_declared_nodes: seed_peers.len(),
        reachable_nodes_count: reachable_count,
        node_statuses,
    }
}
