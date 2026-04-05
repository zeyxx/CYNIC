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
    /// URL to query loaded model identity (e.g. /v1/models).
    pub models_url: Option<String>,
    /// Expected context tokens from configuration.
    pub expected_n_ctx: u32,
    /// Expected model name from backends.toml. Used for identity verification.
    pub expected_model: String,
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
                let models_url = t.models_url.clone();
                let expected_n_ctx = t.expected_n_ctx;
                let expected_model = t.expected_model.clone();
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
                        actual_n_ctx = json
                            .pointer("/default_generation_settings/n_ctx")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32);
                    }

                    // Model identity check: GET /v1/models, compare against backends.toml.
                    // This is B4: the root cause of "two models on same port" going undetected.
                    let mut actual_model = None;
                    let mut model_mismatch = false;
                    if reachable
                        && let Some(ref m_url) = models_url
                        && let Ok(resp) = client.get(m_url).send().await
                        && let Ok(json) = resp.json::<serde_json::Value>().await
                    {
                        // OpenAI-compatible /v1/models returns {"data": [{"id": "model-name"}]}
                        actual_model = json
                            .pointer("/data/0/id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        if let Some(ref actual) = actual_model
                            && !expected_model.is_empty()
                        {
                            // Substring match (same as verify_model_loaded at boot)
                            let actual_lower = actual.to_lowercase();
                            let expected_lower = expected_model.to_lowercase();
                            if !actual_lower.contains(&expected_lower)
                                && !expected_lower.contains(&actual_lower)
                            {
                                model_mismatch = true;
                            }
                        }
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
                        actual_model,
                        expected_model: if expected_model.is_empty() {
                            None
                        } else {
                            Some(expected_model)
                        },
                        model_mismatch,
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
            // B4: Model identity mismatch — the root cause of "two models on :8080" invisible for days.
            if r.model_mismatch {
                tracing::error!(
                    dog = %r.dog_name,
                    actual_model = ?r.actual_model,
                    expected_model = ?r.expected_model,
                    "fleet probe: MODEL IDENTITY MISMATCH — wrong model loaded on backend"
                );
                klog!(
                    "[fleet] ⚠ {} MODEL MISMATCH: running {:?} (configured {:?})",
                    r.dog_name,
                    r.actual_model,
                    r.expected_model
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

    #[tokio::test]
    async fn fleet_probe_no_targets_returns_ok() {
        let probe = FleetProbe::new(vec![]);
        let result = probe.sense().await.expect("should not error");
        assert_eq!(result.status, ProbeStatus::Ok);
    }

    #[tokio::test]
    async fn fleet_probe_unreachable_target() {
        let probe = FleetProbe::new(vec![FleetTarget {
            dog_name: "test-dog".into(),
            health_url: "http://127.0.0.1:19999/health".into(),
            props_url: None,
            models_url: None,
            expected_n_ctx: 0,
            expected_model: String::new(),
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
            models_url: None,
            expected_n_ctx: 8192,
            expected_model: String::new(),
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
    #[tokio::test]
    async fn fleet_probe_no_expected_ctx_yields_none() {
        let probe = FleetProbe::new(vec![FleetTarget {
            dog_name: "unconfigured-dog".into(),
            health_url: "http://127.0.0.1:19999/health".into(),
            props_url: None,
            models_url: None,
            expected_n_ctx: 0,
            expected_model: String::new(),
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

    /// B4: Detects when /v1/models returns a different model than backends.toml declares.
    #[tokio::test]
    async fn fleet_probe_detects_model_mismatch() {
        use axum::{Json, Router, routing::get};

        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .route(
                "/v1/models",
                get(|| async {
                    Json(serde_json::json!({
                        "data": [{"id": "gemma-3-4b-it", "object": "model"}]
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
            dog_name: "identity-test-dog".into(),
            health_url: format!("{base}/health"),
            props_url: None,
            models_url: Some(format!("{base}/v1/models")),
            expected_n_ctx: 0,
            expected_model: "Mistral-7B-v0.3".into(), // different from what's loaded
        }]);

        let result = probe.sense().await.expect("should not error");
        assert_eq!(result.status, ProbeStatus::Ok);

        match result.details {
            ProbeDetails::Fleet(ref f) => {
                let dog = &f.dogs[0];
                assert!(dog.reachable);
                assert_eq!(dog.actual_model.as_deref(), Some("gemma-3-4b-it"));
                assert_eq!(dog.expected_model.as_deref(), Some("Mistral-7B-v0.3"));
                assert!(dog.model_mismatch, "should detect model identity mismatch");
            }
            _ => panic!("expected Fleet details"),
        }
    }

    /// Model identity match should pass when names contain each other (substring).
    #[tokio::test]
    async fn fleet_probe_model_match_by_substring() {
        use axum::{Json, Router, routing::get};

        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .route(
                "/v1/models",
                get(|| async {
                    Json(serde_json::json!({
                        "data": [{"id": "Qwen/Qwen2.5-7B-Instruct", "object": "model"}]
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
            dog_name: "match-test-dog".into(),
            health_url: format!("{base}/health"),
            props_url: None,
            models_url: Some(format!("{base}/v1/models")),
            expected_n_ctx: 0,
            expected_model: "Qwen2.5-7B".into(), // substring of what's loaded
        }]);

        let result = probe.sense().await.expect("should not error");
        match result.details {
            ProbeDetails::Fleet(ref f) => {
                assert!(
                    !f.dogs[0].model_mismatch,
                    "substring match should not be flagged as mismatch"
                );
            }
            _ => panic!("expected Fleet details"),
        }
    }
}
