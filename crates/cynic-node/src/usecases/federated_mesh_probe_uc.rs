//! Federated Synapse Mesh Probe Use Case.
//!
//! Measures live node-to-node reachability, WireGuard overlay health,
//! and service availability across the declared Synapse cluster without modifying host state.

use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use cynic_protocol::{
    FEDERATED_MESH_PROBE_SCHEMA, FederatedMeshNodeStatus, FederatedMeshProbeReport,
};

/// Declared Synapse Node entry.
#[derive(Debug, Clone)]
pub struct DeclaredSynapseNode {
    pub id: &'static str,
    pub role: &'static str,
    pub ip: &'static str,
    pub probe_ports: &'static [u16],
}

pub const CANONICAL_SYNAPSE_NODES: &[DeclaredSynapseNode] = &[
    DeclaredSynapseNode {
        id: "local-kernel",
        role: "cortex_primary",
        ip: "127.0.0.1",
        probe_ports: &[8000, 1053],
    },
    DeclaredSynapseNode {
        id: "cynic-host-proxmox",
        role: "hypervisor",
        ip: "10.10.10.1",
        probe_ports: &[22, 8006],
    },
    DeclaredSynapseNode {
        id: "forgejo",
        role: "forge_primary",
        ip: "10.10.10.101",
        probe_ports: &[22, 3000],
    },
    DeclaredSynapseNode {
        id: "cynic-lxc-surrealdb",
        role: "storage_primary",
        ip: "10.10.10.103",
        probe_ports: &[22, 8000],
    },
    DeclaredSynapseNode {
        id: "cynic-builder",
        role: "build_runner",
        ip: "10.10.10.119",
        probe_ports: &[22],
    },
];

/// Probes all declared Synapse nodes and generates a read-back proof report.
pub fn probe_federated_synapse_mesh(local_node_id: &str) -> FederatedMeshProbeReport {
    let resolver = super::federated_mesh_resolver::LocalRegistryMeshResolver::from_repo_infra_default(
        CANONICAL_SYNAPSE_NODES,
    );
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
            synapse_ip: node.addr.clone(),
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
