//! Dynamic Peer & Service Discovery Domain for CYNIC Network
//! Follows CYNIC Coherence Pattern: Pure Observation Model -> Drift Detection -> Sealed Readback Proof.

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
                NodeReachability::Reachable { .. } => {}
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
}
