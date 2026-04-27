//! Background health probe loop — polls Dog health URLs every 30 seconds.
//!
//! Each tick probes all dogs in parallel. Results feed into the same
//! CircuitBreakers used by the /judge path, enabling automatic Dog
//! skipping without any per-request overhead.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::future::join_all;
use reqwest::Client;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tokio_util::sync::CancellationToken;

use crate::domain::health_gate::HealthGate;
use crate::domain::organ::OrganPort;
use crate::infra::circuit_breaker::PROBE_INTERVAL;
use crate::infra::task_health::TaskHealth;

/// Configuration for probing a single Dog backend.
#[derive(Debug)]
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
            klog!(
                "[health_loop] Dog '{}' returned HTTP {} ({})",
                config.dog_id,
                resp.status(),
                config.health_url
            );
            false
        }
        Ok(Err(e)) => {
            klog!(
                "[health_loop] Dog '{}' unreachable: {} ({})",
                config.dog_id,
                e,
                config.health_url
            );
            false
        }
        Err(_) => {
            klog!(
                "[health_loop] Dog '{}' probe timed out (5s) ({})",
                config.dog_id,
                config.health_url
            );
            false
        }
    }
}

/// Fleet-aware hysteresis — tracks node offline duration.
/// Only marks Dogs after a node has been offline for HYSTERESIS_THRESHOLD
/// across at least 2 consecutive ticks.
pub(crate) struct FleetAwareness {
    /// Dog ID → Tailscale hostname
    dog_to_node: HashMap<String, String>,
    /// Node hostname → first time seen offline
    pub(crate) node_offline_since: HashMap<String, Instant>,
}

const HYSTERESIS_THRESHOLD: Duration = Duration::from_secs(30);

impl FleetAwareness {
    pub(crate) fn new(dog_to_node: HashMap<String, String>) -> Self {
        Self {
            dog_to_node,
            node_offline_since: HashMap::new(),
        }
    }

    /// Process one tick. Returns Dog IDs that should be preemptively marked dead.
    pub(crate) fn tick(&mut self, offline_nodes: &[String]) -> Vec<String> {
        let mut to_mark = Vec::new();

        // Clear nodes that came back online
        self.node_offline_since
            .retain(|node, _| offline_nodes.contains(node));

        // Process offline nodes
        for node in offline_nodes {
            let entry = self
                .node_offline_since
                .entry(node.clone())
                .or_insert_with(Instant::now);
            if entry.elapsed() >= HYSTERESIS_THRESHOLD {
                // Find all Dogs mapped to this node
                for (dog_id, mapped_node) in &self.dog_to_node {
                    if mapped_node == node {
                        to_mark.push(dog_id.clone());
                    }
                }
            }
        }

        to_mark
    }
}

/// Spawn a background Tokio task that probes all dogs every `PROBE_INTERVAL`.
///
/// `configs` and `breakers` are parallel — index `i` in configs corresponds
/// to index `i` in breakers (same dog).
pub fn spawn_health_loop(
    configs: Vec<DogProbeConfig>,
    breakers: Vec<Arc<dyn HealthGate>>,
    task_health: Arc<TaskHealth>,
    senses: Vec<Arc<dyn OrganPort>>,
    dog_to_fleet_node: HashMap<String, String>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut fleet_awareness = FleetAwareness::new(dog_to_fleet_node);

        let client = match Client::builder().timeout(Duration::from_secs(5)).build() {
            Ok(c) => c,
            Err(e) => {
                klog!(
                    "[health_loop] FATAL: failed to build HTTP client: {} — loop will not run",
                    e
                );
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
                            cb.record_failure(crate::domain::health_gate::FailureReason::HealthProbe);
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
                    // Fleet awareness: preemptive circuit breaking via Tailscale sense
                    if let Some(ts_sense) = senses.iter().find(|s| s.name() == "tailscale") {
                        match tokio::time::timeout(Duration::from_secs(5), ts_sense.snapshot()).await {
                            Ok(Ok(snap)) => {
                                let offline_nodes: Vec<String> = snap.metrics.iter()
                                    .filter(|m| m.key.starts_with("node_") && m.key.ends_with("_online"))
                                    .filter(|m| matches!(&m.value, crate::domain::organ::MetricValue::Bool(false)))
                                    .filter_map(|m| {
                                        m.key.strip_prefix("node_")
                                            .and_then(|s| s.strip_suffix("_online"))
                                            .map(|s| s.to_string())
                                    })
                                    .collect();

                                let dogs_to_mark = fleet_awareness.tick(&offline_nodes);
                                for dog_id in &dogs_to_mark {
                                    if let Some(idx) = configs.iter().position(|c| c.dog_id == *dog_id) {
                                        breakers[idx].record_failure(
                                            crate::domain::health_gate::FailureReason::FleetOffline
                                        );
                                        klog!("[health_loop] Dog '{}' preemptively marked — fleet node offline >30s", dog_id);
                                    }
                                }
                            }
                            Ok(Err(e)) => tracing::debug!(error = %e, "tailscale snapshot failed — skipping fleet check"),
                            Err(_) => tracing::debug!("tailscale snapshot timed out — skipping fleet check"),
                        }
                    }

                    task_health.touch_health_loop();
                }
            }
        }
    })
}

#[cfg(test)]
mod hysteresis_tests {
    use super::*;

    #[test]
    fn first_offline_does_not_trigger() {
        let mut fa = FleetAwareness::new(HashMap::from([("gpu-dog".into(), "cynic-gpu".into())]));
        let to_mark = fa.tick(&["cynic-gpu".into()]);
        assert!(to_mark.is_empty(), "first tick should not mark anything");
    }

    #[test]
    fn sustained_offline_triggers_after_threshold() {
        let mut fa = FleetAwareness::new(HashMap::from([("gpu-dog".into(), "cynic-gpu".into())]));
        // First tick: record offline
        fa.tick(&["cynic-gpu".into()]);
        // Simulate time passing (>30s) by backdating the Instant
        fa.node_offline_since
            .insert("cynic-gpu".into(), Instant::now() - Duration::from_secs(31));
        // Second tick: should trigger
        let to_mark = fa.tick(&["cynic-gpu".into()]);
        assert_eq!(to_mark, vec!["gpu-dog"]);
    }

    #[test]
    fn node_comes_back_clears_state() {
        let mut fa = FleetAwareness::new(HashMap::from([("gpu-dog".into(), "cynic-gpu".into())]));
        fa.tick(&["cynic-gpu".into()]);
        // Node comes back
        fa.tick(&[]);
        assert!(
            !fa.node_offline_since.contains_key("cynic-gpu"),
            "should clear offline state"
        );
    }

    #[test]
    fn multiple_dogs_on_same_node() {
        let mut fa = FleetAwareness::new(HashMap::from([
            ("dog-a".into(), "shared-node".into()),
            ("dog-b".into(), "shared-node".into()),
        ]));
        fa.tick(&["shared-node".into()]);
        fa.node_offline_since.insert(
            "shared-node".into(),
            Instant::now() - Duration::from_secs(31),
        );
        let mut to_mark = fa.tick(&["shared-node".into()]);
        to_mark.sort();
        assert_eq!(to_mark, vec!["dog-a", "dog-b"]);
    }

    #[test]
    fn empty_dog_map_never_marks() {
        let mut fa = FleetAwareness::new(HashMap::new());
        fa.tick(&["cynic-gpu".into()]);
        fa.node_offline_since
            .insert("cynic-gpu".into(), Instant::now() - Duration::from_secs(31));
        let to_mark = fa.tick(&["cynic-gpu".into()]);
        assert!(to_mark.is_empty());
    }
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
