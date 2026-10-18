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
            let role = node
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or_default();
            if role != "build_runner" {
                continue;
            }
            let node_id = node
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or_default()
                .to_string();
            let ip = node.get("network").and_then(|network| {
                // Use plane_vocabulary if available, else fallback to service field
                network
                    .get("service")
                    .and_then(|s| s.as_str())
                    .map(String::from)
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

/// Resolve all builder endpoints from the registry and health-check each one.
///
/// Calls `health_check` on each builder URL. The `healthy` field is set
/// according to the HTTP response (true = 200 OK, false = anything else).
///
/// This is the production entry point — prefer this over `resolve_builder_pool`
/// when the caller needs to select a builder to dispatch work to.
pub fn resolve_builder_pool_with_health(registry: &Value) -> Vec<BuilderEndpoint> {
    let mut pool = resolve_builder_pool(registry);
    for endpoint in &mut pool {
        endpoint.healthy = health_check(&endpoint.url);
    }
    pool
}

/// HTTP health-check for a single builder endpoint.
///
/// Issues `GET {url}/health` with a 5-second timeout via a blocking
/// `reqwest::Client`. Returns `true` iff the server responds with
/// HTTP 200 OK. Any error (connection refused, timeout, non-2xx status,
/// invalid URL) returns `false` — the builder is considered unhealthy.
pub fn health_check(url: &str) -> bool {
    let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    else {
        return false;
    };
    let health_url = format!("{}/health", url.trim_end_matches('/'));
    client
        .get(&health_url)
        .send()
        .map(|resp| resp.status().is_success())
        .unwrap_or(false)
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
        let pool = vec![BuilderEndpoint {
            node_id: "b1".to_string(),
            url: "http://a:9090".to_string(),
            healthy: false,
        }];
        assert!(select_healthy_builder(&pool).is_none());
    }

    #[test]
    fn select_healthy_picks_first_healthy() {
        let pool = vec![
            BuilderEndpoint {
                node_id: "b1".to_string(),
                url: "http://a:9090".to_string(),
                healthy: false,
            },
            BuilderEndpoint {
                node_id: "b2".to_string(),
                url: "http://b:9090".to_string(),
                healthy: true,
            },
        ];
        let selected = select_healthy_builder(&pool);
        assert!(selected.is_some());
        assert_eq!(selected.map(|s| s.node_id).as_deref(), Some("b2"));
    }

    #[test]
    fn mark_health_updates() {
        let mut ep = BuilderEndpoint {
            node_id: "b1".to_string(),
            url: "http://a:9090".to_string(),
            healthy: false,
        };
        mark_health(&mut ep, true);
        assert!(ep.healthy);
    }

    // ── health_check tests ──────────────────────────────────────────────

    #[test]
    fn health_check_returns_false_for_invalid_url() {
        // An unroutable address — connection will fail, so health_check must return false.
        let result = health_check("http://127.0.0.1:0");
        assert!(!result);
    }

    #[test]
    fn health_check_returns_false_for_malformed_url() {
        // A completely malformed URL — reqwest will fail to parse or connect.
        let result = health_check("not-a-url");
        assert!(!result);
    }

    #[test]
    fn health_check_returns_false_for_unreachable_host() {
        // Port 9 (discard) on localhost — connection will be refused.
        let result = health_check("http://127.0.0.1:9");
        assert!(!result);
    }

    // ── resolve_builder_pool_with_health tests ──────────────────────────

    #[test]
    fn with_health_empty_registry() {
        let reg = json!({"nodes": []});
        let pool = resolve_builder_pool_with_health(&reg);
        assert!(pool.is_empty());
    }

    #[test]
    fn with_health_returns_same_endpoints_as_no_health() {
        let reg = json!({
            "nodes": [
                {"id": "builder-1", "role": "build_runner", "network": {"service": "10.10.10.119"}},
                {"id": "cortex", "role": "cortex_host", "network": {"service": "10.10.10.100"}}
            ]
        });
        let plain = resolve_builder_pool(&reg);
        let checked = resolve_builder_pool_with_health(&reg);
        assert_eq!(plain.len(), checked.len());
        for (p, c) in plain.iter().zip(checked.iter()) {
            assert_eq!(p.node_id, c.node_id);
            assert_eq!(p.url, c.url);
            // In CI the builder may or may not be reachable; both true/false are valid.
            // We only assert the field was set (i.e. the call did not panic).
            let _ = c.healthy;
        }
    }

    #[test]
    fn with_health_all_unhealthy_when_builders_unreachable() {
        // Use a port that is guaranteed to be closed.
        let reg = json!({
            "nodes": [
                {"id": "builder-1", "role": "build_runner", "network": {"service": "127.0.0.1:9"}},
            ]
        });
        let pool = resolve_builder_pool_with_health(&reg);
        assert_eq!(pool.len(), 1);
        assert!(!pool[0].healthy);
        assert!(select_healthy_builder(&pool).is_none());
    }
}
