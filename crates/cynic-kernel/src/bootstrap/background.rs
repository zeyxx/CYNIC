use std::sync::Arc;

use cynic_inference::adapters::embedding::auto_recovery::AutoRecoveryEmbedding;
use cynic_inference::application::embedding_runtime::{
    EmbeddingRuntimeConfig, discover_embedding_backend,
};
use cynic_inference::embedding::EmbeddingPort;
use tokio_util::sync::CancellationToken;

use crate::infra::alerts::SlackAlerter;
use crate::{api::rest::AppState, infra};

pub struct RuntimeBackgroundTaskInputs {
    pub mcp_mode: bool,
    pub embedding_runtime: EmbeddingRuntimeConfig,
    pub embedding: Arc<AutoRecoveryEmbedding>,
    pub coord: Arc<dyn crate::domain::coord::CoordPort>,
    pub storage_port: Arc<dyn crate::domain::storage::StoragePort>,
    pub reconnector: Arc<crate::storage::reconnectable::StorageReconnector>,
    pub usage_tracker: Arc<tokio::sync::Mutex<crate::domain::usage::DogUsageTracker>>,
    pub metrics: Arc<crate::domain::metrics::Metrics>,
    pub task_health: Arc<crate::infra::task_health::TaskHealth>,
    pub event_tx: tokio::sync::broadcast::Sender<crate::domain::events::KernelEvent>,
    pub rest_state: Arc<AppState>,
    pub environment: Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
    pub shutdown: CancellationToken,
}

impl std::fmt::Debug for RuntimeBackgroundTaskInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeBackgroundTaskInputs")
            .field("mcp_mode", &self.mcp_mode)
            .field("embedding_runtime", &self.embedding_runtime)
            .field("embedding_ptr", &Arc::as_ptr(&self.embedding))
            .field("coord_ptr", &Arc::as_ptr(&self.coord))
            .field("storage_port_ptr", &Arc::as_ptr(&self.storage_port))
            .field("reconnector_ptr", &Arc::as_ptr(&self.reconnector))
            .field("metrics_ptr", &Arc::as_ptr(&self.metrics))
            .field("task_health_ptr", &Arc::as_ptr(&self.task_health))
            .field("event_tx", &"broadcast::Sender")
            .field("rest_state_ptr", &Arc::as_ptr(&self.rest_state))
            .field("environment_rwlock_open", &self.environment.read().is_ok())
            .field("shutdown_is_cancelled", &self.shutdown.is_cancelled())
            .finish()
    }
}

pub fn spawn_runtime_background_tasks(inputs: RuntimeBackgroundTaskInputs) {
    let RuntimeBackgroundTaskInputs {
        mcp_mode,
        embedding_runtime,
        embedding,
        coord,
        storage_port,
        reconnector,
        usage_tracker,
        metrics,
        task_health,
        event_tx,
        rest_state,
        environment,
        shutdown,
    } = inputs;

    // ─── Background tasks (REST-only — MCP is a thin tool-dispatch layer) ──
    // MCP mode skips all background tasks: the REST kernel already runs them.
    // Spawning duplicates causes SurrealDB contention, double nightshifts,
    // and 60s+ MCP startup delay that kills Hermes sessions.
    if !mcp_mode {
        // Background embedding discovery — retries every 60s if currently NullEmbedding
        {
            // Cast to dyn EmbeddingPort so the trait method is in scope inside the async block.
            let embed_ref: Arc<AutoRecoveryEmbedding> = Arc::clone(&embedding);
            let embed_probe: Arc<dyn EmbeddingPort> =
                Arc::clone(&embedding) as Arc<dyn EmbeddingPort>;
            let shutdown = shutdown.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                interval.tick().await; // skip first tick
                loop {
                    tokio::select! {
                        _ = shutdown.cancelled() => break,
                        _ = interval.tick() => {
                            if embed_probe.embed("health-probe").await.is_err()
                                && let Some(backend) = discover_embedding_backend(&embedding_runtime).await
                            {
                                klog!(
                                    "[Ring 2] Embedding: auto-discovered backend, hot-swapping"
                                );
                                embed_ref.upgrade(Arc::new(backend)).await;
                            }
                        }
                    }
                }
            });
        }

        // Coordination expiry (every 60s)
        infra::tasks::spawn_coord_expiry(
            Arc::clone(&coord),
            Arc::clone(&task_health),
            shutdown.clone(),
        );
        klog!("[Ring 2] Coordination expiry task started (every 60s)");

        // Usage + organ stats flush (every 60s)
        // Always spawned — ReconnectableStorage handles NullStorage gracefully.
        // If storage reconnects mid-flight, flush starts persisting automatically.
        infra::tasks::spawn_usage_flush_with_organ(
            Arc::clone(&storage_port),
            Arc::clone(&usage_tracker),
            Arc::clone(&task_health),
            shutdown.clone(),
            None,
        );
        klog!("[Ring 2] Usage + organ stats flush task started (every 60s, TTL cleanup every 1h)");

        // Session summarizer disabled

        // ─── One-shot: backfill crystal embeddings (orphan defragmentation) ──
        // Always spawned — on NullStorage the query returns empty (no-op).
        // After reconnect, backfill runs against the live DB.
        infra::tasks::spawn_backfill(
            Arc::clone(&storage_port),
            Arc::clone(&embedding) as Arc<dyn EmbeddingPort>,
            Arc::clone(&metrics),
            Arc::clone(&task_health),
            event_tx.clone(),
            shutdown.clone(),
        );

        // ─── Introspection loop (MAPE-K Analyze, every 5 min) ──
        infra::tasks::spawn_introspection(
            Arc::clone(&storage_port),
            Arc::clone(&metrics),
            Arc::clone(&environment),
            Arc::clone(&rest_state.introspection_alerts),
            event_tx.clone(),
            Arc::clone(&task_health),
            rest_state.senses.clone(),
            Arc::clone(&rest_state.producer_heartbeats),
            shutdown.clone(),
        );

        // ─── State log (hash-chained organism state, every 5min) ────
        infra::tasks::spawn_state_log(
            Arc::clone(&rest_state),
            Arc::clone(&task_health),
            shutdown.clone(),
        );

        // ─── Verdict Submission Queue (Pinocchio onchain anchoring) ────
        // DORMANT: disabled 2026-05-19. Queue ticked every 5min but submitted 0 verdicts
        // onchain (0 confirmed, 1 failed). Rewire when onchain anchoring is needed.
        // infra::tasks::spawn_submission_queue(
        //     Arc::clone(&storage_port),
        //     Arc::clone(&task_health),
        //     shutdown.clone(),
        // );

        // ─── Auto-remediation loop (K15: background node recovery, every 5min) ────
        infra::tasks::spawn_auto_remediation(
            Arc::clone(&storage_port),
            Arc::clone(&task_health),
            shutdown.clone(),
        );
        klog!("[Ring 2] Auto-remediation task started (every 5min)");

        // ─── Pattern analyzer (K15: in-process autonomic self-healing on Anomaly events) ────
        infra::tasks::spawn_pattern_analyzer(
            event_tx.clone(),
            Arc::clone(&storage_port),
            Arc::clone(&task_health),
            shutdown.clone(),
        );
        // ─── Event consumer + K15 alerting (ContractDelta → Slack) ────
        let slack = SlackAlerter::from_env();
        if slack.is_some() {
            klog!("[Ring 2] Slack alerter initialized (CYNIC_SLACK_WEBHOOK set)");
        } else {
            klog!("[Ring 2] Slack alerter disabled (CYNIC_SLACK_WEBHOOK not set)");
        }
        // Event consumer disabled

        // ─── Layer 4 Slack alerter: disabled

        // ─── Storage reconnect (K15: detection → action) ──
        // Ephemeral task — exits once storage is connected. No TaskHealth tracking
        // needed: effect is observable through storage status in /health.
        infra::tasks::spawn_storage_reconnect(Arc::clone(&reconnector), event_tx, shutdown.clone());

        // Probe scheduler disabled

        // Dog tasks disabled

        // Discovery loop disabled

        // ─── Governance (Auto-Submission) ────────────────────────
        let _governance_handle = infra::tasks::spawn_governance_queue(
            Arc::clone(&storage_port),
            Arc::clone(&task_health),
            shutdown,
        );
        klog!("[Ring 2] Governance submission queue started");

        // ─── Storage metrics emitter (every 60s, CHAOS→MATRIX data-centric measurement) ────
        // Storage metrics disabled

        // ─── Crystal immune system (every 5min, Soma L2: Background priority) ───────
        // Crystal challenge disabled

        // ─── Nightshift: DISABLED (K15 violation: no consumer for dev domain crystals) ────
        // Nightshift spawned every 1h to judge commits + observations → create dev crystals.
        // Problem: no code reads these crystals (search_crystals never filters by domain=dev).
        // Result: slots starved, memory pressure, embedding failures (100% failure rate).
        // Revived in Phase 2+ when temporal compounding actually has a consumer that acts.

        // ─── Convergence consumer: polls convergence signals, triggers enriched pipeline ──
        // Convergence disabled
    } else {
        klog!("[Ring 2] MCP mode — background tasks SKIPPED (REST kernel handles them)");
    }

    // MCP mode exits early (before Ring 0) — see MCP PROXY EARLY EXIT above.
    // This branch is unreachable when --mcp is set.
}
