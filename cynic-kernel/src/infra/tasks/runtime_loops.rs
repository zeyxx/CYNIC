use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::events::KernelEvent;
use crate::domain::probe::{EnvironmentSnapshot, Probe, ProbeDetails};
use crate::domain::storage::StoragePort;
use crate::infra::task_health::TaskHealth;

/// How often the event consumer touches `task_health` even when the bus is
/// idle. Must be strictly less than half `task_health`'s `event_consumer`
/// expected_interval (300 s -> stale at 600 s) so an idle but alive consumer
/// never drifts into false-stale. 60 s matches the other liveness-style loops.
const EVENT_CONSUMER_LIVENESS_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

pub fn spawn_event_consumer(
    event_tx: &tokio::sync::broadcast::Sender<KernelEvent>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    spawn_event_consumer_with_liveness(
        event_tx,
        task_health,
        shutdown,
        EVENT_CONSUMER_LIVENESS_INTERVAL,
    )
}

/// Internal: same as `spawn_event_consumer` but with a configurable liveness
/// tick interval, so unit tests can exercise the idle-bus branch without
/// waiting a minute. Production always goes through `spawn_event_consumer`.
fn spawn_event_consumer_with_liveness(
    event_tx: &tokio::sync::broadcast::Sender<KernelEvent>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
    liveness_interval: std::time::Duration,
) -> JoinHandle<()> {
    let mut rx = event_tx.subscribe();
    let handle = tokio::spawn(async move {
        // Idle-liveness tick: the event bus may be quiet for hours. Keep the
        // consumer's liveness signal fresh even when nothing is emitted.
        let mut liveness = tokio::time::interval(liveness_interval);
        liveness.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        liveness.tick().await; // absorb the immediate first tick

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Event consumer stopped");
                    break;
                }
                event = rx.recv() => {
                    match event {
                        Ok(ref evt) => {
                            // Most events are logged at emission. This consumer
                            // acts on contract-level signals — the first acting
                            // consumer on the bus.
                            match evt {
                                KernelEvent::ContractDelta { missing, expected, fulfilled } => {
                                    if !fulfilled {
                                        tracing::warn!(
                                            missing = ?missing,
                                            expected = expected,
                                            "PROPRIOCEPTION: contract not fulfilled — {} Dog(s) missing",
                                            missing.len()
                                        );
                                    } else {
                                        tracing::info!("PROPRIOCEPTION: contract fulfilled — all {expected} Dogs present");
                                    }
                                }
                                KernelEvent::DogDiscovered { dog_id } => {
                                    tracing::info!(dog_id = %dog_id, "PROPRIOCEPTION: Dog discovered and registered");
                                }
                                _ => {}
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(skipped = n, "Event consumer lagged — dropped {n} events");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                    task_health.touch_event_consumer();
                }
                _ = liveness.tick() => {
                    task_health.touch_event_consumer();
                }
            }
        }
    });
    klog!("[Ring 2] Event consumer started (lag detection + liveness)");
    handle
}

pub fn spawn_probe_scheduler(
    probes: Vec<Arc<dyn Probe>>,
    storage: Arc<dyn StoragePort>,
    environment: Arc<std::sync::RwLock<Option<EnvironmentSnapshot>>>,
    organ: Arc<crate::organ::InferenceOrgan>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    use crate::infra::probes::ProbeScheduler;

    let mut scheduler = ProbeScheduler::new(probes);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick (consistent with all other tasks)

        let mut tick_count: u64 = 0;

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Probe scheduler stopped");
                    break;
                }
                _ = interval.tick() => {
                    // K6: per-probe timeouts inside scheduler.tick() satisfy K6.
                    // No outer timeout needed — each probe's .await is bounded.
                    if let Some(snapshot) = scheduler.tick().await {
                        if let Ok(mut env) = environment.write() {
                            *env = Some(snapshot.clone());
                        }

                        match tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            storage.store_infra_snapshot(&snapshot),
                        ).await {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => tracing::warn!("probe snapshot storage failed: {e}"),
                            Err(_) => tracing::warn!("probe snapshot storage timed out"),
                        }

                        for probe in &snapshot.probes {
                            if let ProbeDetails::Fleet(ref fleet) = probe.details {
                                for dog in &fleet.dogs {
                                    if dog.model_mismatch {
                                        organ.degrade_backend(
                                            &dog.dog_name,
                                            format!(
                                                "model mismatch: expected {:?}, actual {:?}",
                                                dog.expected_model, dog.actual_model
                                            ),
                                        );
                                    } else {
                                        organ.promote_if_gate_clear(&dog.dog_name);
                                    }
                                }
                            }
                        }

                        task_health.touch_probe_scheduler();
                    }

                    tick_count += 1;

                    if tick_count.is_multiple_of(100) {
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(10),
                            storage.cleanup_infra_snapshots(7),
                        ).await {
                            Ok(Ok(n)) if n > 0 => {
                                tracing::info!(deleted = n, "infra_snapshot TTL cleanup");
                            }
                            Ok(Err(e)) => tracing::warn!("infra_snapshot cleanup failed: {e}"),
                            Err(_) => tracing::warn!("infra_snapshot cleanup timed out"),
                            _ => {}
                        }
                    }
                }
            }
        }
    })
}

pub fn spawn_dog_ttl_checker(
    rest_state: Arc<crate::api::rest::AppState>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Dog TTL checker stopped");
                    break;
                }
                _ = interval.tick() => {
                    let expired: Vec<String> = match rest_state.registered_dogs.read() {
                        Ok(map) => map
                            .iter()
                            .filter(|(_, dog)| dog.last_heartbeat.elapsed().as_secs() > dog.ttl_secs)
                            .map(|(id, _)| id.clone())
                            .collect(),
                        Err(e) => {
                            tracing::warn!(error = %e, "registered_dogs RwLock poisoned — skipping TTL check");
                            vec![]
                        }
                    };

                    for dog_id in expired {
                        match rest_state.registered_dogs.write() {
                            Ok(mut map) => {
                                map.remove(&dog_id);
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, dog_id = %dog_id, "registered_dogs write lock poisoned — cannot remove expired dog");
                                continue;
                            }
                        }

                        let current = rest_state.judge.load_full();
                        if let Some(new_judge) = crate::judge::Judge::without_dog(&current, &dog_id) {
                            rest_state.judge.store(Arc::new(new_judge));
                            tracing::warn!(dog_id = %dog_id, "Dog TTL expired — removed from active roster");
                        }

                        let _ = rest_state
                            .event_tx
                            .send(crate::domain::events::KernelEvent::DogExpired {
                                dog_id: dog_id.clone(),
                            });
                    }

                    task_health.touch_dog_ttl();
                }
            }
        }
    })
}

/// Service discovery loop — organism-agnostic auto-registration.
/// Every 60s, probes known inference service addresses for /v1/models.
/// If a service responds with the expected model, registers it via POST /dogs/register.
/// Already-registered Dogs are skipped (409 conflict → no-op).
/// Offline services are gracefully skipped — TTL expiry handles cleanup.
///
/// Organism-agnostic: works with Windows services, Docker, Kubernetes, cloud APIs,
/// local processes — anything that exposes OpenAI-compatible /v1/models endpoint.
pub fn spawn_discovery_loop(
    rest_state: Arc<crate::api::rest::AppState>,
    fleet_meta: std::collections::HashMap<String, (String, u32, String, Option<String>)>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if fleet_meta.is_empty() {
            klog!("[Discovery] No discovery targets configured — skipping");
            return;
        }

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Discovery loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    let mut discovered = 0u32;

                    for (dog_name, (base_url, ctx_size, expected_model, api_key)) in &fleet_meta {
                        // Already registered? Check roster.
                        {
                            let map = match rest_state.registered_dogs.read() {
                                Ok(m) => m,
                                Err(e) => {
                                    tracing::warn!(error = %e, dog_id = %dog_name, "registered_dogs read lock poisoned");
                                    continue;
                                }
                            };
                            if map.contains_key(dog_name) {
                                // Already registered, heartbeat will maintain it.
                                continue;
                            }
                        }

                        // Probe /v1/models endpoint to verify model matches
                        let models_url = format!(
                            "{}/v1/models",
                            base_url.trim_end_matches('/').trim_end_matches("/v1")
                        );

                        let model_match = tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            async {
                                match reqwest::Client::new()
                                    .get(&models_url)
                                    .timeout(std::time::Duration::from_secs(5))
                                    .send()
                                    .await
                                {
                                    Ok(resp) if resp.status().is_success() => {
                                        match resp.json::<serde_json::Value>().await {
                                            Ok(body) => {
                                                // Extract model ID from response
                                                // Standard format: { "data": [{ "id": "model-name" }] }
                                                body
                                                    .get("data")
                                                    .and_then(|data| data.as_array())
                                                    .and_then(|arr| arr.first())
                                                    .and_then(|item| item.get("id"))
                                                    .and_then(|id| id.as_str())
                                                    .map(|s| s.contains(expected_model))
                                                    .unwrap_or(false)
                                            }
                                            Err(_) => false,
                                        }
                                    }
                                    _ => false,
                                }
                            },
                        )
                        .await
                        .unwrap_or(false);

                        if !model_match {
                            continue;
                        }

                        // Model matches — try to register via POST /dogs/register
                        let register_url = std::env::var("CYNIC_REST_ADDR")
                            .unwrap_or_else(|_| "http://localhost:3030".to_string());
                        let register_endpoint = format!("{register_url}/dogs/register");

                        let payload = serde_json::json!({
                            "name": dog_name,
                            "base_url": base_url,
                            "model": expected_model,
                            "context_size": ctx_size,
                            "timeout_secs": 90,
                            "api_key": api_key,
                        });

                        let api_key = rest_state.api_key.clone();
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(10),
                            async {
                                let mut req = reqwest::Client::new()
                                    .post(&register_endpoint)
                                    .json(&payload)
                                    .timeout(std::time::Duration::from_secs(10));

                                if let Some(key) = &api_key {
                                    req = req.bearer_auth(key);
                                }

                                req.send().await
                            },
                        )
                        .await
                        {
                            Ok(Ok(resp)) => {
                                match resp.status().as_u16() {
                                    200 => {
                                        discovered += 1;
                                        tracing::info!(
                                            dog_id = %dog_name,
                                            model = %expected_model,
                                            "Dog discovered and registered"
                                        );
                                        let _ = rest_state.event_tx.send(
                                            crate::domain::events::KernelEvent::DogDiscovered {
                                                dog_id: dog_name.clone(),
                                            },
                                        );
                                    }
                                    409 => {
                                        // Already registered (shouldn't happen after roster check, but ok)
                                        tracing::debug!(dog_id = %dog_name, "Dog already registered");
                                    }
                                    422 => {
                                        // Model mismatch or calibration failed — log and skip
                                        tracing::warn!(
                                            dog_id = %dog_name,
                                            status = 422,
                                            "Dog calibration failed (model mismatch?)"
                                        );
                                    }
                                    code => {
                                        tracing::warn!(
                                            dog_id = %dog_name,
                                            status = code,
                                            "Dog registration failed"
                                        );
                                    }
                                }
                            }
                            Ok(Err(e)) => {
                                tracing::debug!(dog_id = %dog_name, error = %e, "Discovery HTTP error");
                            }
                            Err(_) => {
                                tracing::debug!(dog_id = %dog_name, "Discovery timeout (10s)");
                            }
                        }
                    }

                    if discovered > 0 {
                        tracing::info!(count = discovered, "Discovery cycle: {} Dogs registered", discovered);
                    }

                    // Self-model: compare contract against live roster after each cycle.
                    // Emit ContractDelta so acting consumers can react.
                    let live_ids = rest_state.judge.load_full().dog_ids();
                    let delta = rest_state.system_contract.assess(&live_ids);
                    if !delta.fulfilled {
                        tracing::warn!(
                            missing = ?delta.missing,
                            expected = delta.expected,
                            actual = delta.actual,
                            "Contract gap: {} missing Dog(s)",
                            delta.missing.len()
                        );
                    }
                    let _ = rest_state.event_tx.send(
                        crate::domain::events::KernelEvent::ContractDelta {
                            missing: delta.missing,
                            expected: delta.expected,
                            fulfilled: delta.fulfilled,
                        },
                    );

                    task_health.touch_discovery();
                }
            }
        }
    })
}

/// Heartbeat loop — K15 consumer for Dog registration producer.
/// Every 40s, refreshes TTL for all registered Dogs to keep them alive.
/// Runs on the same interval as cynic-node's heartbeat sender (40s), ensuring
/// staggered timing: when a remote node's heartbeat is stale or unavailable,
/// the kernel can still keep bootstrap Dogs alive via local heartbeat.
pub fn spawn_dog_heartbeat_loop(
    rest_state: Arc<crate::api::rest::AppState>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // K6: 40s interval matches cynic-node heartbeat cadence
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(40));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Dog heartbeat loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    // Count Dogs needing heartbeat (TTL < 60s means heartbeat within safe margin)
                    let to_refresh: Vec<String> = match rest_state.registered_dogs.read() {
                        Ok(map) => map
                            .iter()
                            .filter(|(_, dog)| {
                                let elapsed = dog.last_heartbeat.elapsed().as_secs();
                                elapsed > dog.ttl_secs.saturating_sub(60)
                            })
                            .map(|(id, _)| id.clone())
                            .collect(),
                        Err(e) => {
                            tracing::warn!(error = %e, "registered_dogs RwLock poisoned — skipping heartbeat");
                            vec![]
                        }
                    };

                    if !to_refresh.is_empty() {
                        let mut updated = 0;
                        for dog_id in to_refresh {
                            match rest_state.registered_dogs.write() {
                                Ok(mut map) => {
                                    if let Some(entry) = map.get_mut(&dog_id) {
                                        entry.last_heartbeat = std::time::Instant::now();
                                        updated += 1;
                                        tracing::debug!(dog_id = %dog_id, "Dog heartbeat refreshed");
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        error = %e,
                                        dog_id = %dog_id,
                                        "registered_dogs write lock poisoned — cannot refresh heartbeat"
                                    );
                                }
                            }
                        }
                        if updated > 0 {
                            tracing::info!(count = updated, "Dogs heartbeat refreshed");
                        }
                    }

                    task_health.touch_dog_heartbeat();
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::rest::{AppState, PerIpRateLimiter, ReadyCache, StorageInfo};
    use crate::domain::coord::NullCoord;
    use crate::domain::embedding::NullEmbedding;
    use crate::domain::storage::NullStorage;
    use crate::domain::usage::DogUsageTracker;
    use crate::domain::verdict_cache::VerdictCache;
    use arc_swap::ArcSwap;

    #[tokio::test]
    async fn event_consumer_receives_and_shuts_down() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<KernelEvent>(16);
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_event_consumer(&tx, task_health.clone(), shutdown.clone());

        let _ = tx.send(KernelEvent::VerdictIssued {
            verdict_id: "test-v1".into(),
            domain: "test".into(),
            verdict: "Wag".into(),
            q_score: 0.5,
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let snapshot = task_health.snapshot();
        let consumer = snapshot.iter().find(|t| t.name == "event_consumer");
        assert!(
            consumer.is_some(),
            "event_consumer should appear in task_health after receiving an event"
        );

        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("event_consumer should stop within 2s")
            .expect("task should not panic");
    }

    #[tokio::test]
    async fn event_consumer_liveness_tick_touches_health_on_idle_bus() {
        use std::time::Duration;

        let (tx, _rx_sentinel) = tokio::sync::broadcast::channel::<KernelEvent>(16);
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_event_consumer_with_liveness(
            &tx,
            task_health.clone(),
            shutdown.clone(),
            Duration::from_millis(100),
        );

        tokio::time::sleep(Duration::from_millis(350)).await;

        let snapshot = task_health.snapshot();
        let consumer = snapshot
            .iter()
            .find(|t| t.name == "event_consumer")
            .expect("event_consumer entry in snapshot");
        assert_eq!(
            consumer.status, "ok",
            "liveness tick must keep event_consumer ok on an idle bus"
        );
        assert!(
            !task_health.has_stale(),
            "has_stale must not trip on an idle-but-alive event_consumer"
        );

        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("event_consumer should stop within 2s")
            .expect("task should not panic");
    }

    #[tokio::test]
    async fn probe_scheduler_respects_shutdown() {
        let probes: Vec<Arc<dyn Probe>> = vec![Arc::new(crate::domain::probe::NullProbe)];
        let storage: Arc<dyn StoragePort> = Arc::new(NullStorage);
        let environment = Arc::new(std::sync::RwLock::new(None));
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let organ = Arc::new(crate::organ::InferenceOrgan::boot_empty());
        let handle = spawn_probe_scheduler(
            probes,
            storage,
            environment,
            organ,
            task_health,
            shutdown.clone(),
        );
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("probe_scheduler should stop within 2s")
            .expect("task should not panic");
    }

    #[tokio::test]
    async fn dog_ttl_checker_respects_shutdown() {
        let (event_tx, _) = tokio::sync::broadcast::channel::<KernelEvent>(16);
        let judge = crate::judge::Judge::new(vec![], vec![]);
        let rest_state = Arc::new(AppState {
            judge: ArcSwap::new(Arc::new(judge)),
            storage: Arc::new(NullStorage),
            coord: Arc::new(NullCoord),
            embedding: Arc::new(NullEmbedding),
            usage: Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new())),
            verdict_cache: Arc::new(VerdictCache::new()),
            task_health: Arc::new(TaskHealth::new()),
            metrics: Arc::new(crate::domain::metrics::Metrics::new()),
            api_key: None,
            storage_info: StorageInfo {
                namespace: "test".into(),
                database: "test".into(),
            },
            rate_limiter: PerIpRateLimiter::new(30),
            judge_limiter: PerIpRateLimiter::new(10),
            ready_cache: ReadyCache::new(),
            bg_semaphore: Arc::new(tokio::sync::Semaphore::new(4)),
            bg_tasks: tokio_util::task::TaskTracker::new(),
            sse_semaphore: Arc::new(tokio::sync::Semaphore::new(4)),
            introspection_alerts: Arc::new(std::sync::RwLock::new(vec![])),
            event_tx: event_tx.clone(),
            chain_verified: std::sync::atomic::AtomicBool::new(false),
            environment: Arc::new(std::sync::RwLock::new(None)),
            registered_dogs: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            judge_jobs: Arc::new(crate::api::rest::judge_job::JudgeJobStore::new()),
            system_contract: crate::domain::contract::SystemContract::new(vec![], true),
        });
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_dog_ttl_checker(
            Arc::clone(&rest_state),
            Arc::clone(&task_health),
            shutdown.clone(),
        );
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("dog_ttl_checker should stop within 2s")
            .expect("task should not panic");
    }
}
