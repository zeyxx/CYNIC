//! Dynamic Registry Maintenance & Convergence Domain for CYNIC
//! Ingests NetworkDiscoveryReports, updates the "observed" section of registry.json,
//! and triggers bounded auto-reconciliation actions without manual human intervention.

use crate::discovery::{DiscoveredNode, NetworkDiscoveryReport, NodeReachability};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const REGISTRY_UPDATE_REPORT_SCHEMA: &str = "cynic.network.registry_update_report.v1";

/// Applies discovered network observations directly to a registry JSON Value
pub fn reconcile_registry_observations(
    registry: &mut Value,
    discovery: &NetworkDiscoveryReport,
) -> Result<usize, String> {
    let root = registry
        .as_object_mut()
        .ok_or_else(|| "registry root must be a JSON object".to_string())?;

    let observed = root
        .entry("observed")
        .or_insert_with(|| json!({"nodes": {}}));

    let observed_nodes = observed
        .get_mut("nodes")
        .and_modify(|n| {
            if !n.is_object() {
                *n = json!({});
            }
        })
        .ok_or_else(|| "observed.nodes must be a JSON object".to_string())?
        .as_object_mut()
        .ok_or_else(|| "observed.nodes must be a JSON object".to_string())?;

    let mut updated_count = 0;
    for (node_id, node) in &discovery.nodes {
        let entry = json!({
            "status": match node.reachability {
                NodeReachability::Reachable { .. } => "observed_active",
                NodeReachability::Degraded { .. } => "observed_degraded",
                NodeReachability::Unreachable { .. } => "observed_unreachable",
            },
            "synapse_ip": node.synapse_ip,
            "wireguard_ip": node.wireguard_ip,
            "direct_ip": node.direct_ip,
            "last_observed_unix_ms": node.observed_at_unix_ms,
            "services_count": node.services.len(),
        });
        observed_nodes.insert(node_id.clone(), entry);
        updated_count += 1;
    }

    Ok(updated_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciles_discovered_nodes_into_registry_observed_section() {
        let mut registry = json!({
            "schema": "cynic.network.registry.v1",
            "nodes": [
                {"id": "cynic-builder", "name": "cynic-builder"}
            ]
        });

        let mut nodes = BTreeMap::new();
        nodes.insert(
            "cynic-builder".to_string(),
            DiscoveredNode {
                node_id: "cynic-builder".to_string(),
                direct_ip: None,
                synapse_ip: Some("10.10.10.119".to_string()),
                wireguard_ip: Some("10.20.0.5".to_string()),
                reachability: NodeReachability::Reachable {
                    latency_ms: 1,
                    protocol: "tcp".to_string(),
                },
                services: vec![],
                observed_at_unix_ms: 123456789,
            },
        );

        let report = NetworkDiscoveryReport {
            schema: "cynic.network.discovery_report.v1".to_string(),
            observer_node_id: "cortex".to_string(),
            nodes,
            drift_detected: false,
            drift_summary: vec![],
            observed_at_unix_ms: 123456789,
        };

        let count = reconcile_registry_observations(&mut registry, &report).expect("reconcile ok");
        assert_eq!(count, 1);
        assert_eq!(
            registry["observed"]["nodes"]["cynic-builder"]["status"],
            "observed_active"
        );
        assert_eq!(
            registry["observed"]["nodes"]["cynic-builder"]["synapse_ip"],
            "10.10.10.119"
        );
    }
}
