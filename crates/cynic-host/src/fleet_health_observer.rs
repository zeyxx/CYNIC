//! Fleet health observer — aggregates heartbeats into a fleet-wide view.
//!
//! Uses the existing `FleetDiscoveryAdapter` to read declared nodes,
//! the `InMemoryHeartbeatStore` to collect heartbeats, and the pure
//! `aggregate_fleet_health` decision to produce a health aggregation.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::fleet_heartbeat::{
    aggregate_fleet_health, FleetHealthAggregation, FleetHeartbeat, FleetNodeStatus,
};

use crate::fleet_discovery::FleetDiscoveryAdapter;

/// Fleet health observer — the runtime wiring.
pub struct FleetHealthObserver {
    pub(crate) registry_path: String,
    pub(crate) heartbeat_timeout_s: u64,
    pub(crate) heartbeats: Vec<FleetHeartbeat>,
}

impl FleetHealthObserver {
    pub fn new(registry_path: impl Into<String>, heartbeat_timeout_s: u64) -> Self {
        Self {
            registry_path: registry_path.into(),
            heartbeat_timeout_s,
            heartbeats: Vec::new(),
        }
    }

    /// Load declared nodes from registry and create synthetic heartbeats
    /// from the one-shot discovery (for environments without a real
    /// heartbeat stream yet).
    pub fn observe(&self) -> FleetHealthAggregation {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let adapter = FleetDiscoveryAdapter::new(&self.registry_path);
        let report = adapter.discover();

        let declared_node_ids: Vec<String> = report.nodes.iter().map(|n| n.node_id.clone()).collect();

        // Convert discovery results into synthetic heartbeats.
        let heartbeats: Vec<FleetHeartbeat> = report
            .nodes
            .iter()
            .map(|n| FleetHeartbeat {
                node_id: n.node_id.clone(),
                emitted_at_unix_s: n.last_seen_unix_s,
                self_status: n.status,
                uptime_s: 0,
                unknowns: vec![],
            })
            .collect();

        aggregate_fleet_health(
            &declared_node_ids,
            &heartbeats,
            now,
            self.heartbeat_timeout_s,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_returns_aggregation() {
        let observer = FleetHealthObserver::new("infra/registry.json", 30);
        let agg = observer.observe();
        assert_eq!(agg.schema, "cynic.fleet_health_aggregation.v1");
        // Registry has 7 nodes — all should be declared.
        assert_eq!(agg.total_declared, 7);
    }
}
