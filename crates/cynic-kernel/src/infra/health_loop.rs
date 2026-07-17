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

use crate::domain::health_gate::HealthGate;
use crate::domain::slot_semaphore::SlotSemaphoreMap;
use crate::domain::slot_tracker::SlotTracker;
use crate::domain::storage::StoragePort;
use crate::infra::circuit_breaker::PROBE_INTERVAL;
use crate::infra::task_health::TaskHealth;

/// Configuration for probing a single Dog backend.
#[derive(Debug)]
pub struct DogProbeConfig {
    pub dog_id: String,
    pub health_url: String,
    /// llama-server `/slots` URL — derived from health_url. None for cloud APIs.
    pub slots_url: Option<String>,
    /// API key for llama-server auth. Sent as `Authorization: Bearer` on `/slots`.
    /// Without this, llama-server returns 401 and SlotTracker stays empty.
    pub api_key: Option<String>,
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

/// Parse llama-server `/slots` JSON into domain slot descriptors.
/// Infra responsibility: JSON parsing stays out of domain (K5).
/// Expected format: `[{"id":0,"is_processing":false,"n_ctx":32768,...}, ...]`
fn parse_slots_json(json: &serde_json::Value) -> Option<Vec<(u32, bool, u32)>> {
    let arr = json.as_array()?;
    let mut descriptors = Vec::with_capacity(arr.len());
    for entry in arr {
        let id = entry.get("id")?.as_u64()? as u32;
        let is_processing = entry
            .get("is_processing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let n_ctx = entry.get("n_ctx").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        descriptors.push((id, is_processing, n_ctx));
    }
    Some(descriptors)
}

/// Probe llama-server `/slots` endpoint. Returns parsed slot data or None.
/// Times out after 3 seconds (shorter than health probe — slots is lightweight).
async fn probe_slots(
    client: &Client,
    slots_url: &str,
    api_key: Option<&str>,
) -> Option<crate::domain::slot_tracker::BackendSlots> {
    let timeout = Duration::from_secs(3);
    let mut req = client.get(slots_url);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    let resp = tokio::time::timeout(timeout, req.send())
        .await
        .ok()? // R2-exempt: timeout → None (skip slot update)
        .ok()?; // R2-exempt: connection error → None
    if !resp.status().is_success() {
        tracing::warn!(
            slots_url,
            status = %resp.status(),
            has_key = api_key.is_some(),
            "slot probe failed — SlotTracker will report stale data"
        );
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?; // R2-exempt: parse error → None
    let descriptors = parse_slots_json(&json)?;
    Some(crate::domain::slot_tracker::build_backend_slots(
        descriptors,
    ))
}

/// Spawn a background Tokio task that probes all dogs every `PROBE_INTERVAL`.
///
/// `configs` and `breakers` are parallel — index `i` in configs corresponds
/// to index `i` in breakers (same dog).
#[allow(clippy::too_many_arguments)]
// WHY: 10 args (3 over limit). Each is a distinct Arc/Vec with no natural grouping.
// A HealthLoopConfig struct would just move the problem — callers still pass 10 values.
pub fn spawn_health_loop(
    configs: Vec<DogProbeConfig>,
    breakers: Vec<Arc<dyn HealthGate>>,
    task_health: Arc<TaskHealth>,
    storage: Arc<dyn StoragePort>,
    slot_tracker: Arc<SlotTracker>,
    slot_semaphores: Arc<SlotSemaphoreMap>,
    routing_calc: Arc<crate::infra::routing_calc::RoutingCalculator>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Track previous probe state per Dog for transition detection
        let mut prev_healthy: std::collections::HashMap<String, bool> =
            std::collections::HashMap::new();

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
                    let dogs_total = results.len();
                    let dogs_healthy = results.iter().filter(|ok| **ok).count();

                    for (i, ok) in results.iter().enumerate() {
                        let cb = &breakers[i];
                        let dog_id = cb.dog_id();
                        if *ok {
                            cb.record_success();
                        } else {
                            cb.record_failure(crate::domain::health_gate::FailureReason::HealthProbe);
                            failure_count += 1;
                            klog!(
                                "[health_loop] Dog '{}' probe failed ({})",
                                dog_id,
                                configs[i].health_url
                            );
                        }

                        // Detect state transitions → emit observations
                        let was_healthy = prev_healthy.get(dog_id).copied();
                        if was_healthy.is_some() && was_healthy != Some(*ok) {
                            let transition = if *ok { "recovered" } else { "degraded" };
                            // Option A: reset routing data on recovery → benefit of doubt restored (K22)
                            // Prevents stale failure history from permanently excluding a recovered dog.
                            if *ok {
                                routing_calc.reset_dog(dog_id);
                            }
                            let context = format!(
                                "{{\"transition\":\"{transition}\",\"dog_id\":\"{dog_id}\",\"healthy\":{ok},\"dogs_healthy\":{dogs_healthy},\"dogs_total\":{dogs_total}}}"
                            );
                            let obs = crate::domain::ccm::build_observation_with_ledger(
                                "health_loop".to_string(),
                                Some(dog_id.to_string()),
                                Some("kernel-lifecycle".to_string()),
                                Some(transition.to_string()),
                                Some(context),
                                None, None, None, None,
                                None, Some("observed".to_string()),
                                Some("organism".to_string()),
                                Some("self-healing signal".to_string()),
                                None, None,
                            );
                            // Fire-and-forget — don't block the health loop
                            let s = Arc::clone(&storage);
                            tokio::spawn(async move {
                                let _ = s.store_observation(&obs).await;
                            });
                            klog!(
                                "[health_loop] Dog '{}' transition: {} ({}→{})",
                                dog_id, transition,
                                if was_healthy == Some(true) { "healthy" } else { "unhealthy" },
                                if *ok { "healthy" } else { "unhealthy" }
                            );
                        }
                        prev_healthy.insert(dog_id.to_string(), *ok);
                    }

                    if failure_count > 0 {
                        klog!(
                            "[health_loop] probe tick: {}/{} dogs healthy",
                            dogs_healthy,
                            dogs_total
                        );
                    }

                    // Soma L2: probe /slots on sovereign backends (parallel, fire-and-forget timing)
                    let slot_futures: Vec<_> = configs.iter()
                        .filter_map(|cfg| {
                            cfg.slots_url.as_ref().map(|url| {
                                let dog_id = cfg.dog_id.clone();
                                let url = url.clone();
                                let api_key = cfg.api_key.clone();
                                let client = &client;
                                async move { (dog_id, probe_slots(client, &url, api_key.as_deref()).await) }
                            })
                        })
                        .collect();
                    if !slot_futures.is_empty() {
                        let slot_results = join_all(slot_futures).await;
                        for (dog_id, maybe_slots) in slot_results {
                            if let Some(slots) = maybe_slots {
                                tracing::debug!(
                                    dog_id = %dog_id,
                                    total = slots.total,
                                    busy = slots.busy,
                                    "slot probe update"
                                );
                                let total_slots = slots.total;
                                slot_tracker.update(&dog_id, slots);
                                slot_semaphores.upsert(&dog_id, total_slots);
                            }
                            // Soma L2+: tick saturation counter for this Dog
                            slot_tracker.tick_saturation(&dog_id);
                        }
                        // After the loop: check for stuck slots
                        for dog_id in slot_tracker.saturated_dogs(3) {
                            klog!(
                                "[health_loop] Dog '{}' slots saturated for 3+ ticks — signaling remediation",
                                dog_id
                            );
                            if let Some(idx) = configs.iter().position(|c| c.dog_id == dog_id) {
                                breakers[idx].record_failure(
                                    crate::domain::health_gate::FailureReason::SlotSaturation
                                );
                            }
                        }
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
            slots_url: None,
            api_key: None,
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
            slots_url: None,
            api_key: None,
        };
        let result = probe_dog(&client, &cfg).await;
        assert!(!result, "expected false for unreachable port");
    }

    #[test]
    fn parse_slots_json_valid() {
        let json = serde_json::json!([
            {"id": 0, "is_processing": false, "n_ctx": 32768},
            {"id": 1, "is_processing": true, "n_ctx": 32768}
        ]);
        let result = parse_slots_json(&json).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], (0, false, 32768));
        assert_eq!(result[1], (1, true, 32768));
    }

    #[test]
    fn parse_slots_json_not_array() {
        let json = serde_json::json!({"error": "not slots"});
        assert!(parse_slots_json(&json).is_none());
    }

    #[test]
    fn parse_slots_json_missing_id() {
        let json = serde_json::json!([{"is_processing": false, "n_ctx": 32768}]);
        assert!(parse_slots_json(&json).is_none());
    }
}
