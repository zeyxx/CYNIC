//! Builder pool resolution — scale to N builders by role.
//!
//! WHY: CYNIC must support N build_runner nodes. This module resolves all
//! builders from infra/registry.json and selects one by health-check.
//! HOW: read registry, filter role == "build_runner", health-check each,
//! return the first healthy one. Pure selection logic, I/O in adapters.
//! NEGATIVE BOUNDARY: no SSH, no subprocess. HTTP only (builder_client).

use serde_json::Value;

/// A resolved builder endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuilderEndpoint {
    pub node_id: String,
    pub url: String,
    pub healthy: bool,
}

/// Resolve all builder endpoints from the registry.
///
/// Returns a list of (node_id, url) pairs for all nodes with role == "build_runner".
/// Does not perform health checks — caller must check.
pub fn resolve_builder_pool(registry: &Value) -> Vec<BuilderEndpoint> {
    let mut pool = Vec::new();
    if let Some(nodes) = registry.get("nodes").and_then(|n| n.as_array()) {
        for node in nodes {
            let role = node.get("role").and_then(|r| r.as_str()).unwrap_or_default();
            if role != "build_runner" {
                continue;
            }
            let node_id = node.get("id").and_then(|i| i.as_str()).unwrap_or_default().to_string();
            let ip = node.get("network").and_then(|network| {
                // Use plane_vocabulary if available, else fallback to service field
                network.get("service").and_then(|s| s.as_str()).map(String::from)
            });
            if let Some(ip) = ip {
                pool.push(BuilderEndpoint {
                    node_id,
                    url: format!("http://{ip}:9090"),
                    healthy: false,
                });
            }
        }
    }
    pool
}

/// Select the first healthy builder from the pool.
///
/// Returns None if no builder is healthy. Caller must handle the None case
/// (fail closed, never guess).
pub fn select_healthy_builder(pool: &[BuilderEndpoint]) -> Option<BuilderEndpoint> {
    pool.iter().find(|b| b.healthy).cloned()
}

/// Mark a builder as healthy or unhealthy based on a health-check result.
pub fn mark_health(endpoint: &mut BuilderEndpoint, healthy: bool) {
    endpoint.healthy = healthy;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolve_empty_registry() {
        let reg = json!({"nodes": []});
        let pool = resolve_builder_pool(&reg);
        assert!(pool.is_empty());
    }

    #[test]
    fn resolve_single_builder() {
        let reg = json!({
            "nodes": [
                {"id": "builder-1", "role": "build_runner", "network": {"service": "10.10.10.119"}},
                {"id": "cortex", "role": "cortex_host", "network": {"service": "10.10.10.100"}}
            ]
        });
        let pool = resolve_builder_pool(&reg);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool[0].node_id, "builder-1");
        assert_eq!(pool[0].url, "http://10.10.10.119:9090");
        assert!(!pool[0].healthy);
    }

    #[test]
    fn resolve_multiple_builders() {
        let reg = json!({
            "nodes": [
                {"id": "builder-1", "role": "build_runner", "network": {"service": "10.10.10.119"}},
                {"id": "builder-2", "role": "build_runner", "network": {"service": "10.10.10.120"}},
                {"id": "cortex", "role": "cortex_host", "network": {"service": "10.10.10.100"}}
            ]
        });
        let pool = resolve_builder_pool(&reg);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn select_healthy_none_when_all_unhealthy() {
        let pool = vec![
            BuilderEndpoint { node_id: "b1".to_string(), url: "http://a:9090".to_string(), healthy: false },
        ];
        assert!(select_healthy_builder(&pool).is_none());
    }

    #[test]
    fn select_healthy_picks_first_healthy() {
        let pool = vec![
            BuilderEndpoint { node_id: "b1".to_string(), url: "http://a:9090".to_string(), healthy: false },
            BuilderEndpoint { node_id: "b2".to_string(), url: "http://b:9090".to_string(), healthy: true },
        ];
        let selected = select_healthy_builder(&pool).unwrap();
        assert_eq!(selected.node_id, "b2");
    }

    #[test]
    fn mark_health_updates() {
        let mut ep = BuilderEndpoint { node_id: "b1".to_string(), url: "http://a:9090".to_string(), healthy: false };
        mark_health(&mut ep, true);
        assert!(ep.healthy);
    }
}