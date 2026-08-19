//! Fleet heartbeat & health aggregation — the living layer of the network.
//!
//! Existing code covers one-shot discovery (fleet_discovery.rs) and policy
//! shapes (network_actuation.rs). This module adds the continuous pulse:
//! each node emits heartbeats, the aggregator assembles a fleet-wide health
//! view, and partition detection emerges from the gaps.
//!
//! Pattern: desired state (registry) → observe (heartbeats) → reconcile
//! (aggregate health) → bounded action (alert/failover) → read-back proof.

use serde::{Deserialize, Serialize};

pub use crate::fleet_discovery::FleetNodeStatus;

/// A single heartbeat emitted by a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetHeartbeat {
    pub node_id: String,
    pub emitted_at_unix_s: u64,
    /// Node's self-assessed status.
    pub self_status: FleetNodeStatus,
    /// Node's uptime in seconds.
    pub uptime_s: u64,
    /// Residual unknowns the node could not observe.
    pub unknowns: Vec<String>,
}

/// Aggregated health view for the whole fleet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetHealthAggregation {
    pub schema: &'static str,
    pub aggregated_at_unix_s: u64,
    pub total_declared: usize,
    pub total_observed: usize,
    pub alive: usize,
    pub degraded: usize,
    pub unreachable: usize,
    /// Nodes that declared themselves but sent no heartbeat within the window.
    pub silent: Vec<String>,
    /// Nodes that sent a heartbeat but are not in the registry.
    pub unexpected: Vec<String>,
    /// Per-node latest heartbeat.
    pub heartbeats: Vec<FleetHeartbeat>,
    /// Partition detection: are there unreachable segments?
    pub partition_detected: bool,
    pub residual_unknowns: Vec<String>,
}

/// Pure decision: aggregate heartbeats into a fleet health view.
pub fn aggregate_fleet_health(
    declared_node_ids: &[String],
    heartbeats: &[FleetHeartbeat],
    now_unix_s: u64,
    heartbeat_timeout_s: u64,
) -> FleetHealthAggregation {
    let mut alive = 0usize;
    let mut degraded = 0usize;
    let mut unreachable = 0usize;
    let mut silent = Vec::new();
    let mut unexpected = Vec::new();
    let mut recent_heartbeats = Vec::new();

    // Check each declared node for a recent heartbeat.
    for node_id in declared_node_ids {
        let latest = heartbeats
            .iter()
            .filter(|h| &h.node_id == node_id)
            .max_by_key(|h| h.emitted_at_unix_s);

        match latest {
            Some(hb) if now_unix_s - hb.emitted_at_unix_s <= heartbeat_timeout_s => {
                recent_heartbeats.push(hb.clone());
                match hb.self_status {
                    FleetNodeStatus::Alive => alive += 1,
                    FleetNodeStatus::Degraded => degraded += 1,
                    FleetNodeStatus::Unreachable => unreachable += 1,
                }
            }
            _ => {
                silent.push(node_id.clone());
                unreachable += 1;
            }
        }
    }

    // Detect heartbeats from nodes not in the registry.
    for hb in heartbeats {
        if !declared_node_ids.contains(&hb.node_id) {
            unexpected.push(hb.node_id.clone());
        }
    }

    // Partition detection: if >0 declared nodes are silent AND at least one
    // node is alive, we may have a network partition.
    let partition_detected = !silent.is_empty() && alive > 0;

    FleetHealthAggregation {
        schema: "cynic.fleet_health_aggregation.v1",
        aggregated_at_unix_s: now_unix_s,
        total_declared: declared_node_ids.len(),
        total_observed: recent_heartbeats.len(),
        alive,
        degraded,
        unreachable,
        silent,
        unexpected,
        heartbeats: recent_heartbeats,
        partition_detected,
        residual_unknowns: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, status: FleetNodeStatus, at: u64) -> FleetHeartbeat {
        FleetHeartbeat {
            node_id: id.into(),
            emitted_at_unix_s: at,
            self_status: status,
            uptime_s: 100,
            unknowns: vec![],
        }
    }

    #[test]
    fn all_healthy() {
        let declared = vec!["a".into(), "b".into()];
        let heartbeats = vec![
            node("a", FleetNodeStatus::Alive, 1000),
            node("b", FleetNodeStatus::Alive, 1000),
        ];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.alive, 2);
        assert_eq!(agg.unreachable, 0);
        assert!(!agg.partition_detected);
    }

    #[test]
    fn silent_node_detected() {
        let declared = vec!["a".into(), "b".into()];
        let heartbeats = vec![node("a", FleetNodeStatus::Alive, 1000)];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.alive, 1);
        assert_eq!(agg.silent, vec!["b"]);
        assert!(agg.partition_detected);
    }

    #[test]
    fn stale_heartbeat_treated_as_silent() {
        let declared = vec!["a".into()];
        let heartbeats = vec![node("a", FleetNodeStatus::Alive, 900)];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.silent, vec!["a"]);
        assert!(agg.partition_detected);
    }

    #[test]
    fn unexpected_node_flagged() {
        let declared = vec!["a".into()];
        let heartbeats = vec![
            node("a", FleetNodeStatus::Alive, 1000),
            node("rogue", FleetNodeStatus::Alive, 1000),
        ];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.unexpected, vec!["rogue"]);
    }

    #[test]
    fn degraded_counts_separately() {
        let declared = vec!["a".into(), "b".into()];
        let heartbeats = vec![
            node("a", FleetNodeStatus::Alive, 1000),
            node("b", FleetNodeStatus::Degraded, 1000),
        ];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.alive, 1);
        assert_eq!(agg.degraded, 1);
        assert!(!agg.partition_detected);
    }

    #[test]
    fn empty_fleet_is_unknown() {
        let agg = aggregate_fleet_health(&[], &[], 1000, 30);
        assert_eq!(agg.total_declared, 0);
        assert!(!agg.partition_detected);
    }
}
