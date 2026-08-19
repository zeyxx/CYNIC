//! Fleet health observer — aggregates heartbeats into a fleet-wide view.
//!
//! Uses the existing `FleetDiscoveryAdapter` to read declared nodes,
//! the `InMemoryHeartbeatStore` to collect heartbeats, and the pure
//! `aggregate_fleet_health` decision to produce a health aggregation.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::fleet_heartbeat::{
    FleetHealthAggregation, FleetHeartbeat, aggregate_fleet_health,
};

use crate::fleet_discovery::FleetDiscoveryAdapter;

/// Fleet health observer — the runtime wiring.
pub struct FleetHealthObserver {
    pub(crate) registry_path: String,
    pub(crate) heartbeat_timeout_s: u64,
    #[allow(dead_code)]
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

        let declared_node_ids: Vec<String> =
            report.nodes.iter().map(|n| n.node_id.clone()).collect();

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
        // An explicit fixture, not the relative "infra/registry.json": cargo runs
        // unit tests with the crate directory as CWD, so that path never resolves
        // and the aggregation silently reported 0 declared nodes.
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cynic-fleet-health-{unique}.json"));
        std::fs::write(
            &path,
            r#"{
              "nodes": [
                {"id":"node-a","role":"kernel_primary"},
                {"id":"node-b","role":"build_runner"},
                {"id":"node-c","role":"storage_primary"}
              ]
            }"#,
        )
        .expect("write fixture");

        let observer = FleetHealthObserver::new(path.to_str().expect("utf8 temp path"), 30);
        let agg = observer.observe();
        assert_eq!(agg.schema, "cynic.fleet_health_aggregation.v1");
        assert_eq!(
            agg.total_declared, 3,
            "every node declared in the fixture must be counted"
        );
    }

    #[test]
    fn unreadable_registry_declares_nothing_rather_than_inventing_nodes() {
        let observer = FleetHealthObserver::new("/nonexistent/cynic-registry.json", 30);
        let agg = observer.observe();
        assert_eq!(
            agg.total_declared, 0,
            "an unreadable registry must not fabricate declared nodes"
        );
    }
}
