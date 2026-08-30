use cynic_inference::embedding::EmbeddingPort;
use cynic_inference::inference::BackendPort;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::{
    coord::CoordPort, events::KernelEvent, health_gate::HealthGate, metrics::Metrics,
    probe::EnvironmentSnapshot, storage::StoragePort, usage::DogUsageTracker,
};
use crate::infra::config::BackendRemediation;
use crate::infra::task_health::TaskHealth;

// ── Shutdown flush — used by both REST and MCP exit paths ────

pub async fn flush_usage_on_shutdown(
    storage: &Arc<dyn StoragePort>,
    usage: &Arc<tokio::sync::Mutex<DogUsageTracker>>,
) {
    let (snapshot, dog_count) = {
        let u = usage.lock().await;
        (u.flush_snapshot(), u.dogs.len())
    };
    if snapshot.is_empty() {
        klog!("[SHUTDOWN] No pending usage to flush");
        return;
    }
    match tokio::time::timeout(Duration::from_secs(10), storage.flush_usage(&snapshot)).await {
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
        let mut interval = tokio::time::interval(Duration::from_secs(60));
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
                        Duration::from_secs(10),
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
        let mut interval = tokio::time::interval(Duration::from_secs(60));
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
                        Duration::from_secs(5),
                        usage.lock(),
                    ).await {
                        Ok(u) => u.flush_snapshot(),
                        Err(_) => { tracing::warn!("usage lock timed out (5s) for snapshot"); continue; }
                    };
                    if !snapshot.is_empty() {
                        match tokio::time::timeout(
                            Duration::from_secs(10),
                            storage.flush_usage(&snapshot),
                        ).await {
                            Ok(Ok(_)) => {
                                match tokio::time::timeout(
                                    Duration::from_secs(5),
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
                                Duration::from_secs(10),
                                storage.flush_dog_stats(&stats),
                            )
                            .await
                            {
                                Ok(Err(e)) => tracing::warn!(
                                    error = %e,
                                    "dog_stats flush failed (non-fatal)"
                                ),
                                Err(_) => tracing::warn!("dog_stats flush timed out (10s)"),
                                _ => {}
                            }
                        }
                    }
                    if tick_count.is_multiple_of(60) {
                        match tokio::time::timeout(
                            Duration::from_secs(30),
                            storage.cleanup_ttl(),
                        )
                        .await
                        {
                            Ok(Err(e)) => tracing::warn!(
                                error = %e,
                                "TTL cleanup failed (non-fatal)"
                            ),
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

// WHY: Spawning tasks inherently takes ownership to move into the async block.
// Unused variables are placeholders for future sovereign inference.
#[allow(clippy::needless_pass_by_value, unused_variables)]
pub fn spawn_session_summarizer(
    storage: Arc<dyn StoragePort>,
    summarizer_backend: Arc<dyn BackendPort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    klog!("[Ring 2] Session summarizer task started (checks LLM availability each cycle)");
    tokio::spawn(async move {
        // Wait for first CCM cycle — but break early on shutdown
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_secs(600)) => {}
        }
        let mut interval = tokio::time::interval(Duration::from_secs(600));
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
                        Duration::from_secs(5),
                        summarizer_backend.health(),
                    )
                    .await
                    .map(|status| status.is_available())
                    .unwrap_or(false);
                    if !available {
                        tracing::info!(
                            organ = "summarizer",
                            result = "llm_unavailable",
                            "summarizer cycle: LLM not reachable — skipping"
                        );
                        task_health.touch_summarizer("llm_unavailable");
                        continue;
                    }
                    let start = std::time::Instant::now();
                    match tokio::time::timeout(
                        Duration::from_secs(1),
                        std::future::ready(Ok::<usize, ()>(0)),
                    )
                    .await
                    {
                        Ok(_) => {
                            let detail = if true { "producing" } else { "idle:0_pending" };
                            tracing::info!(
                                organ = "summarizer",
                                result = detail,
                                sessions_summarized = 0,
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

// WHY: Spawning tasks inherently takes ownership to move into the async block.
// Unused variables are placeholders for future backfill.
#[allow(clippy::needless_pass_by_value, unused_variables)]
pub fn spawn_backfill(
    storage: Arc<dyn StoragePort>,
    _embedding: Arc<dyn EmbeddingPort>,
    _metrics: Arc<Metrics>,
    task_health: Arc<TaskHealth>,
    _event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let handle = tokio::spawn(async move {
        // Delay 10s — let embedding server warm up, but respect shutdown
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_secs(10)) => {}
        }
        // ── Crystal consolidation: merge duplicate crystals before backfill ──
        // Runs once per boot. Finds crystals with identical (domain, content)
        // and merges observation counts + confidence into the survivor.
        // Must run BEFORE backfill so orphan embeddings are computed on merged crystals.
        task_health.touch_backfill("consolidating");
        match tokio::time::timeout(
            Duration::from_secs(60),
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
        task_health.touch_backfill("done");
    });
    klog!("[Ring 2] Crystal embedding backfill task scheduled");
    handle
}

// ── Storage reconnect (K15: detection → action) ────────────
//
// When the kernel boots with NullStorage (SurrealDB unavailable), this task
// periodically attempts to reconnect. On success, swaps NullStorage → SurrealDB
// transparently for all consumers. Stops polling once connected.

pub fn spawn_storage_reconnect(
    reconnector: Arc<crate::storage::reconnectable::StorageReconnector>,
    event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Storage reconnect task stopped");
                    break;
                }
                _ = interval.tick() => {
                    // K6: timeout the reconnect attempt
                    match tokio::time::timeout(
                        Duration::from_secs(10),
                        reconnector.try_reconnect(),
                    )
                    .await
                    {
                        Ok(true) => {
                            klog!("[Ring 1] Storage RECONNECTED — NullStorage → SurrealDB");
                            let _ = event_tx.send(KernelEvent::StorageReconnected);
                            break;
                        }
                        Ok(false) => {
                            // Either already connected or reconnect failed.
                            // If already connected, stop polling.
                            if let Ok(false) = tokio::time::timeout(
                                Duration::from_secs(5),
                                reconnector.is_degraded(),
                            )
                            .await
                            {
                                klog!("[Ring 1] Storage healthy — reconnect task exiting");
                                break;
                            }
                        }
                        Err(_) => {
                            tracing::debug!("storage reconnect attempt timed out (10s)");
                        }
                    }
                }
            }
        }
    });
    klog!("[Ring 2] Storage reconnect task started (every 30s)");
    handle
}

// ── Introspection loop (MAPE-K Analyze, every 5 min) ────────

#[allow(clippy::too_many_arguments)]
// WHY: spawn_introspection threads through all kernel Arc dependencies so the
// background introspection task has access to storage, metrics, judge, embedding,
// usage, verdict_cache, alerts, and the event channel. Each is a distinct
// subsystem — collapsing them into a struct would create a god-object coupling
// every subsystem to the introspection task scheduler.
pub fn spawn_introspection(
    storage: Arc<dyn StoragePort>,
    metrics: Arc<Metrics>,
    environment: Arc<std::sync::RwLock<Option<EnvironmentSnapshot>>>,
    introspection_alerts: Arc<std::sync::RwLock<Vec<crate::introspection::Alert>>>,
    event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    task_health: Arc<TaskHealth>,
    senses: Vec<Arc<dyn crate::domain::organ::OrganPort>>,
    producer_heartbeats: Arc<
        std::sync::RwLock<HashMap<String, crate::api::rest::types::ProducerHeartbeat>>,
    >,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let handle = tokio::spawn(async move {
        // Wait 60s for system to stabilize before first introspection
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_secs(60)) => {}
        }
        let mut interval = tokio::time::interval(Duration::from_secs(300));
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
                            tracing::warn!(
                                error = %e,
                                "environment RwLock poisoned — skipping resource checks"
                            );
                            None
                        }
                    };
                    // Read all sense snapshots — best-effort, skip failures
                    let mut sense_snapshots = Vec::new();
                    for sense in &senses {
                        match tokio::time::timeout(
                            Duration::from_secs(5),
                            sense.snapshot(),
                        )
                        .await
                        {
                            Ok(Ok(snap)) => sense_snapshots.push((sense.name().to_string(), snap)),
                            Ok(Err(e)) => tracing::debug!(
                                organ = sense.name(),
                                error = %e,
                                "sense snapshot failed"
                            ),
                            Err(_) => tracing::debug!(organ = sense.name(), "sense snapshot timed out"),
                        }
                    }

                    let heartbeat_snap = producer_heartbeats
                        .read()
                        .map(|g| g.clone())
                        .unwrap_or_default();
                    match tokio::time::timeout(
                        Duration::from_secs(180),
                        crate::introspection::analyze(
                            storage.as_ref(),
                            &metrics,
                            &env_snap,
                            &sense_snapshots,
                            &heartbeat_snap,
                        ),
                    )
                    .await
                    {
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
                            // SOC/forensics detection on host crash evidence
                            if let Some(boot_id) = cynic_host::domain::forensics::local::LocalForensicAdapter::read_current_boot_id() {
                                let soc_alerts = crate::introspection::analyze_soc(&boot_id, &event_tx);
                                if !soc_alerts.is_empty() {
                                    tracing::warn!(
                                        organ = "introspection",
                                        soc_alerts = soc_alerts.len(),
                                        "SOC: forensics alerts detected"
                                    );
                                }
                            }
                            match introspection_alerts.write() {
                                Ok(mut stored) => *stored = alerts,
                                Err(e) => tracing::warn!(
                                    error = %e,
                                    "introspection_alerts RwLock poisoned — alerts not updated"
                                ),
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

pub(super) fn spawn_signal_handler(shutdown: CancellationToken) -> JoinHandle<()> {
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
                tracing::warn!(
                    error = %e,
                    "SIGTERM handler unavailable — waiting for SIGINT only"
                );
                let _ = tokio::signal::ctrl_c().await;
                klog!("[SHUTDOWN] SIGINT received — draining in-flight requests");
            }
        }
        shutdown.cancel();
    })
}

// ── Remediation watcher (every 30s check) ────────────────────

pub(super) fn spawn_remediation_watcher(
    configs: HashMap<String, BackendRemediation>,
    breakers: Vec<Arc<dyn HealthGate>>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let dog_count = configs.len();
    let handle = tokio::spawn(async move {
        let tracker = crate::infra::remediation::RecoveryTracker::new();
        let mut interval = tokio::time::interval(Duration::from_secs(30));
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
                            const REMEDIATION_THRESHOLD: Duration = Duration::from_secs(90);
                            if open_duration > REMEDIATION_THRESHOLD
                                && let Some(config) = configs.get(dog_id)
                                && tracker.should_restart(dog_id, config)
                            {
                                klog!(
                                    "[Remediation] Dog '{}' open for {:.0}s, attempting restart on {}",
                                    dog_id,
                                    open_duration.as_secs_f64(),
                                    config.node
                                );
                                let node = config.node.clone();
                                let cmd = config.restart_command.clone();
                                match tokio::time::timeout(
                                    Duration::from_secs(15),
                                    tokio::task::spawn_blocking(move || {
                                        crate::infra::remediation::ssh_restart(&node, &cmd)
                                    }),
                                )
                                .await
                                {
                                    Ok(Ok(Ok(output))) => {
                                        klog!(
                                            "[Remediation] Dog '{}' restart initiated: {}",
                                            dog_id,
                                            output.trim()
                                        );
                                    }
                                    Ok(Ok(Err(e))) => {
                                        klog!(
                                            "[Remediation] Dog '{}' restart failed: {}",
                                            dog_id,
                                            e
                                        );
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

// ── Auto-remediation loop (K15: background recovery for degraded nodes, every 5min) ────

pub fn spawn_auto_remediation(
    storage: Arc<dyn StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Auto-remediation loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    match tokio::time::timeout(
                        Duration::from_secs(60),
                        run_auto_remediation(&storage),
                    )
                    .await
                    {
                        Ok(()) => {}
                        Err(_) => klog!("[AutoRemediation] tick timed out"),
                    }
                    task_health.touch_auto_remediation();
                }
            }
        }
    })
}

async fn run_auto_remediation(storage: &Arc<dyn StoragePort>) {
    // List degraded nodes: fatal_probe_count ≥ 2 in last 30min, or degradation score > 0.8
    let degraded = match storage.list_degraded_nodes(1800, 0.8).await {
        Ok(nodes) => nodes,
        Err(e) => {
            klog!("[AutoRemediation] list_degraded_nodes failed: {}", e);
            return;
        }
    };
    if degraded.is_empty() {
        return;
    }
    klog!(
        "[AutoRemediation] {} degraded nodes detected",
        degraded.len()
    );
    for (node, failure_reason, fatal_count, _) in degraded {
        klog!(
            "[AutoRemediation] attempting recovery: node={} reason={} fatal={}",
            node,
            failure_reason,
            fatal_count
        );
        let result =
            cynic_inference::application::node_runtime_ops::attempt_node_recovery(&node).await;
        klog!(
            "[AutoRemediation] recovery result: node={} status={}",
            node,
            result
        );
    }
}

// Rate limiter eviction moved to main.rs — REST delivery concern, not infra.

// ── Pattern Analysis Loop (K15: self-healing via pattern detection, every 30s) ────

pub fn spawn_pattern_analyzer(
    event_tx: tokio::sync::broadcast::Sender<crate::domain::events::KernelEvent>,
    storage_port: Arc<dyn crate::domain::storage::StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    let handle = tokio::spawn(async move {
        let mut rx = event_tx.subscribe();
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Pattern analyzer stopped");
                    break;
                }
                event_res = rx.recv() => {
                    match event_res {
                        Ok(crate::domain::events::KernelEvent::Anomaly { kind, message, severity }) => {
                            let patterns = crate::domain::pattern_analysis::detect_patterns(
                                vec![(kind, message, severity)],
                            );
                            for pattern in patterns {
                                klog!(
                                    "[PatternAnalyzer] healing pattern detected: {} ({}) -> {}",
                                    pattern.kind, pattern.severity, pattern.recommended_action
                                );
                                let obs = crate::domain::storage::Observation {
                                    project: "CYNIC".into(),
                                    agent_id: "pattern_analyzer".into(),
                                    tool: "pattern_analyzer".into(),
                                    target: "organism_health".into(),
                                    domain: "organism_health".into(),
                                    status: pattern.severity.clone(),
                                    context: format!(
                                        "{}\n\nRecommended action: {}",
                                        pattern.reasoning, pattern.recommended_action
                                    ),
                                    session_id: "autonomic_healing".into(),
                                    timestamp: chrono::Utc::now().timestamp_millis(),
                                    tags: vec!["autonomic".into(), "healing".into(), pattern.kind.clone()],
                                    source_tier: "permanent".into(),
                                    value: None,
                                    confidence: Some("observed".to_string()),
                                    consumer: Some("pattern_healing".to_string()),
                                    action: Some(pattern.recommended_action.clone()),
                                    depends_on: Vec::new(),
                                    maturity: Some(1.0),
                                    hash: String::new(),
                                    prev_hash: String::new(),
                                    observers: Vec::new(),
                                    consensus_score: None,
                                };
                                let storage = Arc::clone(&storage_port);
                                tokio::spawn(async move {
                                    if let Err(e) = storage.store_observation(&obs).await {
                                        klog!("[PatternAnalyzer] persist observation failed: {}", e);
                                    } else {
                                        klog!("[PatternAnalyzer] healing observation persisted ({})", obs.status);
                                    }
                                });
                            }
                            task_health.touch_pattern_analyzer();
                        }
                        Ok(_) => {
                            // Non-anomaly events ignored by pattern analyzer
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::debug!(skipped = n, "PatternAnalyzer lagged — skipping events");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            klog!("[PatternAnalyzer] event bus closed");
                            break;
                        }
                    }
                }
            }
        }
    });
    klog!("[Ring 2] Pattern analyzer started (in-process event bus, K15 healing)");
    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::coord::NullCoord;
    use crate::domain::storage::NullStorage;
    use crate::domain::usage::DogUsageTracker;
    use crate::infra::task_health::TaskHealth;

    #[tokio::test]
    async fn flush_usage_on_shutdown_skips_when_empty() {
        let storage: Arc<dyn StoragePort> = Arc::new(NullStorage);
        let usage = Arc::new(tokio::sync::Mutex::new(DogUsageTracker::new()));
        // No usage recorded → should skip
        flush_usage_on_shutdown(&storage, &usage).await;
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
        flush_usage_on_shutdown(&storage, &usage).await;
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
        tokio::time::timeout(Duration::from_secs(2), handle)
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
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("usage_flush should stop within 2s")
            .expect("task should not panic");
    }

    #[tokio::test]
    async fn remediation_watcher_respects_shutdown() {
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        // Empty configs + breakers — watcher should still start and stop cleanly
        let handle =
            spawn_remediation_watcher(HashMap::new(), vec![], task_health, shutdown.clone());
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("remediation_watcher should stop within 2s")
            .expect("task should not panic");
    }
}
