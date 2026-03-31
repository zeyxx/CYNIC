//! FleetProbe — HTTP health probes to Dog backend endpoints.
//! Reads health_url from backends.toml (via constructor) and probes each.
//! Reports ALL dog states (not just first failure). Checks HTTP status, not just connection.

use crate::domain::probe::{
    DogHealthDetails, FleetDetails, ProbeDetails, ProbeError, ProbeResult, ProbeStatus,
};
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::domain::probe::Probe;

/// A Dog endpoint to health-check.
#[derive(Debug, Clone)]
pub struct FleetTarget {
    pub dog_name: String,
    pub health_url: String,
    /// Explicit properties URL to check actual context limits.
    pub props_url: Option<String>,
    /// Expected context tokens from configuration.
    pub expected_n_ctx: u32,
}

/// Probe that HTTP GETs the health endpoint of each configured Dog backend.
/// Reports all results via FleetDetails, not just the first failure.
#[derive(Debug)]
pub struct FleetProbe {
    targets: Vec<FleetTarget>,
    client: reqwest::Client,
}

impl FleetProbe {
    pub fn new(targets: Vec<FleetTarget>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { targets, client }
    }
}

#[async_trait]
impl crate::domain::probe::Probe for FleetProbe {
    fn name(&self) -> &str {
        "fleet"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        if self.targets.is_empty() {
            return Ok(ProbeResult {
                name: "fleet".to_string(),
                status: ProbeStatus::Ok,
                details: ProbeDetails::Fleet(FleetDetails { dogs: Vec::new() }),
                duration_ms: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        // Probe all targets in parallel.
        let futures: Vec<_> = self
            .targets
            .iter()
            .map(|t| {
                let client = self.client.clone();
                let url = t.health_url.clone();
                let name = t.dog_name.clone();
                let props_url = t.props_url.clone();
                let expected_n_ctx = t.expected_n_ctx;
                async move {
                    let probe_start = Instant::now();
                    // Check HTTP status code, not just connection success.
                    let reachable = match client.get(&url).send().await {
                        Ok(resp) => resp.status().is_success(),
                        Err(_) => false,
                    };

                    let mut actual_n_ctx = None;
                    if let Some(ref p_url) = props_url
                        && let Ok(resp) = client.get(p_url).send().await
                        && let Ok(json) = resp.json::<serde_json::Value>().await
                    {
                        // Extract n_ctx from llama-server /props or /v1/models (best effort)
                        actual_n_ctx = json
                            .pointer("/default_generation_settings/n_ctx")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32);
                    }

                    let latency_ms = Some(probe_start.elapsed().as_millis() as u64);
                    DogHealthDetails {
                        dog_name: name,
                        reachable,
                        latency_ms,
                        actual_n_ctx,
                        expected_n_ctx: if expected_n_ctx > 0 {
                            Some(expected_n_ctx)
                        } else {
                            None
                        },
                    }
                }
            })
            .collect();

        let results = futures_util::future::join_all(futures).await;

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        let reachable_count = results.iter().filter(|r| r.reachable).count();
        let total = results.len();

        let status = if reachable_count == total {
            ProbeStatus::Ok
        } else if reachable_count == 0 {
            ProbeStatus::Unavailable
        } else {
            ProbeStatus::Degraded
        };

        // Log unreachable dogs (state-transition logging is a follow-up).
        for r in &results {
            if !r.reachable {
                tracing::warn!(
                    dog = %r.dog_name,
                    latency_ms = ?r.latency_ms,
                    "fleet probe: Dog unreachable"
                );
            }
            if let (Some(actual), Some(expected)) = (r.actual_n_ctx, r.expected_n_ctx)
                && actual < expected
            {
                tracing::error!(
                    dog = %r.dog_name,
                    actual = actual,
                    expected = expected,
                    "fleet probe: CONTEXT DRIFT DETECTED — backend running with reduced context"
                );
                klog!(
                    "[fleet] ⚠ {} CONTEXT DRIFT: running {} (configured {})",
                    r.dog_name,
                    actual,
                    expected
                );
            }
        }

        Ok(ProbeResult {
            name: "fleet".to_string(),
            status,
            details: ProbeDetails::Fleet(FleetDetails { dogs: results }),
            duration_ms,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn fleet_probe_no_targets_returns_ok() {
        let probe = FleetProbe::new(vec![]);
        let result = probe.sense().await.expect("should not error");
        assert_eq!(result.status, ProbeStatus::Ok);
    }

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn fleet_probe_unreachable_target() {
        let probe = FleetProbe::new(vec![FleetTarget {
            dog_name: "test-dog".into(),
            health_url: "http://127.0.0.1:19999/health".into(),
            props_url: None,
            expected_n_ctx: 0,
        }]);
        let result = probe.sense().await.expect("should not error");
        assert_eq!(result.status, ProbeStatus::Unavailable);
        match result.details {
            ProbeDetails::Fleet(ref f) => {
                assert_eq!(f.dogs.len(), 1);
                assert_eq!(f.dogs[0].dog_name, "test-dog");
                assert!(!f.dogs[0].reachable);
            }
            _ => panic!("expected Fleet details"),
        }
    }

    /// Spins up a minimal Axum server that simulates a llama-server with
    /// context drift: /health → 200, /props → n_ctx=2048 (less than expected 8192).
    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn fleet_probe_detects_context_drift() {
        use axum::{Json, Router, routing::get};

        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .route(
                "/props",
                get(|| async {
                    Json(serde_json::json!({
                        "default_generation_settings": { "n_ctx": 2048 }
                    }))
                }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let base = format!("http://{addr}");
        let probe = FleetProbe::new(vec![FleetTarget {
            dog_name: "drifted-dog".into(),
            health_url: format!("{base}/health"),
            props_url: Some(format!("{base}/props")),
            expected_n_ctx: 8192,
        }]);

        let result = probe.sense().await.expect("should not error");
        assert_eq!(result.status, ProbeStatus::Ok);

        match result.details {
            ProbeDetails::Fleet(ref f) => {
                assert_eq!(f.dogs.len(), 1);
                let dog = &f.dogs[0];
                assert!(dog.reachable);
                assert_eq!(
                    dog.actual_n_ctx,
                    Some(2048),
                    "should read actual n_ctx from /props"
                );
                assert_eq!(
                    dog.expected_n_ctx,
                    Some(8192),
                    "should carry expected n_ctx"
                );
                // Drift: 2048 < 8192 — logged as error (verified by log output, not assertion)
            }
            _ => panic!("expected Fleet details"),
        }
    }

    /// When expected_n_ctx is 0 (unconfigured), both fields should be None.
    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn fleet_probe_no_expected_ctx_yields_none() {
        let probe = FleetProbe::new(vec![FleetTarget {
            dog_name: "unconfigured-dog".into(),
            health_url: "http://127.0.0.1:19999/health".into(),
            props_url: None,
            expected_n_ctx: 0,
        }]);
        let result = probe.sense().await.expect("should not error");
        match result.details {
            ProbeDetails::Fleet(ref f) => {
                assert_eq!(f.dogs[0].expected_n_ctx, None, "0 sentinel → None");
                assert_eq!(f.dogs[0].actual_n_ctx, None, "no props_url → no actual");
            }
            _ => panic!("expected Fleet details"),
        }
    }
}
