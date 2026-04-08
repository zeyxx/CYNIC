//! Background task spawns — extracted from main.rs for composition root clarity.
//!
//! Each function takes its dependencies as parameters and spawns a tokio task.
//! Returns `JoinHandle<()>` — caller can collect into a `JoinSet` for structured concurrency.
//! All tasks respect the CancellationToken for graceful shutdown.
//! All `.await` inside spawns are wrapped in `tokio::time::timeout()` (Rule #10).

use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::coord::CoordPort;
use crate::domain::events::KernelEvent;
use crate::domain::health_gate::HealthGate;
use crate::domain::metrics::Metrics;
use crate::domain::probe::EnvironmentSnapshot;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::infra::config::BackendRemediation;
use crate::infra::task_health::TaskHealth;

// ── Shutdown flush — used by both REST and MCP exit paths ────

pub async fn flush_usage_on_shutdown(
    storage: &Arc<dyn StoragePort>,
    usage: &Arc<tokio::sync::Mutex<DogUsageTracker>>,
    has_db: bool,
) {
    if !has_db {
        klog!("[SHUTDOWN] No DB — skipping flush");
        return;
    }
    let (snapshot, dog_count) = {
        let u = usage.lock().await;
        (u.flush_snapshot(), u.dogs.len())
    };
    if snapshot.is_empty() {
        klog!("[SHUTDOWN] No pending usage to flush");
        return;
    }
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        storage.flush_usage(&snapshot),
    )
    .await
    {
        Ok(Ok(_)) => {
            let mut u = usage.lock().await;
            u.absorb_flush();
            klog!("[SHUTDOWN] Usage flushed ({} dogs)", dog_count);
        }
        Ok(Err(e)) => tracing::warn!(error = %e, "shutdown usage flush failed"),
        Err(_) => tracing::warn!("shutdown usage flush timed out (10s)"),
    }
}

// ── Coordination expiry (every 60s) ──────────────────────────

pub fn spawn_coord_expiry(
    coord: Arc<dyn CoordPort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Coord expiry stopped");
                    break;
                }
                _ = interval.tick() => {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        coord.expire_stale(),
                    ).await {
                        Ok(Err(e)) => tracing::warn!(error = %e, "coord expire_stale failed"),
                        Err(_) => tracing::warn!("coord expire_stale timed out (10s)"),
                        _ => {}
                    }
                    task_health.touch_coord_expiry();
                }
            }
        }
    })
}

// ── Usage flush (every 60s, TTL cleanup every 1h) ────────────

pub fn spawn_usage_flush(
    storage: Arc<dyn StoragePort>,
    usage: Arc<tokio::sync::Mutex<DogUsageTracker>>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    spawn_usage_flush_with_organ(storage, usage, task_health, shutdown, None)
}

/// Usage flush with optional organ stats persistence (B5 amnesia fix).
pub fn spawn_usage_flush_with_organ(
    storage: Arc<dyn StoragePort>,
    usage: Arc<tokio::sync::Mutex<DogUsageTracker>>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
    organ: Option<Arc<crate::organ::InferenceOrgan>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        let mut tick_count: u64 = 0;
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Usage flush stopped");
                    break;
                }
                _ = interval.tick() => {
                    tick_count += 1;
                    let snapshot = match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        usage.lock(),
                    ).await {
                        Ok(u) => u.flush_snapshot(),
                        Err(_) => { tracing::warn!("usage lock timed out (5s) for snapshot"); continue; }
                    };
                    if !snapshot.is_empty() {
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(10),
                            storage.flush_usage(&snapshot),
                        ).await {
                            Ok(Ok(_)) => {
                                match tokio::time::timeout(
                                    std::time::Duration::from_secs(5),
                                    usage.lock(),
                                ).await {
                                    Ok(mut u) => u.absorb_flush(),
                                    Err(_) => tracing::warn!("usage lock timed out (5s) for absorb_flush"),
                                }
                            }
                            Ok(Err(e)) => tracing::warn!(error = %e, "usage flush DB write failed, will retry"),
                            Err(_) => tracing::warn!("usage flush DB write timed out (10s), will retry"),
                        }
                    }
                    // B5: flush DogStats to DB (organ quality persistence)
                    if let Some(ref org) = organ {
                        let stats = org.snapshot_stats();
                        if !stats.is_empty() {
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(10),
                                storage.flush_dog_stats(&stats),
                            ).await {
                                Ok(Err(e)) => tracing::warn!(error = %e, "dog_stats flush failed (non-fatal)"),
                                Err(_) => tracing::warn!("dog_stats flush timed out (10s)"),
                                _ => {}
                            }
                        }
                    }
                    if tick_count.is_multiple_of(60) {
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(30),
                            storage.cleanup_ttl(),
                        ).await {
                            Ok(Err(e)) => tracing::warn!(error = %e, "TTL cleanup failed (non-fatal)"),
                            Err(_) => tracing::warn!("TTL cleanup timed out (30s)"),
                            _ => {}
                        }
                    }
                    task_health.touch_usage_flush();
                }
            }
        }
    })
}

// ── Session summarizer (sovereign inference, background) ─────

pub fn spawn_session_summarizer(
    storage: Arc<dyn StoragePort>,
    summarizer: crate::backends::summarizer::SovereignSummarizer,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    klog!("[Ring 2] Session summarizer task started (checks LLM availability each cycle)");
    tokio::spawn(async move {
        // Wait for first CCM cycle — but break early on shutdown
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(std::time::Duration::from_secs(600)) => {}
        }
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Session summarizer stopped");
                    break;
                }
                _ = interval.tick() => {
                    // Timeout on health check — prevents 60s stall on unresponsive LLM
                    let available = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        summarizer.is_available(),
                    ).await.unwrap_or(false);
                    if !available {
                        tracing::info!(organ = "summarizer", result = "llm_unavailable", "summarizer cycle: LLM not reachable — skipping");
                        task_health.touch_summarizer("llm_unavailable");
                        continue;
                    }
                    let start = std::time::Instant::now();
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(120),
                        crate::pipeline::summarize_pending_sessions(storage.as_ref(), &summarizer),
                    ).await {
                        Ok(count) => {
                            let detail = if count > 0 { "producing" } else { "idle:0_pending" };
                            tracing::info!(
                                organ = "summarizer",
                                result = detail,
                                sessions_summarized = count,
                                duration_ms = start.elapsed().as_millis() as u64,
                                "summarizer cycle complete"
                            );
                            task_health.touch_summarizer(detail);
                        }
                        Err(_) => tracing::warn!("session summarizer timed out (120s)"),
                    }
                }
            }
        }
    })
}

// ── Crystal embedding backfill (one-shot) ────────────────────

pub fn spawn_backfill(
    storage: Arc<dyn StoragePort>,
    embedding: Arc<dyn crate::domain::embedding::EmbeddingPort>,
    metrics: Arc<Metrics>,
    task_health: Arc<TaskHealth>,
    event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let handle = tokio::spawn(async move {
        // Delay 10s — let embedding server warm up, but respect shutdown
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {}
        }
        // ── Crystal consolidation: merge duplicate crystals before backfill ──
        // Runs once per boot. Finds crystals with identical (domain, content)
        // and merges observation counts + confidence into the survivor.
        // Must run BEFORE backfill so orphan embeddings are computed on merged crystals.
        task_health.touch_backfill("consolidating");
        match tokio::time::timeout(
            std::time::Duration::from_secs(60),
            storage.consolidate_duplicate_crystals(),
        )
        .await
        {
            Ok(Ok(removed)) => {
                if removed > 0 {
                    klog!("[Ring 2] Crystal consolidation: merged {removed} duplicate crystals");
                }
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "crystal consolidation failed (non-fatal)");
            }
            Err(_) => {
                tracing::warn!("crystal consolidation timed out (60s)");
            }
        }

        task_health.touch_backfill("running");
        match tokio::time::timeout(
            std::time::Duration::from_secs(300),
            crate::pipeline::backfill_crystal_embeddings(
                storage.as_ref(),
                embedding.as_ref(),
                &metrics,
            ),
        )
        .await
        {
            Ok(count) => {
                let detail = if count > 0 { "done" } else { "clean:0_orphans" };
                task_health.touch_backfill(detail);
                if count > 0 {
                    klog!("[Ring 2] Backfill: embedded {} orphan crystals", count);
                    let _ = event_tx.send(KernelEvent::BackfillComplete { count });
                }
            }
            Err(_) => {
                task_health.touch_backfill("timeout");
                tracing::warn!("crystal backfill timed out (300s)");
            }
        }
    });
    klog!("[Ring 2] Crystal embedding backfill task scheduled");
    handle
}

// ── Introspection loop (MAPE-K Analyze, every 5 min) ────────

#[allow(clippy::too_many_arguments)]
// WHY: spawn_introspection threads through all kernel Arc dependencies so the background
// introspection task has access to storage, metrics, judge, embedding, usage, verdict_cache,
// alerts, and the event channel. Each is a distinct subsystem — collapsing them into a struct
// would create a god-object coupling every subsystem to the introspection task scheduler.
pub fn spawn_introspection(
    storage: Arc<dyn StoragePort>,
    metrics: Arc<Metrics>,
    environment: Arc<std::sync::RwLock<Option<EnvironmentSnapshot>>>,
    introspection_alerts: Arc<std::sync::RwLock<Vec<crate::introspection::Alert>>>,
    event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let handle = tokio::spawn(async move {
        // Wait 60s for system to stabilize before first introspection
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
        }
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Introspection loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    let env_snap = match environment.read() {
                        Ok(guard) => guard.clone(),
                        Err(e) => {
                            tracing::warn!(error = %e, "environment RwLock poisoned — skipping resource checks");
                            None
                        }
                    };
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(180),
                        crate::introspection::analyze(
                            storage.as_ref(),
                            &metrics,
                            &env_snap,
                        ),
                    ).await {
                        Ok(alerts) => {
                            let alert_count = alerts.len();
                            let critical = alerts.iter().filter(|a| a.severity == "critical").count();
                            tracing::info!(
                                organ = "introspection",
                                alerts = alert_count,
                                critical = critical,
                                "introspection cycle: {} alerts ({} critical)",
                                alert_count, critical
                            );
                            for alert in &alerts {
                                let _ = event_tx.send(KernelEvent::Anomaly {
                                    kind: alert.kind.to_string(),
                                    message: alert.message.clone(),
                                    severity: alert.severity.to_string(),
                                });
                            }
                            match introspection_alerts.write() {
                                Ok(mut stored) => *stored = alerts,
                                Err(e) => tracing::warn!(error = %e, "introspection_alerts RwLock poisoned — alerts not updated"),
                            }
                            task_health.touch_introspection();
                        }
                        Err(_) => tracing::warn!("introspection analyze timed out (180s)"),
                    }
                }
            }
        }
    });
    klog!("[Ring 2] Introspection loop started (every 5min, 60s warmup)");
    handle
}

// ── Signal handler (SIGINT + SIGTERM → cancel shutdown token) ─

pub fn spawn_signal_handler(shutdown: CancellationToken) -> JoinHandle<()> {
    tokio::spawn(async move {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        klog!("[SHUTDOWN] SIGINT received — draining in-flight requests");
                    }
                    _ = sigterm.recv() => {
                        klog!("[SHUTDOWN] SIGTERM received — draining in-flight requests");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "SIGTERM handler unavailable — waiting for SIGINT only");
                let _ = tokio::signal::ctrl_c().await;
                klog!("[SHUTDOWN] SIGINT received — draining in-flight requests");
            }
        }
        shutdown.cancel();
    })
}

// ── Remediation watcher (every 30s check) ────────────────────

pub fn spawn_remediation_watcher(
    configs: std::collections::HashMap<String, BackendRemediation>,
    breakers: Vec<Arc<dyn HealthGate>>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let dog_count = configs.len();
    let handle = tokio::spawn(async move {
        let tracker = crate::infra::remediation::RecoveryTracker::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Remediation watcher stopped");
                    break;
                }
                _ = interval.tick() => {
                    for cb in &breakers {
                        let dog_id = cb.dog_id();
                        if let Some(open_duration) = cb.opened_since() {
                            const REMEDIATION_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(90);
                            if open_duration > REMEDIATION_THRESHOLD
                                && let Some(config) = configs.get(dog_id)
                                && tracker.should_restart(dog_id, config)
                            {
                                klog!(
                                    "[Remediation] Dog '{}' open for {:.0}s, attempting restart on {}",
                                    dog_id, open_duration.as_secs_f64(), config.node
                                );
                                let node = config.node.clone();
                                let cmd = config.restart_command.clone();
                                match tokio::time::timeout(
                                    std::time::Duration::from_secs(15),
                                    tokio::task::spawn_blocking(move || {
                                        crate::infra::remediation::ssh_restart(&node, &cmd)
                                    }),
                                ).await {
                                    Ok(Ok(Ok(output))) => {
                                        klog!("[Remediation] Dog '{}' restart initiated: {}", dog_id, output.trim());
                                    }
                                    Ok(Ok(Err(e))) => {
                                        klog!("[Remediation] Dog '{}' restart failed: {}", dog_id, e);
                                    }
                                    _ => {
                                        klog!("[Remediation] Dog '{}' restart timed out or panicked", dog_id);
                                    }
                                }
                                tracker.record_attempt(dog_id, config.max_retries);
                            }
                        } else {
                            tracker.reset(dog_id);
                        }
                    }
                    task_health.touch_remediation();
                }
            }
        }
    });
    klog!(
        "[Ring 2] Remediation watcher started (90s threshold, {} Dogs)",
        dog_count
    );
    handle
}

// ── Event bus consumer (internal reactions) ──────────────────

/// How often the event consumer touches `task_health` even when the bus is
/// idle. Must be strictly less than half `task_health`'s `event_consumer`
/// expected_interval (300 s → stale at 600 s) so an idle but alive consumer
/// never drifts into false-stale. 60 s matches the other liveness-style
/// loops in this file (`coord_expiry`, `usage_flush`).
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
        // Idle-liveness tick: the event bus may be quiet for hours (observed
        // 8+ h in prod on 2026-04-08). Before this branch existed, task_health
        // could not distinguish "alive and idle" from "dead" and marked the
        // consumer stale after ~600 s, degrading /health on a perfectly
        // healthy kernel. Same K15 class as the backfill false-stale fix,
        // but here the task is genuinely periodic — we just keep its
        // liveness signal alive during quiet periods.
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
                        Ok(_) => {
                            // Events are logged at emission (pipeline.rs, coord.rs).
                            // Metrics are counted at origin (judge.rs, introspection.rs).
                            // This consumer exists for: lag detection + task health liveness.
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

// Rate limiter eviction moved to main.rs — REST delivery concern, not infra.

// ── Probe scheduler (every 10s, 7-day TTL cleanup) ───────────

pub fn spawn_probe_scheduler(
    probes: Vec<Arc<dyn crate::domain::probe::Probe>>,
    storage: Arc<dyn StoragePort>,
    environment: Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
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
                        // Update in-memory state for /health
                        if let Ok(mut env) = environment.write() {
                            *env = Some(snapshot.clone());
                        }

                        // Persist to DB for trend detection (Rule 3: consumer = future linear regression on disk/memory).
                        // list_infra_snapshots() exists in StoragePort but has no caller yet — tracked as B-item.
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            storage.store_infra_snapshot(&snapshot),
                        ).await {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => tracing::warn!("probe snapshot storage failed: {e}"),
                            Err(_) => tracing::warn!("probe snapshot storage timed out"),
                        }

                        // Fleet probe → organ: degrade backends with model mismatch,
                        // promote backends where mismatch is resolved + gate clear.
                        for probe in &snapshot.probes {
                            if let crate::domain::probe::ProbeDetails::Fleet(ref fleet) = probe.details {
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

                    // TTL cleanup every 100 ticks (~17 min)
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

// ── Dog TTL checker (every 30s) ──────────────────────────────

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
                    // Collect expired dog IDs under a short read lock
                    let expired: Vec<String> = match rest_state.registered_dogs.read() {
                        Ok(map) => map
                            .iter()
                            .filter(|(_, dog)| {
                                dog.last_heartbeat.elapsed().as_secs() > dog.ttl_secs
                            })
                            .map(|(id, _)| id.clone())
                            .collect(),
                        Err(e) => {
                            tracing::warn!(error = %e, "registered_dogs RwLock poisoned — skipping TTL check");
                            vec![]
                        }
                    };

                    for dog_id in expired {
                        // Remove from registered map
                        match rest_state.registered_dogs.write() {
                            Ok(mut map) => { map.remove(&dog_id); }
                            Err(e) => {
                                tracing::warn!(error = %e, dog_id = %dog_id, "registered_dogs write lock poisoned — cannot remove expired dog");
                                continue;
                            }
                        }

                        // Swap judge without the expired dog
                        let current = rest_state.judge.load_full();
                        if let Some(new_judge) = crate::judge::Judge::without_dog(&current, &dog_id) {
                            rest_state.judge.store(Arc::new(new_judge));
                            tracing::warn!(dog_id = %dog_id, "Dog TTL expired — removed from active roster");
                        }

                        // Emit event (fire-and-forget — lagging subscribers just miss it)
                        let _ = rest_state.event_tx.send(crate::domain::events::KernelEvent::DogExpired {
                            dog_id: dog_id.clone(),
                        });
                    }

                    task_health.touch_dog_ttl();
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::coord::NullCoord;
    use crate::domain::events::KernelEvent;
    use crate::domain::storage::NullStorage;
    use crate::domain::usage::DogUsageTracker;
    use crate::infra::task_health::TaskHealth;

    #[tokio::test]
    async fn flush_usage_on_shutdown_skips_when_no_db() {
        let storage: Arc<dyn StoragePort> = Arc::new(NullStorage);
        let usage = Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new()));
        // has_db=false → should skip immediately without error
        flush_usage_on_shutdown(&storage, &usage, false).await;
    }

    #[tokio::test]
    async fn flush_usage_on_shutdown_skips_when_empty() {
        let storage: Arc<dyn StoragePort> = Arc::new(NullStorage);
        let usage = Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new()));
        // has_db=true but no usage recorded → should skip
        flush_usage_on_shutdown(&storage, &usage, true).await;
    }

    #[tokio::test]
    async fn flush_usage_on_shutdown_attempts_flush_when_data_present() {
        let storage: Arc<dyn StoragePort> = Arc::new(NullStorage);
        let usage = Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new()));
        // Record some usage
        {
            let mut u = usage.lock().await;
            u.record("test-dog", 100, 50, 200);
        }
        // NullStorage will return error on flush, but function should not panic
        flush_usage_on_shutdown(&storage, &usage, true).await;
    }

    #[tokio::test]
    async fn coord_expiry_respects_shutdown() {
        let coord: Arc<dyn CoordPort> = Arc::new(NullCoord);
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_coord_expiry(coord, task_health, shutdown.clone());
        // Cancel immediately
        shutdown.cancel();
        // Task should exit within a reasonable time
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("coord_expiry should stop within 2s")
            .expect("task should not panic");
    }

    #[tokio::test]
    async fn usage_flush_respects_shutdown() {
        let storage: Arc<dyn StoragePort> = Arc::new(NullStorage);
        let usage = Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new()));
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_usage_flush(storage, usage, task_health, shutdown.clone());
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("usage_flush should stop within 2s")
            .expect("task should not panic");
    }

    #[tokio::test]
    async fn event_consumer_receives_and_shuts_down() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<KernelEvent>(16);
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_event_consumer(&tx, task_health.clone(), shutdown.clone());

        // Send an event — should not panic
        let _ = tx.send(KernelEvent::VerdictIssued {
            verdict_id: "test-v1".into(),
            domain: "test".into(),
            verdict: "Wag".into(),
            q_score: 0.5,
        });

        // Give consumer time to process
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify task_health was touched
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

        // Short liveness interval so the test doesn't wait a minute.
        // Production uses EVENT_CONSUMER_LIVENESS_INTERVAL (60 s).
        let handle = spawn_event_consumer_with_liveness(
            &tx,
            task_health.clone(),
            shutdown.clone(),
            Duration::from_millis(100),
        );

        // Never send an event. Wait for >= 2 liveness ticks (scheduler jitter
        // margin included).
        tokio::time::sleep(Duration::from_millis(350)).await;

        // Consumer must have touched task_health purely from the idle branch.
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
    async fn remediation_watcher_respects_shutdown() {
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        // Empty configs + breakers — watcher should still start and stop cleanly
        let handle = spawn_remediation_watcher(
            std::collections::HashMap::new(),
            vec![],
            task_health,
            shutdown.clone(),
        );
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("remediation_watcher should stop within 2s")
            .expect("task should not panic");
    }

    #[tokio::test]
    async fn probe_scheduler_respects_shutdown() {
        let probes: Vec<Arc<dyn crate::domain::probe::Probe>> =
            vec![Arc::new(crate::domain::probe::NullProbe)];
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
        use crate::api::rest::{AppState, PerIpRateLimiter, ReadyCache, StorageInfo};
        use crate::domain::coord::NullCoord;
        use crate::domain::embedding::NullEmbedding;
        use crate::domain::usage::DogUsageTracker;
        use crate::domain::verdict_cache::VerdictCache;
        use arc_swap::ArcSwap;

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
