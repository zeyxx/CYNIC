use cynic_inference::adapters::embedding::auto_recovery::AutoRecoveryEmbedding;
use cynic_inference::application::embedding_runtime::{
    EmbeddingRuntimeConfig, build_initial_embedding_port, discover_embedding_backend,
};
use cynic_inference::embedding::EmbeddingPort;
use cynic_kernel::bootstrap::entry::{
    KernelEntryFlags, configure_stdio_mode, init_tracing, load_env_file,
    print_version_and_exit_if_requested, run_mcp_proxy_mode_if_requested,
};
use cynic_kernel::bootstrap::{
    config::load_kernel_config_bundle, storage::connect_storage_runtime,
};
use cynic_kernel::bootstrap::{
    rest_state::{RestStateInputs, build_rest_state},
    services::build_runtime_services,
};
use cynic_kernel::domain::probe::Probe;
use cynic_kernel::infra::alerts::SlackAlerter;
use cynic_kernel::*;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// ============================================================
// BOOT SEQUENCE
// ============================================================
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_env_file();
    let entry_flags = KernelEntryFlags::from_env_args();

    if print_version_and_exit_if_requested(entry_flags)? {
        return Ok(());
    }

    configure_stdio_mode(entry_flags);
    init_tracing(entry_flags);

    if run_mcp_proxy_mode_if_requested(entry_flags).await? {
        return Ok(());
    }

    tracing::info!("CYNIC V2 — SOVEREIGN BOOT");
    klog!("╔══════════════════════════════════════╗");
    klog!("║       CYNIC OS V2 — SOVEREIGN BOOT    ║");
    klog!("╚══════════════════════════════════════╝");
    let node_config = probe::run(entry_flags.force_reprobe).await?;

    klog!("[Ring 0] Omniscience Active. Reality Mapped.");
    klog!(
        "[Ring 0] Host: {} | Compute: {:?} | VRAM: {}GB",
        std::env::consts::OS,
        node_config.compute.backend,
        node_config.compute.vram_gb
    );

    let kernel_config = load_kernel_config_bundle();
    let backends_path = kernel_config.backends_path.clone();
    let storage_config = kernel_config.storage_config.clone();
    let backend_configs = kernel_config.backend_configs.clone();
    let dog_thresholds = Arc::clone(&kernel_config.dog_thresholds);
    // Summarizer disabled pending excision review
    let system_contract = Arc::new(std::sync::RwLock::new(kernel_config.system_contract));

    let storage_runtime = connect_storage_runtime(&storage_config).await;
    let storage_port = storage_runtime.storage_port;
    let coord = storage_runtime.coord;
    let reconnector = storage_runtime.reconnector;

    // Validate config — probe health URLs, log warnings.
    // Spawned as background task (not awaited) so boot isn't delayed by network.
    // Health loop will catch any unhealthy backends within ~60s.
    {
        let configs = backend_configs.clone();
        tokio::spawn(async move {
            backends::health_probe::validate_config(configs).await;
        });
    }

    // ─── RING 1: Load domain prompts (chess.md, trading.md, etc.) ──
    // RC3: use runtime path discovery, not compile-time CARGO_MANIFEST_DIR
    let _domain_prompts = Arc::new(infra::config::load_domain_prompts());

    // ─── RING 2: Build Dogs + organ from backend_configs ──────
    // Factory loop lives in infra::boot so main stays a composition root.
    // build_dogs_and_organ disabled

    // Soma L2: Slot tracker — shared between health loop (writer) and Judge + REST (readers).
    let slot_tracker = Arc::new(domain::slot_tracker::SlotTracker::new());

    // Soma L2: Slot semaphore map — per-Dog permit coordination.
    // Created here (before health loop) so health loop can upsert slot counts at boot.
    let slot_semaphores = Arc::new(domain::slot_semaphore::SlotSemaphoreMap::new());

    // Soma L2+: Inference queues — priority-ordered slot access.
    // Replaces binary pass/skip with bounded priority queue per Dog.
    // Queue capacity = 8: enough to buffer a burst without unbounded growth.
    let inference_queues = Arc::new(domain::inference_queue::InferenceQueueMap::new());
    for cfg in &backend_configs {
        if cfg.sovereign {
            inference_queues.upsert(&cfg.name, 8);
            klog!("[Ring 2] InferenceQueue: {} (capacity=8)", cfg.name);
        }
    }

    // ─── RING 2: Health Loop + Remediation ──────────────────────
    // Remediation disabled

    // ─── RING 2: Embedding backend (sovereign, auto-recovery) ────
    let embedding_runtime = EmbeddingRuntimeConfig::from_env();
    let initial_embed: Arc<dyn EmbeddingPort> =
        build_initial_embedding_port(&embedding_runtime).await;
    let embedding: Arc<AutoRecoveryEmbedding> = Arc::new(AutoRecoveryEmbedding::new(initial_embed));

    // ─── RING 2: Build Judge ──────────────────────────────────
    let _breakers: Vec<Arc<dyn domain::health_gate::HealthGate>> = vec![];
    // Judge disabled
    // Background task health tracker — updated by each spawned task, exposed in /health
    let task_health = Arc::new(infra::task_health::TaskHealth::new());
    // Lifecycle orchestration — all background tasks select! on this token.
    // Signal handler cancels it → tasks break at safe boundaries → drain → flush → exit.
    let shutdown = CancellationToken::new();

    // Seed integrity hash chain from last stored verdict + verify integrity
    let chain_verified = infra::boot::seed_integrity_chain(&storage_port).await;

    // EPOCHÉ disabled

    // Sense registry — best-effort organ readers (RTK, Hermes X).
    let senses = senses::build_sense_registry();
    if !senses.is_empty() {
        klog!("[senses] {} organ(s) registered", senses.len());
        for s in &senses {
            klog!("[senses]   → {}", s.name());
        }
    }

    // ─── Routing calculator — dynamic Dog selection based on observed latencies ──
    // Constructed here (before health loop) so health_loop can call reset_dog on recovery.
    // Consumes dog_performance observations and adapts routing in real-time.
    let routing_calc = Arc::new(infra::routing_calc::RoutingCalculator::with_storage(
        Arc::clone(&storage_port),
    ));

    // ─── RING 2: Spawn health loop + remediation watcher ──────
    let all_breakers: Vec<Arc<dyn domain::health_gate::HealthGate>> = vec![];
    {
        let probe_configs: Vec<infra::health_loop::DogProbeConfig> = vec![];
        if !probe_configs.is_empty() {
            let probe_breakers: Vec<_> = probe_configs
                .iter()
                .filter_map(|pc| {
                    all_breakers
                        .iter()
                        .find(|cb| cb.dog_id() == pc.dog_id)
                        .cloned()
                })
                .collect();
            infra::health_loop::spawn_health_loop(
                probe_configs,
                probe_breakers,
                Arc::clone(&task_health),
                Arc::clone(&storage_port),
                Arc::clone(&slot_tracker),
                Arc::clone(&slot_semaphores),
                Arc::clone(&routing_calc),
                shutdown.clone(),
            );
            klog!(
                "[Ring 2] Health loop started (every {}s)",
                infra::circuit_breaker::PROBE_INTERVAL.as_secs()
            );
        }
    }
    if false {
        infra::tasks::spawn_remediation_watcher(
            std::collections::HashMap::new(),
            all_breakers,
            Arc::clone(&task_health),
            shutdown.clone(),
        );
    }

    // ─── RING 2: Organ remediation (C4/E1 — silence → auto-restart) ──
    {
        let organ_remediations = infra::config::load_organ_remediations(&backends_path);
        if !organ_remediations.is_empty() {
            klog!(
                "[Ring 2] Organ remediation: {} organs configured for auto-restart",
                organ_remediations.len()
            );
            infra::tasks::spawn_organ_remediation(
                organ_remediations,
                Arc::clone(&storage_port),
                Arc::clone(&task_health),
                shutdown.clone(),
            );
        }
    }

    // ─── RING 3: REST API (for React/external clients) ────────
    // let judge = Arc::new(judge);
    // judge_swap removed
    let usage_tracker = Arc::new(tokio::sync::Mutex::new(
        domain::usage::DogUsageTracker::new(),
    ));
    // Wire per-Dog cost rates from backends.toml
    {
        let mut usage = usage_tracker.lock().await;
        for (name, input_rate, output_rate) in &std::vec::Vec::<(String, u64, u64)>::new() {
            usage.set_cost_rate(name, *input_rate as f64, *output_rate as f64);
        }
        if false {
            klog!("[Ring 2] Cost rates: {} Dogs configured", 0);
        }
    }
    // Load historical usage from DB (survives restarts)
    match storage_port.load_usage_history().await {
        Ok(rows) if !rows.is_empty() => {
            let mut usage = usage_tracker.lock().await;
            usage.load_historical(&rows);
            klog!(
                "[Ring 2] Usage: loaded {} Dog histories ({} all-time requests)",
                rows.len(),
                usage.all_time_requests()
            );
        }
        Err(e) => klog!("[Ring 2] Usage: failed to load history (non-fatal): {}", e),
        _ => {}
    }
    let api_key = infra::boot::load_rest_api_key().map_err(std::io::Error::other)?;

    // Role-scoped authentication: three tokens, three roles.
    // Legacy CYNIC_API_KEY grants Cortex (backward-compat).
    let role_keys = api::rest::types::RoleKeyMap {
        cortex: std::env::var("CYNIC_API_KEY_CORTEX")
            .ok()
            .filter(|s| !s.is_empty()),
        organ: std::env::var("CYNIC_API_KEY_ORGAN")
            .ok()
            .filter(|s| !s.is_empty()),
        internal: std::env::var("CYNIC_API_KEY_INTERNAL")
            .ok()
            .filter(|s| !s.is_empty()),
        legacy: api_key.clone(),
    };

    // Single VerdictCache shared by REST and MCP — avoids duplicate caches (T4 fix)
    let verdict_cache = Arc::new(domain::verdict_cache::VerdictCache::new());
    // Pipeline metrics — shared by REST and MCP, exposed via /metrics
    let metrics = Arc::new(domain::metrics::Metrics::new());

    // Hydrate metrics from DB so counters survive reboots.
    // On NullStorage these return Err (non-fatal) — counters start at 0.
    {
        match storage_port.count_verdicts().await {
            Ok(v_count) => {
                metrics
                    .verdicts_total
                    .store(v_count, std::sync::atomic::Ordering::Relaxed);
                // K15 reconstruction: verdicts in database = consumption opportunity existed.
                // Prior verdicts were accessible via REST endpoints (/verdict/{id}, /verdicts).
                // Backfill verdicts_served to match verdicts_total so K15 ratio starts honest.
                // Going forward: direct measurement via inc_verdict_served() in each endpoint.
                metrics
                    .verdicts_served_total
                    .store(v_count, std::sync::atomic::Ordering::Relaxed);
                klog!(
                    "[Ring 2] Metrics: hydrated verdicts_total = {} (backfilled verdicts_served_total)",
                    v_count
                );
            }
            Err(e) => klog!(
                "[Ring 2] Metrics: failed to hydrate verdicts_total (counters start at 0): {}",
                e
            ),
        }
        match storage_port.count_crystal_observations().await {
            Ok(co_count) => {
                metrics
                    .crystal_observations_total
                    .store(co_count, std::sync::atomic::Ordering::Relaxed);
                klog!(
                    "[Ring 2] Metrics: hydrated crystal_observations_total = {}",
                    co_count
                );
            }
            Err(e) => klog!(
                "[Ring 2] Metrics: failed to hydrate crystal_observations (counters start at 0): {}",
                e
            ),
        }
        match storage_port.count_observations().await {
            Ok(o_count) => {
                metrics
                    .observations_ingested_total
                    .store(o_count, std::sync::atomic::Ordering::Relaxed);
                klog!(
                    "[Ring 2] Metrics: hydrated observations_ingested_total = {}",
                    o_count
                );
            }
            Err(e) => klog!(
                "[Ring 2] Metrics: failed to hydrate observations (counters start at 0): {}",
                e
            ),
        }
    }
    // ── Probe system (proprioception) ──
    let environment: Arc<std::sync::RwLock<Option<domain::probe::EnvironmentSnapshot>>> =
        Arc::new(std::sync::RwLock::new(None));
    let backup_dir = std::env::var("CYNIC_BACKUP_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".surrealdb")
                .join("backups")
        });
    // Build fleet probe targets from Dog health_urls (backends.toml SoT).
    // Pure transform extracted to infra::boot.
    let fleet_targets = infra::boot::build_fleet_targets(
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    );
    let fleet_inventory_probe = infra::probes::build_default_fleet_inventory_probe();

    // ── STARTUP VERIFICATION: Fail fast on backend coherence violations ──
    // Probe backends immediately (context drift, model mismatch) before claiming sovereignty.
    // This ensures organism knows its own inference constraints at boot time.
    let fleet_probe = infra::probes::FleetProbe::new(fleet_targets.clone());
    let startup_check = match fleet_probe.sense().await {
        Ok(result) => {
            if let domain::probe::ProbeDetails::Fleet(fleet) = &result.details {
                let mut coherence_ok = true;
                for dog in &fleet.dogs {
                    // Context drift: actual < expected
                    if let (Some(actual), Some(expected)) = (dog.actual_n_ctx, dog.expected_n_ctx)
                        && actual < expected
                    {
                        coherence_ok = false;
                        klog!(
                            "[BOOT FAIL] {} context drift at startup: {} (expected {}). Fix llama-server launch flags or backends.toml.",
                            dog.dog_name,
                            actual,
                            expected
                        );
                    }
                    // Model mismatch: wrong model loaded
                    if dog.model_mismatch {
                        coherence_ok = false;
                        klog!(
                            "[BOOT FAIL] {} model mismatch at startup: {:?} vs {:?}. Stop llama-server and verify correct model is loaded.",
                            dog.dog_name,
                            dog.actual_model,
                            dog.expected_model
                        );
                    }
                }
                coherence_ok
            } else {
                true // Not fleet details, skip check
            }
        }
        Err(_) => true, // Probe error, let periodic probes handle it
    };

    if !startup_check {
        // Backend coherence violations detected, but continue in degraded mode.
        // Circuit breaker will mark affected Dogs as failed, fallback routing will activate.
        // This ensures organism doesn't claim sovereignty (logs violations), but doesn't block startup.
        klog!(
            "[Boot] Backend coherence violations logged. Running in degraded mode — circuit breaker will isolate failed Dogs."
        );
    }

    let _probes: Vec<Arc<dyn domain::probe::Probe>> = vec![
        Arc::new(infra::probes::ResourceProbe::default()),
        Arc::new(infra::probes::BackupProbe::new(backup_dir)),
        Arc::new(infra::probes::ProcessProbe),
        Arc::new(infra::probes::PressureProbe),
        Arc::new(infra::probes::NetworkProbe),
        Arc::new(infra::probes::ProxyProbe),
        Arc::new(infra::probes::SomaProbe),
        Arc::new(fleet_probe),
        Arc::new(fleet_inventory_probe),
        Arc::new(infra::probes::VersionProbe::new()),
    ];

    let runtime_services = build_runtime_services(&backend_configs, &dog_thresholds);

    // DORMANT: Organism recovery (K15 consumer: state history → crystal cache)
    // state history consumer not yet complete — deferred to next phase
    let event_tx = runtime_services.event_tx.clone();
    let rest_state = build_rest_state(
        RestStateInputs {
            storage: Arc::clone(&storage_port),
            coord: Arc::clone(&coord),
            embedding: Arc::clone(&embedding),
            usage: Arc::clone(&usage_tracker),
            verdict_cache: Arc::clone(&verdict_cache),
            task_health: Arc::clone(&task_health),
            metrics: Arc::clone(&metrics),
            api_key,
            role_keys,
            storage_config: storage_config.clone(),
            chain_verified,
            environment: Arc::clone(&environment),
            system_contract: system_contract.clone(),
            backend_configs: backend_configs.clone(),
            senses,
            routing_calc: Arc::clone(&routing_calc),
            slot_semaphores: Arc::clone(&slot_semaphores),
            slot_tracker: Arc::clone(&slot_tracker),
        },
        runtime_services,
    );
    let rest_app = api::rest::router(Arc::clone(&rest_state));

    // ─── Background tasks (REST-only — MCP is a thin tool-dispatch layer) ──
    // MCP mode skips all background tasks: the REST kernel already runs them.
    // Spawning duplicates causes SurrealDB contention, double nightshifts,
    // and 60s+ MCP startup delay that kills Hermes sessions.
    if !entry_flags.mcp_mode {
        // Background embedding discovery — retries every 60s if currently NullEmbedding
        {
            // Cast to dyn EmbeddingPort so the trait method is in scope inside the async block.
            let embed_ref: Arc<AutoRecoveryEmbedding> = Arc::clone(&embedding);
            let embed_probe: Arc<dyn EmbeddingPort> =
                Arc::clone(&embedding) as Arc<dyn EmbeddingPort>;
            let embedding_runtime = embedding_runtime.clone();
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

        // DORMANT: Mail consumer — will activate when backends::mail exists.

        // ─── Pattern analyzer (K15: self-healing via pattern detection, every 30s) ────
        // R23-exempt: env var references (CYNIC_REST_ADDR, CYNIC_API_KEY), not hardcoded secrets
        let kernel_addr =
            std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| "127.0.0.1:3030".into());
        let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_default();
        infra::tasks::spawn_pattern_analyzer(
            kernel_addr,
            api_key,
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
        infra::tasks::spawn_storage_reconnect(
            Arc::clone(&reconnector),
            event_tx.clone(),
            shutdown.clone(),
        );

        // Probe scheduler disabled

        // Dog tasks disabled

        // Discovery loop disabled

        // ─── Governance (Auto-Submission) ────────────────────────
        let _governance_handle = infra::tasks::spawn_governance_queue(
            Arc::clone(&storage_port),
            Arc::clone(&task_health),
            shutdown.clone(),
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

    // ─── RING 3: REST Server ─────────────────────────────────
    let rest_addr = std::env::var("CYNIC_REST_ADDR")
        .unwrap_or_else(|_| domain::constants::DEFAULT_REST_ADDR.to_string());
    klog!("[Ring 3] REST API on http://{}", rest_addr);

    // Kernel self-observation: boot event
    {
        let storage = Arc::clone(&storage_port);
        let obs = domain::storage::Observation {
            project: "CYNIC".into(),
            agent_id: "kernel".into(),
            tool: "boot".into(),
            target: "self".into(),
            domain: "kernel-lifecycle".into(),
            status: "event".into(),
            context: format!(
                "version={} build={} sha={} dogs={}",
                env!("CARGO_PKG_VERSION"),
                env!("CYNIC_VERSION"),
                env!("CYNIC_GIT_SHA"),
                0
            ),
            session_id: String::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            tags: vec!["kernel-self-obs".into()],
            source_tier: "permanent".to_string(),
            value: None,
            confidence: None,
            consumer: None,
            action: None,
            depends_on: vec![],
            maturity: None,
            hash: String::new(),
            prev_hash: String::new(),
            observers: vec![],
            consensus_score: None,
        };
        let _ = storage.store_observation(&obs).await;
    }

    // Rate limiter eviction (REST delivery concern — lives here, not in infra/tasks.rs)
    {
        let rs = Arc::clone(&rest_state);
        let th = Arc::clone(&task_health);
        let sd = shutdown.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await;
            loop {
                tokio::select! {
                    _ = sd.cancelled() => {
                        klog!("[SHUTDOWN] Rate limiter eviction stopped");
                        break;
                    }
                    _ = interval.tick() => {
                        match tokio::time::timeout(std::time::Duration::from_secs(5), rs.rate_limiter.evict_stale()).await {
                            Ok(()) => {}
                            Err(_) => tracing::warn!("rate_limiter evict_stale timed out (5s)"),
                        }
                        match tokio::time::timeout(std::time::Duration::from_secs(5), rs.judge_limiter.evict_stale()).await {
                            Ok(()) => {}
                            Err(_) => tracing::warn!("judge_limiter evict_stale timed out (5s)"),
                        }
                        th.touch_rate_eviction();
                    }
                }
            }
        });
    }

    let rest_listener = tokio::net::TcpListener::bind(&rest_addr).await?;

    // Set up Unix Domain Socket listener
    let uds_path = "/tmp/cynic-kernel.sock";
    let _ = std::fs::remove_file(uds_path); // Cleanup stale socket
    let uds_listener = tokio::net::UnixListener::bind(uds_path)?;
    // Secure the UDS: only the owner can read/write (0600)
    let _ = std::fs::set_permissions(
        uds_path,
        std::os::unix::fs::PermissionsExt::from_mode(0o600),
    );

    // Signal handler — fires the shutdown token
    infra::tasks::spawn_signal_handler(shutdown.clone());

    klog!("╔══════════════════════════════════════╗");
    klog!("║   CYNIC SOVEREIGN — ALL SYSTEMS GO   ║");
    klog!("║   REST/WS: http://{} / ws://{}", rest_addr, rest_addr);
    klog!("║   UDS:     unix://{}    ║", uds_path);
    klog!("║   Max confidence: phi^-1 = 0.618     ║");
    klog!("╚══════════════════════════════════════╝");

    // Spawn UDS server loop
    {
        let mut make_service = rest_app.clone().into_make_service();
        let shutdown_uds = shutdown.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_uds.cancelled() => {
                        klog!("[SHUTDOWN] UDS server drained");
                        break;
                    }
                    accept_result = uds_listener.accept() => {
                        if let Ok((socket, _addr)) = accept_result {
                            let io = hyper_util::rt::TokioIo::new(socket);
                            // Unpack the tower service
                            let Ok(tower_service) = tower::Service::call(&mut make_service, ()).await;
                            let svc = hyper::service::service_fn(move |req| {
                                let mut svc = tower_service.clone();
                                async move { tower::Service::call(&mut svc, req).await }
                            });
                            tokio::spawn(async move {
                                if let Err(e) = hyper::server::conn::http1::Builder::new()
                                    .serve_connection(io, svc)
                                    .with_upgrades() // CRITICAL: Enables WebSocket over UDS!
                                    .await
                                {
                                    tracing::debug!("UDS Connection error: {}", e);
                                }
                            });
                        }
                    }
                }
            }
        });
    }

    // axum with graceful shutdown for TCP (Tailscale remote organs)
    // F2: into_make_service_with_connect_info injects ConnectInfo<SocketAddr>
    if let Err(e) = axum::serve(
        rest_listener,
        rest_app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown.clone().cancelled_owned())
    .await
    {
        tracing::error!(error = %e, "REST/TCP server fatal error");
    }

    klog!("[SHUTDOWN] REST server drained — flushing state");
    infra::tasks::flush_usage_on_shutdown(&storage_port, &usage_tracker).await;

    // Drain fire-and-forget background tasks (observations, audits)
    rest_state.bg_tasks.close();
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        rest_state.bg_tasks.wait(),
    )
    .await
    {
        Ok(()) => klog!("[SHUTDOWN] Background tasks drained ({} completed)", 0),
        Err(_) => tracing::warn!("background tasks did not drain within 5s"),
    }

    Ok(())
}
