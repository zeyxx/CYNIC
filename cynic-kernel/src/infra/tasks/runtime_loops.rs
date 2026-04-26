use std::sync::Arc;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::constants;
use crate::domain::events::KernelEvent;
use crate::domain::probe::{EnvironmentSnapshot, Probe, ProbeDetails};
use crate::domain::storage::StoragePort;
use crate::infra::alerts::SlackAlerter;
use crate::infra::task_health::TaskHealth;
use chrono::Utc;

pub fn spawn_event_consumer(
    event_tx: &tokio::sync::broadcast::Sender<KernelEvent>,
    rest_state: Arc<crate::api::rest::AppState>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
    slack: Option<SlackAlerter>,
) -> JoinHandle<()> {
    spawn_event_consumer_with_liveness(
        event_tx,
        rest_state,
        task_health,
        shutdown,
        constants::EVENT_LIVENESS_INTERVAL,
        slack,
    )
}

/// Internal: same as `spawn_event_consumer` but with a configurable liveness
/// tick interval, so unit tests can exercise the idle-bus branch without
/// waiting a minute. Production always goes through `spawn_event_consumer`.
fn spawn_event_consumer_with_liveness(
    event_tx: &tokio::sync::broadcast::Sender<KernelEvent>,
    rest_state: Arc<crate::api::rest::AppState>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
    liveness_interval: std::time::Duration,
    slack: Option<SlackAlerter>,
) -> JoinHandle<()> {
    let mut rx = event_tx.subscribe();
    tokio::spawn(async move {
        // Idle-liveness tick: the event bus may be quiet for hours. Keep the
        // consumer's liveness signal fresh even when nothing is emitted.
        let mut liveness = tokio::time::interval(liveness_interval);
        liveness.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        liveness.tick().await; // absorb the immediate first tick

        // State tracking for ContractDelta changes (K15 acting consumer).
        let mut last_fulfilled: Option<bool> = None;
        let mut missing_since: std::collections::HashMap<String, std::time::Instant> =
            std::collections::HashMap::new();

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Event consumer stopped");
                    break;
                }
                event = rx.recv() => {
                    match event {
                        Ok(ref evt) => {
                            // Kernel self-observation: significant events → /observe store.
                            // Closes maladie 2: kernel tracks its own lifecycle events.
                            let self_obs = match evt {
                                KernelEvent::DogFailed { dog_id, error } =>
                                    Some(("dog_failed", format!("{dog_id}: {}", &error[..error.len().min(100)]))),
                                KernelEvent::DogExpired { dog_id } =>
                                    Some(("dog_expired", dog_id.clone())),
                                KernelEvent::DogDiscovered { dog_id } =>
                                    Some(("dog_discovered", dog_id.clone())),
                                KernelEvent::StorageReconnected =>
                                    Some(("storage_reconnected", String::new())),
                                KernelEvent::Anomaly { kind, message, .. } =>
                                    Some(("anomaly", format!("{kind}: {}", &message[..message.len().min(100)]))),
                                _ => None,
                            };
                            if let Some((tool, context)) = self_obs {
                                let obs = crate::domain::storage::Observation {
                                    project: "CYNIC".into(),
                                    agent_id: "kernel".into(),
                                    tool: tool.into(),
                                    target: "self".into(),
                                    domain: "kernel-lifecycle".into(),
                                    status: "event".into(),
                                    context,
                                    session_id: String::new(),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    tags: vec!["kernel-self-obs".into()],
                                };
                                // Fire-and-forget — don't block event processing.
                                let storage = Arc::clone(&rest_state.storage);
                                tokio::spawn(async move {
                                    let _ = storage.store_observation(&obs).await;
                                });
                            }

                            // K15: Anomaly → Slack alert (acting consumer)
                            if let KernelEvent::Anomaly { kind, message, severity } = evt
                                && let Some(ref alerter) = slack
                            {
                                let alert = format!(
                                    "🔺 CYNIC Anomaly [{severity}]: {kind} — {message}"
                                );
                                let _ = alerter.send(&alert).await;
                            }
                            if let KernelEvent::ContractDelta { missing, expected, fulfilled } = evt {
                                    if !fulfilled {
                                        // Track how long each Dog has been missing
                                        let now = std::time::Instant::now();
                                        for dog_id in missing {
                                            missing_since.entry(dog_id.clone()).or_insert(now);
                                        }
                                        missing_since.retain(|id, _| missing.contains(id));

                                        // Auto-prune: if a Dog is missing for > 10 min, remove from contract
                                        let to_prune: Vec<String> = missing_since
                                            .iter()
                                            .filter(|(_, since)| since.elapsed() > std::time::Duration::from_secs(600))
                                            .map(|(id, _)| id.clone())
                                            .collect();

                                        for dog_id in to_prune {
                                            let mut guard = rest_state.system_contract.write().unwrap_or_else(|e| e.into_inner());
                                            if guard.prune(&dog_id) {
                                                klog!("[Sovereignty] Dog '{}' missing > 10m, pruned from SystemContract", dog_id);
                                                missing_since.remove(&dog_id);
                                            }
                                        }

                                        if last_fulfilled != Some(false) {
                                            if let Some(ref alerter) = slack {
                                                let message = crate::infra::alerts::format_contract_delta(
                                                    missing,
                                                    *expected,
                                                    *expected - missing.len(),
                                                );
                                                let _ = alerter.send(&message).await;
                                            }
                                            last_fulfilled = Some(false);
                                        }
                                    } else {
                                        missing_since.clear();
                                        last_fulfilled = Some(true);
                                    }
                            }
                        }
                        Err(_) => break,
                    }
                    task_health.touch_event_consumer();
                }
                _ = liveness.tick() => {
                    task_health.touch_event_consumer();
                }
            }
        }
    })
}

pub fn spawn_probe_scheduler(
    probes: Vec<Arc<dyn Probe>>,
    _storage: Arc<dyn StoragePort>,
    environment: Arc<std::sync::RwLock<Option<EnvironmentSnapshot>>>,
    organ: Arc<crate::organ::InferenceOrgan>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    use crate::infra::probes::ProbeScheduler;

    let mut scheduler = ProbeScheduler::new(probes);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(constants::PROBE_SCHEDULER_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick (consistent with all other tasks)

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
        let mut interval = tokio::time::interval(constants::DOG_TTL_CHECK_INTERVAL);
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

        let mut interval = tokio::time::interval(constants::DISCOVERY_INTERVAL);
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
                            constants::DEFAULT_HTTP_TIMEOUT,
                            async {
                                match reqwest::Client::new()
                                    .get(&models_url)
                                    .timeout(constants::DEFAULT_HTTP_TIMEOUT)
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
                            .unwrap_or_else(|_| constants::DEFAULT_REST_ADDR.to_string());
                        let register_endpoint = format!("{register_url}/dogs/register");

                        let payload = serde_json::json!({
                            "name": dog_name,
                            "base_url": base_url,
                            "model": expected_model,
                            "context_size": ctx_size,
                            "timeout_secs": constants::DEFAULT_REGISTRATION_TTL,
                            "api_key": api_key,
                        });

                        let api_key = rest_state.api_key.clone();
                        match tokio::time::timeout(
                            constants::REGISTRATION_TIMEOUT,
                            async {
                                let mut req = reqwest::Client::new()
                                    .post(&register_endpoint)
                                    .json(&payload)
                                    .timeout(constants::REGISTRATION_TIMEOUT);

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
                    let delta = {
                        let guard = rest_state.system_contract.read().unwrap_or_else(|e| e.into_inner());
                        guard.assess(&live_ids)
                    };
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
        let mut interval = tokio::time::interval(constants::DOG_HEARTBEAT_INTERVAL);
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
                                elapsed > dog.ttl_secs.saturating_sub(constants::HEARTBEAT_TTL_MARGIN)
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

/// Crystal challenge loop — re-judge oldest crystallized crystals without injection.
/// If Q-Score delta > φ⁻² (0.382), dissolve the crystal (DEBT-A2: prevents write-once poisoning).
///
/// Runs every 300s to re-evaluate aged crystals. This is the immune system that
/// detects when crystallized knowledge becomes false over time.
pub fn spawn_crystal_challenge_loop(
    judge: Arc<crate::judge::Judge>,
    storage: Arc<dyn StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Challenge interval: 5 min. Balances responsiveness vs compute cost.
        let mut interval = tokio::time::interval(constants::CRYSTAL_CHALLENGE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Crystal challenge loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    // K15: background task that challenges oldest Crystallized/Canonical crystals
                    match tokio::time::timeout(
                        constants::CRYSTAL_CHALLENGE_TIMEOUT,
                        challenge_one_crystal(&judge, &storage),
                    )
                    .await
                    {
                        Ok(Ok(Some(dissolved_id))) => {
                            tracing::info!(
                                crystal_id = %dissolved_id,
                                "crystal challenge: Q-delta > φ⁻² — dissolved"
                            );
                            task_health.touch_crystal_challenge();
                        }
                        Ok(Ok(None)) => {
                            // No crystallized crystals to challenge, or all are stable
                            task_health.touch_crystal_challenge();
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(error = %e, "crystal challenge failed");
                            task_health.touch_crystal_challenge();
                        }
                        Err(_) => {
                            tracing::warn!("crystal challenge timed out (60s)");
                        }
                    }
                }
            }
        }
    })
}

/// Challenge one crystal: re-judge without injection, compare Q-scores.
/// Returns Some(crystal_id) if dissolved, None if stable.
async fn challenge_one_crystal(
    judge: &Arc<crate::judge::Judge>,
    storage: &Arc<dyn StoragePort>,
) -> Result<Option<String>, String> {
    use crate::domain::metrics::Metrics;

    // Step 1: fetch oldest Crystallized/Canonical crystal
    let crystal = storage
        .list_crystals_filtered(
            constants::CRYSTAL_CHALLENGE_BATCH,
            None,
            Some("crystallized"),
        )
        .await
        .map_err(|e| format!("failed to list crystals: {e}"))?
        .into_iter()
        .chain(
            storage
                .list_crystals_filtered(constants::CRYSTAL_CHALLENGE_BATCH, None, Some("canonical"))
                .await
                .unwrap_or_default(),
        )
        .min_by_key(|c| c.created_at.clone()) // oldest first
        .ok_or_else(|| "no crystallized crystals to challenge".to_string())?;

    let original_q = crystal.confidence;

    // Step 2: re-judge the crystal's content without injection
    let stimulus = crate::domain::dog::Stimulus {
        content: crystal.content.clone(),
        context: None,
        domain: Some(crystal.domain.clone()),
        request_id: None,
    };

    let metrics = Metrics::new();
    let verdict = match judge.evaluate(&stimulus, None, &metrics).await {
        Ok(v) => v,
        Err(e) => return Err(format!("failed to re-judge crystal: {e}")),
    };

    let new_q = verdict.q_score.total;
    let q_delta = (original_q - new_q).abs();

    // Step 3: if delta > φ⁻² (0.382), the crystal's knowledge has degraded significantly
    use crate::domain::dog::PHI_INV2;
    if q_delta > PHI_INV2 {
        // Record a degraded observation with the new score to transition crystal state
        let timestamp = Utc::now().to_rfc3339();
        storage
            .observe_crystal(
                &crystal.id,
                &crystal.content,
                &crystal.domain,
                new_q,
                &timestamp,
                verdict.voter_count,
                &verdict.id,
                format!("{:?}", verdict.kind).as_str(),
            )
            .await
            .map_err(|e| format!("failed to degrade crystal via observation: {e}"))?;

        Ok(Some(crystal.id))
    } else {
        // Crystal is still valid
        Ok(None)
    }
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

    fn test_app_state(event_tx: &tokio::sync::broadcast::Sender<KernelEvent>) -> Arc<AppState> {
        let judge = crate::judge::Judge::new(vec![], vec![]);
        Arc::new(AppState {
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
            system_contract: Arc::new(std::sync::RwLock::new(
                crate::domain::contract::SystemContract::new(vec![], true),
            )),
            enricher: None,
        })
    }

    #[tokio::test]
    async fn event_consumer_receives_and_shuts_down() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<KernelEvent>(16);
        let rest_state = test_app_state(&tx);
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle =
            spawn_event_consumer(&tx, rest_state, task_health.clone(), shutdown.clone(), None);

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

        let rest_state = test_app_state(&tx);
        let handle = spawn_event_consumer_with_liveness(
            &tx,
            rest_state,
            task_health.clone(),
            shutdown.clone(),
            Duration::from_millis(100),
            None,
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
        let rest_state = test_app_state(&event_tx);
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
