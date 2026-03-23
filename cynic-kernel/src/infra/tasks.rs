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
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::health_gate::HealthGate;
use crate::infra::config::BackendRemediation;
use crate::domain::metrics::Metrics;
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
    ).await {
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

// ── CCM Workflow Aggregator (configurable interval) ──────────

pub fn spawn_ccm_aggregator(
    storage: Arc<dyn StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let interval_secs: u64 = std::env::var("CYNIC_AGGREGATE_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] CCM aggregator stopped");
                    break;
                }
                _ = interval.tick() => {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        crate::domain::ccm::aggregate_observations(storage.as_ref(), "CYNIC"),
                    ).await {
                        Ok(count) => {
                            let detail = if count > 0 { "active" } else { "idle:0" };
                            task_health.touch_ccm_aggregate(detail);
                            if count > 0 {
                                klog!("[CCM] aggregated {} patterns", count);
                            }
                        }
                        Err(_) => tracing::warn!("CCM aggregate_observations timed out (30s)"),
                    }
                }
            }
        }
    });
    klog!("[Ring 2] CCM workflow aggregator started (every {}s)", interval_secs);
    handle
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
                        task_health.touch_summarizer("llm_unavailable");
                        continue;
                    }
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(120),
                        crate::pipeline::summarize_pending_sessions(storage.as_ref(), &summarizer),
                    ).await {
                        Ok(count) => {
                            let detail = if count > 0 { "producing" } else { "idle:0_pending" };
                            if count > 0 {
                                klog!("[CCM/summarizer] {} sessions summarized", count);
                            }
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
        task_health.touch_backfill("running");
        match tokio::time::timeout(
            std::time::Duration::from_secs(300),
            crate::pipeline::backfill_crystal_embeddings(storage.as_ref(), embedding.as_ref(), &metrics),
        ).await {
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

pub fn spawn_introspection(
    storage: Arc<dyn StoragePort>,
    metrics: Arc<Metrics>,
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
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        crate::introspection::analyze(storage.as_ref(), &metrics),
                    ).await {
                        Ok(alerts) => {
                            for alert in &alerts {
                                let _ = event_tx.send(KernelEvent::Anomaly {
                                    kind: alert.kind.to_string(),
                                    message: alert.message.clone(),
                                    severity: alert.severity.to_string(),
                                });
                            }
                            if let Ok(mut stored) = introspection_alerts.write() {
                                *stored = alerts;
                            }
                            task_health.touch_introspection();
                        }
                        Err(_) => tracing::warn!("introspection analyze timed out (30s)"),
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
    klog!("[Ring 2] Remediation watcher started (90s threshold, {} Dogs)", dog_count);
    handle
}

// ── Rate limiter eviction (REST-only, every 60s) ─────────────
// Spawned inline in main.rs — REST delivery concern, not infra.
// The rate limiters live in api::rest::AppState and stay there.

pub fn spawn_rate_eviction(
    rest_state: Arc<crate::api::rest::AppState>,
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
                    klog!("[SHUTDOWN] Rate limiter eviction stopped");
                    break;
                }
                _ = interval.tick() => {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        rest_state.rate_limiter.evict_stale(),
                    ).await {
                        Ok(()) => {}
                        Err(_) => tracing::warn!("rate_limiter evict_stale timed out (5s)"),
                    }
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        rest_state.judge_limiter.evict_stale(),
                    ).await {
                        Ok(()) => {}
                        Err(_) => tracing::warn!("judge_limiter evict_stale timed out (5s)"),
                    }
                    task_health.touch_rate_eviction();
                }
            }
        }
    })
}
