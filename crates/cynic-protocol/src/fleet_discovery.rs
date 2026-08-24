//! Fleet discovery — read-only, network-wide discovery of declared nodes.
//!
//! PURE types only. The adapter (in cynic-host) probes declared nodes and
//! produces a `FleetDiscoveryReport`. The CLI and MCP surfaces project from it.
//!
//! Source of truth: `infra/registry.json` (declared state). This module never
//! invents nodes — unregistered hosts are "unknown" (preserved, not fabricated).

use serde::{Deserialize, Serialize};

pub const FLEET_DISCOVERY_SCHEMA: &str = "cynic.fleet_discovery.v1";

/// One discovered node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetNodeDiscovery {
    pub node_id: String,
    pub role: String,
    /// Resolved IP (synapse plane preferred, then management).
    pub ip: String,
    /// "alive" | "unreachable" | "degraded"
    pub status: FleetNodeStatus,
    pub services: Vec<FleetServiceStatus>,
    pub last_seen_unix_s: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetNodeStatus {
    Alive,
    Unreachable,
    Degraded,
}

/// One service probe result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetServiceStatus {
    pub name: String,
    /// "healthy" | "unhealthy" | "unreachable"
    pub status: FleetServiceHealth,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetServiceHealth {
    Healthy,
    Unhealthy,
    Unreachable,
}

/// Machine-readable discovery report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetDiscoveryReport {
    pub schema: String,
    pub corpus_unix_s: u64,
    pub total_nodes: usize,
    pub alive: usize,
    pub degraded: usize,
    pub unreachable: usize,
    pub nodes: Vec<FleetNodeDiscovery>,
    /// What remains unknown (residual risk, never fabricated).
    pub residual_unknowns: Vec<String>,
}

/// Pure rule: classify a node from its service statuses.
pub fn classify_node(services: &[FleetServiceStatus]) -> FleetNodeStatus {
    if services.is_empty() {
        return FleetNodeStatus::Unreachable;
    }
    let all_healthy = services
        .iter()
        .all(|s| s.status == FleetServiceHealth::Healthy);
    let any_healthy = services
        .iter()
        .any(|s| s.status == FleetServiceHealth::Healthy);
    if all_healthy {
        FleetNodeStatus::Alive
    } else if any_healthy {
        FleetNodeStatus::Degraded
    } else {
        FleetNodeStatus::Unreachable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_all_healthy_is_alive() {
        let services = vec![
            FleetServiceStatus {
                name: "s1".into(),
                status: FleetServiceHealth::Healthy,
                latency_ms: 10,
            },
            FleetServiceStatus {
                name: "s2".into(),
                status: FleetServiceHealth::Healthy,
                latency_ms: 20,
            },
        ];
        assert_eq!(classify_node(&services), FleetNodeStatus::Alive);
    }

    #[test]
    fn classify_partial_healthy_is_degraded() {
        let services = vec![
            FleetServiceStatus {
                name: "s1".into(),
                status: FleetServiceHealth::Healthy,
                latency_ms: 10,
            },
            FleetServiceStatus {
                name: "s2".into(),
                status: FleetServiceHealth::Unhealthy,
                latency_ms: 0,
            },
        ];
        assert_eq!(classify_node(&services), FleetNodeStatus::Degraded);
    }

    #[test]
    fn classify_no_healthy_is_unreachable() {
        let services = vec![FleetServiceStatus {
            name: "s1".into(),
            status: FleetServiceHealth::Unhealthy,
            latency_ms: 0,
        }];
        assert_eq!(classify_node(&services), FleetNodeStatus::Unreachable);
    }

    #[test]
    fn classify_empty_services_is_unreachable() {
        assert_eq!(classify_node(&[]), FleetNodeStatus::Unreachable);
    }

    #[test]
    fn schema_is_stable() {
        assert_eq!(FLEET_DISCOVERY_SCHEMA, "cynic.fleet_discovery.v1");
    }
}
