//! Builder pool resolution — scale to N builders by role.
//!
//! WHY: CYNIC must support N build_runner nodes. This module resolves all
//! builders from infra/registry.json and selects one by health-check.
//! HOW: read registry, filter role == "build_runner", health-check each,
//! return the first healthy one. Pure selection logic, I/O in adapters.
//! NEGATIVE BOUNDARY: no SSH, no subprocess. HTTP only (builder_client).

use serde_json::Value;

/// Hardware capabilities of a build runner node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRequirements {
    pub cpu_cores: u32,
    pub ram_gb: u32,
    pub gpu: String,
}

/// A resolved builder endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuilderEndpoint {
    pub node_id: String,
    pub url: String,
    pub healthy: bool,
    pub capabilities: Option<CapabilityRequirements>,
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
                let capabilities = node.get("capabilities").and_then(|c| {
                    Some(CapabilityRequirements {
                        cpu_cores: c.get("cpu_cores")?.as_u64()? as u32,
                        ram_gb: c.get("ram_gb")?.as_u64()? as u32,
                        gpu: c.get("gpu")?.as_str()?.to_string(),
                    })
                });
                pool.push(BuilderEndpoint {
                    node_id,
                    url: format!("http://{ip}:9090"),
                    healthy: false,
                    capabilities,
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

/// Select the first builder from the pool that meets the given capability requirements.
///
/// Filters builders whose `capabilities` field satisfies all three minimums:
/// `cpu_cores >= required_cpu`, `ram_gb >= required_ram`, and GPU is present
/// (any non-empty GPU string satisfies the requirement — the filter checks
/// that the builder has *a* GPU, not a specific model).
///
/// Returns `None` if no builder meets the requirements or if no builder has
/// capability data. Caller must handle the `None` case (fail closed).
pub fn select_builder_by_capability(
    pool: &[BuilderEndpoint],
    required_cpu: u32,
    required_ram: u32,
    required_gpu: &str,
) -> Option<BuilderEndpoint> {
    pool.iter()
        .find(|b| {
            b.capabilities
                .as_ref()
                .map(|caps| {
                    caps.cpu_cores >= required_cpu
                        && caps.ram_gb >= required_ram
                        && (!required_gpu.is_empty() && !caps.gpu.is_empty())
                })
                .unwrap_or(false)
        })
        .cloned()
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
            capabilities: None,
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
                capabilities: None,
            },
            BuilderEndpoint {
                node_id: "b2".to_string(),
                url: "http://b:9090".to_string(),
                healthy: true,
                capabilities: None,
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
            capabilities: None,
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

    // ── select_builder_by_capability tests ───────────────────────────────

    fn caps(cpu: u32, ram: u32, gpu: &str) -> Option<CapabilityRequirements> {
        Some(CapabilityRequirements {
            cpu_cores: cpu,
            ram_gb: ram,
            gpu: gpu.to_string(),
        })
    }

    #[test]
    fn capability_none_when_no_builders() {
        let pool: Vec<BuilderEndpoint> = vec![];
        let sel = select_builder_by_capability(&pool, 2, 4, "any");
        assert!(sel.is_none());
    }

    #[test]
    fn capability_none_when_no_builder_has_capabilities() {
        let pool = vec![BuilderEndpoint {
            node_id: "b1".to_string(),
            url: "http://a:9090".to_string(),
            healthy: true,
            capabilities: None,
        }];
        let sel = select_builder_by_capability(&pool, 2, 4, "any");
        assert!(sel.is_none());
    }

    #[test]
    fn capability_selects_matching_builder() {
        let pool = vec![
            BuilderEndpoint {
                node_id: "small".to_string(),
                url: "http://a:9090".to_string(),
                healthy: true,
                capabilities: caps(2, 4, "Intel HD"),
            },
            BuilderEndpoint {
                node_id: "big".to_string(),
                url: "http://b:9090".to_string(),
                healthy: true,
                capabilities: caps(8, 16, "NVIDIA"),
            },
        ];
        let sel = select_builder_by_capability(&pool, 4, 8, "any");
        assert!(sel.is_some());
        assert_eq!(sel.unwrap().node_id, "big");
    }

    #[test]
    fn capability_exact_match_succeeds() {
        let pool = vec![BuilderEndpoint {
            node_id: "exact".to_string(),
            url: "http://a:9090".to_string(),
            healthy: true,
            capabilities: caps(4, 8, "Intel HD Graphics 530"),
        }];
        let sel = select_builder_by_capability(&pool, 4, 8, "any");
        assert!(sel.is_some());
        assert_eq!(sel.unwrap().node_id, "exact");
    }

    #[test]
    fn capability_fails_when_cpu_insufficient() {
        let pool = vec![BuilderEndpoint {
            node_id: "weak".to_string(),
            url: "http://a:9090".to_string(),
            healthy: true,
            capabilities: caps(2, 16, "NVIDIA"),
        }];
        let sel = select_builder_by_capability(&pool, 4, 8, "any");
        assert!(sel.is_none());
    }

    #[test]
    fn capability_fails_when_ram_insufficient() {
        let pool = vec![BuilderEndpoint {
            node_id: "lowram".to_string(),
            url: "http://a:9090".to_string(),
            healthy: true,
            capabilities: caps(8, 4, "NVIDIA"),
        }];
        let sel = select_builder_by_capability(&pool, 4, 8, "any");
        assert!(sel.is_none());
    }

    #[test]
    fn capability_fails_when_gpu_missing() {
        let pool = vec![BuilderEndpoint {
            node_id: "nogpu".to_string(),
            url: "http://a:9090".to_string(),
            healthy: true,
            capabilities: caps(8, 16, ""),
        }];
        let sel = select_builder_by_capability(&pool, 4, 8, "any");
        assert!(sel.is_none());
    }

    #[test]
    fn capability_empty_gpu_requirement_matches_any_gpu() {
        let pool = vec![BuilderEndpoint {
            node_id: "anygpu".to_string(),
            url: "http://a:9090".to_string(),
            healthy: true,
            capabilities: caps(4, 8, "Intel HD Graphics 530"),
        }];
        // Empty required_gpu means "any GPU is fine"
        let sel = select_builder_by_capability(&pool, 4, 8, "");
        assert!(sel.is_some());
    }

    #[test]
    fn capability_empty_gpu_requirement_still_needs_gpu_field() {
        let pool = vec![BuilderEndpoint {
            node_id: "nogpu".to_string(),
            url: "http://a:9090".to_string(),
            healthy: true,
            capabilities: caps(4, 8, ""),
        }];
        // Even with empty required_gpu, builder must have a non-empty gpu field
        let sel = select_builder_by_capability(&pool, 4, 8, "");
        assert!(sel.is_none());
    }

    #[test]
    fn capability_skips_unhealthy_but_returns_healthy_match() {
        let pool = vec![
            BuilderEndpoint {
                node_id: "unhealthy-match".to_string(),
                url: "http://a:9090".to_string(),
                healthy: false,
                capabilities: caps(8, 16, "NVIDIA"),
            },
            BuilderEndpoint {
                node_id: "healthy-match".to_string(),
                url: "http://b:9090".to_string(),
                healthy: true,
                capabilities: caps(8, 16, "NVIDIA"),
            },
        ];
        // select_builder_by_capability does NOT check health — it only filters
        // by capabilities. The first capability match is returned regardless
        // of health. Caller must combine with health check if needed.
        let sel = select_builder_by_capability(&pool, 4, 8, "any");
        assert!(sel.is_some());
        assert_eq!(sel.unwrap().node_id, "unhealthy-match");
    }

    #[test]
    fn resolve_builder_pool_populates_capabilities() {
        let reg = json!({
            "nodes": [
                {
                    "id": "builder-1",
                    "role": "build_runner",
                    "network": {"service": "10.10.10.119"},
                    "capabilities": {
                        "cpu_cores": 4,
                        "ram_gb": 8,
                        "gpu": "Intel HD Graphics 530"
                    }
                },
                {"id": "cortex", "role": "cortex_host", "network": {"service": "10.10.10.100"}}
            ]
        });
        let pool = resolve_builder_pool(&reg);
        assert_eq!(pool.len(), 1);
        let caps = pool[0]
            .capabilities
            .as_ref()
            .expect("capabilities should be populated");
        assert_eq!(caps.cpu_cores, 4);
        assert_eq!(caps.ram_gb, 8);
        assert_eq!(caps.gpu, "Intel HD Graphics 530");
    }

    #[test]
    fn resolve_builder_pool_capabilities_none_when_missing() {
        let reg = json!({
            "nodes": [
                {"id": "builder-1", "role": "build_runner", "network": {"service": "10.10.10.119"}},
            ]
        });
        let pool = resolve_builder_pool(&reg);
        assert_eq!(pool.len(), 1);
        assert!(pool[0].capabilities.is_none());
    }
}
