//! Background health probe loop — polls Dog health URLs every 30 seconds.
//!
//! Each tick probes all dogs in parallel. Results feed into the same
//! CircuitBreakers used by the /judge path, enabling automatic Dog
//! skipping without any per-request overhead.

use std::sync::Arc;
use std::time::Duration;

use futures_util::future::join_all;
use reqwest::Client;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tokio_util::sync::CancellationToken;

use crate::infra::circuit_breaker::{CircuitBreaker, PROBE_INTERVAL};
use crate::infra::task_health::TaskHealth;

/// Configuration for probing a single Dog backend.
pub struct DogProbeConfig {
    pub dog_id: String,
    pub health_url: String,
}

/// GET the health URL, returning true if the response is a 2xx status.
/// Times out after 5 seconds.
pub(crate) async fn probe_dog(client: &Client, config: &DogProbeConfig) -> bool {
    let timeout = Duration::from_secs(5);
    match tokio::time::timeout(timeout, client.get(&config.health_url).send()).await {
        Ok(Ok(resp)) if resp.status().is_success() => true,
        Ok(Ok(resp)) => {
            klog!("[health_loop] Dog '{}' returned HTTP {} ({})", config.dog_id, resp.status(), config.health_url);
            false
        }
        Ok(Err(e)) => {
            klog!("[health_loop] Dog '{}' unreachable: {} ({})", config.dog_id, e, config.health_url);
            false
        }
        Err(_) => {
            klog!("[health_loop] Dog '{}' probe timed out (5s) ({})", config.dog_id, config.health_url);
            false
        }
    }
}

/// Spawn a background Tokio task that probes all dogs every `PROBE_INTERVAL`.
///
/// `configs` and `breakers` are parallel — index `i` in configs corresponds
/// to index `i` in breakers (same dog).
pub fn spawn_health_loop(
    configs: Vec<DogProbeConfig>,
    breakers: Vec<Arc<CircuitBreaker>>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let client = match Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                klog!("[health_loop] FATAL: failed to build HTTP client: {} — loop will not run", e);
                return;
            }
        };

        let mut ticker = interval(PROBE_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        ticker.tick().await; // skip first immediate tick

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Health loop stopped");
                    break;
                }
                _ = ticker.tick() => {
                    // Probe all dogs in parallel.
                    let futures = configs.iter().map(|cfg| probe_dog(&client, cfg));
                    let results: Vec<bool> = join_all(futures).await;

                    let mut failure_count = 0usize;
                    for (i, ok) in results.iter().enumerate() {
                        let cb = &breakers[i];
                        if *ok {
                            cb.record_success();
                        } else {
                            cb.record_failure();
                            failure_count += 1;
                            klog!(
                                "[health_loop] Dog '{}' probe failed ({})",
                                cb.dog_id(),
                                configs[i].health_url
                            );
                        }
                    }

                    if failure_count > 0 {
                        klog!(
                            "[health_loop] probe tick: {}/{} dogs healthy",
                            results.len() - failure_count,
                            results.len()
                        );
                    }
                    task_health.touch_health_loop();
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_config_stores_fields() {
        let cfg = DogProbeConfig {
            dog_id: "test-dog".to_string(),
            health_url: "http://localhost:9999/health".to_string(),
        };
        assert_eq!(cfg.dog_id, "test-dog");
        assert_eq!(cfg.health_url, "http://localhost:9999/health");
    }

    #[tokio::test]
    async fn probe_dog_returns_false_on_unreachable() {
        let client = Client::new();
        let cfg = DogProbeConfig {
            dog_id: "dead-dog".to_string(),
            // Port chosen to be unreachable — nothing listens on 19999.
            health_url: "http://127.0.0.1:19999/health".to_string(),
        };
        let result = probe_dog(&client, &cfg).await;
        assert!(!result, "expected false for unreachable port");
    }
}
