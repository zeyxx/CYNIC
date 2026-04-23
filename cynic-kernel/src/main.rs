use cynic_kernel::domain::inference::BackendPort;
use cynic_kernel::infra::alerts::SlackAlerter;
use cynic_kernel::*;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

fn select_summarizer_backend(
    backend_configs: &[infra::config::BackendConfig],
) -> Option<infra::config::BackendConfig> {
    if let Ok(explicit_name) = std::env::var("CYNIC_SUMMARIZER_BACKEND")
        && let Some(cfg) = backend_configs.iter().find(|cfg| cfg.name == explicit_name)
    {
        return Some(cfg.clone());
    }

    backend_configs
        .iter()
        .find(|cfg| cfg.name == "qwen35-9b-gpu")
        .cloned()
        .or_else(|| {
            backend_configs
                .iter()
                .find(|cfg| cfg.name == "sovereign")
                .cloned()
        })
        .or_else(|| {
            backend_configs
                .iter()
                .find(|cfg| {
                    cfg.backend_type != infra::config::BackendType::Cli
                        && cfg.base_url.starts_with("http://")
                })
                .cloned()
        })
}

fn build_summarizer(
    cfg: Option<&infra::config::BackendConfig>,
) -> Result<backends::summarizer::SovereignSummarizer, domain::inference::BackendInitError> {
    if let Some(cfg) = cfg {
        backends::summarizer::SovereignSummarizer::from_backend_config(cfg)
    } else {
        backends::summarizer::SovereignSummarizer::from_env()
    }
}

fn load_rest_api_key() -> Result<Option<String>, String> {
    match std::env::var("CYNIC_API_KEY") {
        Ok(key) if !key.is_empty() => Ok(Some(key)),
        Ok(_) | Err(std::env::VarError::NotPresent) => {
            let allow_open = std::env::var("CYNIC_ALLOW_OPEN_API")
                .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                .unwrap_or(false);
            if allow_open {
                Ok(None)
            } else {
                Err(
                    "CYNIC_API_KEY is required for REST auth; set CYNIC_ALLOW_OPEN_API=1 only for explicit local development"
                        .into(),
                )
            }
        }
        Err(std::env::VarError::NotUnicode(_)) => Err("CYNIC_API_KEY must be valid UTF-8".into()),
    }
}

// ============================================================
// BOOT SEQUENCE
// ============================================================
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse flags early — MCP mode needs stderr-only logging from the start.
    let force_reprobe = std::env::args().any(|a| a == "--reset");
    let mcp_mode = std::env::args().any(|a| a == "--mcp");

    if std::env::args().any(|a| a == "--version") {
        #[allow(clippy::print_stdout)]
        // WHY: This is the --version flag output path. Printing the version string to stdout is
        // the correct and expected behaviour here; the MCP_MODE guard (which bans stdout to avoid
        // corrupting the JSON-RPC stream) has not yet been set when this branch executes.
        {
            println!("cynic-kernel {}", env!("CARGO_PKG_VERSION"));
        }
        return Ok(());
    }

    // Set global MCP flag BEFORE any module has a chance to println! to stdout.
    if mcp_mode {
        cynic_kernel::MCP_MODE.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    // ─── Structured logging (tracing) ────────────────────────
    // RUST_LOG controls filtering. Default: info for cynic, warn for deps.
    // MCP mode: write to stderr (stdout reserved for JSON-RPC).
    // REST mode: write to stdout with JSON format for production.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("cynic_kernel=info,warn"));

    if mcp_mode {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json().with_writer(std::io::stderr))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().json())
            .init();
    }

    tracing::info!("CYNIC V2 — SOVEREIGN BOOT");
    klog!("╔══════════════════════════════════════╗");
    klog!("║       CYNIC OS V2 — SOVEREIGN BOOT    ║");
    klog!("╚══════════════════════════════════════╝");
    let node_config = probe::run(force_reprobe).await?;

    klog!("[Ring 0] Omniscience Active. Reality Mapped.");
    klog!(
        "[Ring 0] Host: {} | Compute: {:?} | VRAM: {}GB",
        std::env::consts::OS,
        node_config.compute.backend,
        node_config.compute.vram_gb
    );

    // ─── RING 1: Load config (single source of truth) ─────────
    let backends_path = dirs::config_dir()
        .unwrap_or_else(|| {
            klog!("[Ring 1] dirs::config_dir() unavailable — falling back to current directory");
            std::path::PathBuf::from(".")
        })
        .join("cynic")
        .join("backends.toml");
    let storage_config = infra::config::load_storage_config(&backends_path);
    klog!("[Ring 1] Config: {}", backends_path.display());
    klog!(
        "[Ring 1] Storage: url={} ns={} db={}",
        storage_config.url,
        storage_config.namespace,
        storage_config.database
    );

    // ─── RING 1: Native Storage Client (UAL) ──────────────────
    // HTTP adapter to SurrealDB 3.x — graceful degradation if unavailable.
    // Wrapped in ReconnectableStorage/Coord so a background task can swap
    // NullStorage → SurrealHttpStorage without restarting the kernel.
    let (storage_port, coord, reconnector): (
        Arc<dyn domain::storage::StoragePort>,
        Arc<dyn domain::coord::CoordPort>,
        Arc<storage::reconnectable::StorageReconnector>,
    ) = {
        let (raw_storage, raw_coord): (
            Arc<dyn domain::storage::StoragePort>,
            Arc<dyn domain::coord::CoordPort>,
        ) = match storage::SurrealHttpStorage::init(&storage_config).await {
            Ok(s) => {
                klog!("[Ring 1] Storage: HEALTHY (SurrealDB HTTP)");
                let db = Arc::new(s);
                (Arc::clone(&db) as _, Arc::clone(&db) as _)
            }
            Err(e) => {
                tracing::warn!(error = %e, "storage DEGRADED — will auto-reconnect when SurrealDB is available");
                (
                    Arc::new(domain::storage::NullStorage) as _,
                    Arc::new(domain::coord::NullCoord) as _,
                )
            }
        };
        let reconnectable_storage = Arc::new(storage::reconnectable::ReconnectableStorage::new(
            raw_storage,
        ));
        let reconnectable_coord =
            Arc::new(storage::reconnectable::ReconnectableCoord::new(raw_coord));
        let reconnector = Arc::new(storage::reconnectable::StorageReconnector::new(
            reconnectable_storage.shared_lock(),
            reconnectable_coord.shared_lock(),
            storage_config.clone(),
        ));
        (
            reconnectable_storage as Arc<dyn domain::storage::StoragePort>,
            reconnectable_coord as Arc<dyn domain::coord::CoordPort>,
            reconnector,
        )
    };

    // ─── RING 2: Load Backend Configs ──────────────────────────

    let backend_configs = if backends_path.exists() {
        klog!("[Ring 2] Loading backends from {}", backends_path.display());
        infra::config::load_backends(&backends_path)
    } else {
        klog!("[Ring 2] No backends.toml found, using env var fallback");
        infra::config::load_backends_from_env()
    };
    let summarizer_backend_cfg = select_summarizer_backend(&backend_configs);
    if let Some(cfg) = summarizer_backend_cfg.as_ref() {
        klog!(
            "[Ring 2] Sovereign summarizer backend: {} (timeout={}s)",
            cfg.name,
            cfg.timeout_secs
        );
    }

    // Self-model: load SystemContract from backends.toml — ALL declared Dogs,
    // regardless of whether their env vars resolve right now.
    let system_contract = infra::config::load_system_contract(&backends_path);
    let expected_count = system_contract.expected_count();
    let expected_dogs = system_contract.expected_dogs().to_vec();
    let system_contract = Arc::new(std::sync::RwLock::new(system_contract));

    klog!(
        "[Ring 2] SystemContract: {} expected Dogs {:?}",
        expected_count,
        expected_dogs
    );

    // Validate config — probe health URLs, log warnings.
    // Spawned as background task (not awaited) so boot isn't delayed by network.
    // Health loop will catch any unhealthy backends within ~60s.
    {
        let configs = backend_configs.clone();
        tokio::spawn(async move {
            backends::health_probe::validate_config(&configs).await;
        });
    }

    // ─── RING 1: Load domain prompts (chess.md, trading.md, etc.) ──
    // RC3: use runtime path discovery, not compile-time CARGO_MANIFEST_DIR
    let project_root = std::env::var("CYNIC_PROJECT_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            // Try git discovery, fall back to binary's parent dir
            std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| std::path::PathBuf::from(s.trim()))
                .unwrap_or_else(|| {
                    let fallback =
                        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                    klog!(
                        "[Ring 1] No CYNIC_PROJECT_ROOT and git not available — using cwd: {}",
                        fallback.display()
                    );
                    fallback
                })
        });
    // Validate project_root has a .git directory — nightshift depends on this
    if !project_root.join(".git").exists() {
        tracing::error!(
            path = %project_root.display(),
            "project_root has no .git/ — nightshift git log will fail silently. Set CYNIC_PROJECT_ROOT or start from repo root."
        );
    }
    let domain_prompts = Arc::new(infra::config::load_domain_prompts(&project_root));

    // ─── RING 2: Build Dogs + organ from backend_configs ──────
    // Factory loop lives in infra::boot so main stays a composition root.
    let infra::boot::DogsAndOrgan {
        dogs,
        organ_handles,
        organ,
        cost_rates,
        health_urls,
        fleet_meta,
        remediation_configs,
    } = infra::boot::build_dogs_and_organ(backend_configs, &domain_prompts, &storage_port).await;

    // ─── RING 2: Health Loop + Remediation ──────────────────────
    // Config comes from backends.toml (SoT) — no separate remediation.toml needed.
    if !remediation_configs.is_empty() {
        klog!(
            "[Ring 2] Remediation: {} Dogs configured for auto-restart",
            remediation_configs.len()
        );
    }

    // ─── RING 2: Embedding backend (sovereign, auto-recovery) ────
    // Always wire the real backend — if down at boot, embed() returns Err (same as NullEmbedding).
    // When the server comes back, calls start succeeding automatically. No manual recovery needed.
    let embedding: Arc<dyn domain::embedding::EmbeddingPort> =
        match backends::embedding::EmbeddingBackend::from_env() {
            Ok(embed_backend) => {
                let embed_health = embed_backend.health().await;
                if embed_health.is_available() {
                    klog!("[Ring 2] Embedding: {:?} (sovereign)", embed_health);
                } else {
                    klog!(
                        "[Ring 2] Embedding: {:?} — will auto-recover when server is available",
                        embed_health
                    );
                }
                Arc::new(embed_backend)
            }
            Err(e) => {
                klog!(
                    "[Ring 2] Embedding: HTTP client init failed ({}) — using NullEmbedding",
                    e
                );
                Arc::new(domain::embedding::NullEmbedding)
            }
        };

    // ─── RING 2: Build Judge ──────────────────────────────────
    let breakers: Vec<Arc<dyn domain::health_gate::HealthGate>> = dogs
        .iter()
        .map(|d| {
            Arc::new(infra::circuit_breaker::CircuitBreaker::new(
                d.id().to_string(),
            )) as Arc<dyn domain::health_gate::HealthGate>
        })
        .collect();
    let judge = judge::Judge::new(dogs, breakers).with_organ_handles(organ_handles);
    // Background task health tracker — updated by each spawned task, exposed in /health
    let task_health = Arc::new(infra::task_health::TaskHealth::new());
    // Lifecycle orchestration — all background tasks select! on this token.
    // Signal handler cancels it → tasks break at safe boundaries → drain → flush → exit.
    let shutdown = CancellationToken::new();

    // Seed integrity hash chain from last stored verdict + verify integrity
    let chain_verified = infra::boot::seed_integrity_chain(&storage_port, &judge).await;

    // ─── RING 2: Spawn health loop + remediation watcher ──────
    let all_breakers: Vec<Arc<dyn domain::health_gate::HealthGate>> =
        judge.breakers().iter().map(Arc::clone).collect();
    {
        let probe_configs: Vec<infra::health_loop::DogProbeConfig> = judge
            .dog_ids()
            .iter()
            .filter(|id| *id != "deterministic-dog")
            .filter_map(|id| {
                health_urls
                    .get(id.as_str())
                    .and_then(|opt| opt.as_ref())
                    .map(|url| infra::health_loop::DogProbeConfig {
                        dog_id: id.clone(),
                        health_url: url.clone(),
                    })
            })
            .collect();
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
                shutdown.clone(),
            );
            klog!(
                "[Ring 2] Health loop started (every {}s)",
                infra::circuit_breaker::PROBE_INTERVAL.as_secs()
            );
        }
    }
    if !remediation_configs.is_empty() {
        infra::tasks::spawn_remediation_watcher(
            remediation_configs.clone(),
            all_breakers,
            Arc::clone(&task_health),
            shutdown.clone(),
        );
    }

    // ─── RING 3: REST API (for React/external clients) ────────
    let judge = Arc::new(judge);
    let judge_swap = arc_swap::ArcSwap::from(Arc::clone(&judge));
    let usage_tracker = Arc::new(tokio::sync::Mutex::new(
        domain::usage::DogUsageTracker::new(),
    ));
    // Wire per-Dog cost rates from backends.toml
    {
        let mut usage = usage_tracker.lock().await;
        for (name, input_rate, output_rate) in &cost_rates {
            usage.set_cost_rate(name, *input_rate, *output_rate);
        }
        if !cost_rates.is_empty() {
            klog!("[Ring 2] Cost rates: {} Dogs configured", cost_rates.len());
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
    let api_key = load_rest_api_key().map_err(std::io::Error::other)?;
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
                klog!("[Ring 2] Metrics: hydrated verdicts_total = {}", v_count);
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
    let fleet_targets = infra::boot::build_fleet_targets(&health_urls, &fleet_meta);
    let probes: Vec<Arc<dyn domain::probe::Probe>> = vec![
        Arc::new(infra::probes::ResourceProbe::default()),
        Arc::new(infra::probes::BackupProbe::new(backup_dir)),
        Arc::new(infra::probes::ProcessProbe),
        Arc::new(infra::probes::PressureProbe),
        Arc::new(infra::probes::NetworkProbe),
        Arc::new(infra::probes::FleetProbe::new(fleet_targets)),
    ];

    // ── Token enricher (Helius) — optional, graceful degradation ──
    let enricher: Option<Arc<dyn domain::enrichment::TokenEnricherPort>> =
        match backends::helius::HeliusEnricher::from_env() {
            Some(h) => {
                klog!("[Boot] Helius enricher configured — token-analysis will use on-chain data");
                Some(Arc::new(h))
            }
            None => {
                klog!(
                    "[Boot] Helius enricher not configured — token-analysis will use raw addresses"
                );
                None
            }
        };

    // Event bus — broadcast channel for SSE/WebSocket subscribers.
    // Capacity 256: events are small JSON, subscribers should keep up.
    let (event_tx, _) = tokio::sync::broadcast::channel::<domain::events::KernelEvent>(256);
    let rest_state = Arc::new(api::rest::AppState {
        judge: judge_swap,
        storage: Arc::clone(&storage_port),
        coord: Arc::clone(&coord),
        embedding: Arc::clone(&embedding),
        usage: Arc::clone(&usage_tracker),
        verdict_cache: Arc::clone(&verdict_cache),
        task_health: Arc::clone(&task_health),
        metrics: Arc::clone(&metrics),
        api_key,
        storage_info: api::rest::StorageInfo {
            namespace: storage_config.namespace.clone(),
            database: storage_config.database.clone(),
        },
        rate_limiter: api::rest::PerIpRateLimiter::new(30), // 30 requests/minute global
        judge_limiter: api::rest::PerIpRateLimiter::new(10), // 10 /judge per minute (inference costs money)
        ready_cache: api::rest::ReadyCache::new(),
        bg_semaphore: Arc::new(tokio::sync::Semaphore::new(
            domain::constants::BG_SEMAPHORE_PERMITS,
        )),
        bg_tasks: tokio_util::task::TaskTracker::new(),
        sse_semaphore: Arc::new(tokio::sync::Semaphore::new(
            domain::constants::SSE_SEMAPHORE_PERMITS,
        )),
        introspection_alerts: Arc::new(std::sync::RwLock::new(Vec::new())),
        event_tx: event_tx.clone(),
        chain_verified: std::sync::atomic::AtomicBool::new(chain_verified),
        environment: Arc::clone(&environment),
        registered_dogs: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        judge_jobs: Arc::new(api::rest::judge_job::JudgeJobStore::new()),
        system_contract: system_contract.clone(),
        enricher: enricher.clone(),
    });
    let rest_app = api::rest::router(Arc::clone(&rest_state));

    // ─── Background tasks (universal — run in BOTH MCP and REST modes) ──
    // These MUST be spawned before the MCP/REST branch.

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
        Some(Arc::clone(&organ)),
    );
    klog!("[Ring 2] Usage + organ stats flush task started (every 60s, TTL cleanup every 1h)");

    // ─── RING 2: Session summarizer (sovereign inference, background) ──
    if let Ok(summarizer) = build_summarizer(summarizer_backend_cfg.as_ref()) {
        infra::tasks::spawn_session_summarizer(
            Arc::clone(&storage_port),
            summarizer,
            Arc::clone(&task_health),
            shutdown.clone(),
        );
    } else {
        klog!("[Ring 2] Session summarizer DISABLED — HTTP client init failed");
    }

    // ─── One-shot: backfill crystal embeddings (orphan defragmentation) ──
    // Always spawned — on NullStorage the query returns empty (no-op).
    // After reconnect, backfill runs against the live DB.
    infra::tasks::spawn_backfill(
        Arc::clone(&storage_port),
        Arc::clone(&embedding),
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
        shutdown.clone(),
    );

    // ─── Event consumer + K15 alerting (ContractDelta → Slack) ────
    let slack = SlackAlerter::from_env();
    if slack.is_some() {
        klog!("[Ring 2] Slack alerter initialized (CYNIC_SLACK_WEBHOOK set)");
    } else {
        klog!("[Ring 2] Slack alerter disabled (CYNIC_SLACK_WEBHOOK not set)");
    }
    infra::tasks::spawn_event_consumer(
        &event_tx,
        Arc::clone(&rest_state),
        Arc::clone(&task_health),
        shutdown.clone(),
        slack,
    );

    // ─── Storage reconnect (K15: detection → action) ──
    // Ephemeral task — exits once storage is connected. No TaskHealth tracking
    // needed: effect is observable through storage status in /health.
    infra::tasks::spawn_storage_reconnect(
        Arc::clone(&reconnector),
        event_tx.clone(),
        shutdown.clone(),
    );

    infra::tasks::spawn_probe_scheduler(
        probes,
        Arc::clone(&storage_port),
        Arc::clone(&environment),
        Arc::clone(&organ),
        Arc::clone(&task_health),
        shutdown.clone(),
    );
    klog!(
        "[Ring 2] Probe scheduler started (resource+process+pressure+network+fleet: 30s, backup: 1h)"
    );

    infra::tasks::spawn_dog_ttl_checker(
        Arc::clone(&rest_state),
        Arc::clone(&task_health),
        shutdown.clone(),
    );
    klog!("[Ring 2] Dog TTL checker started (every 30s)");

    infra::tasks::spawn_dog_heartbeat_loop(
        Arc::clone(&rest_state),
        Arc::clone(&task_health),
        shutdown.clone(),
    );
    klog!("[Ring 2] Dog heartbeat loop started (every 40s, K15 consumer)");

    infra::tasks::spawn_discovery_loop(
        Arc::clone(&rest_state),
        fleet_meta,
        Arc::clone(&task_health),
        shutdown.clone(),
    );
    klog!("[Ring 2] Discovery loop started (every 60s, organism-agnostic)");

    // ─── Crystal immune system (re-judge oldest crystals, dissolve if degraded) ──
    infra::tasks::spawn_crystal_challenge_loop(
        rest_state.judge.load_full(),
        Arc::clone(&storage_port),
        Arc::clone(&task_health),
        shutdown.clone(),
    );
    klog!("[Ring 2] Crystal challenge loop started (every 5min, immune system)");

    // ─── Nightshift: autonomous dev judgment (every 4h) ───────
    let _nightshift_handle = infra::tasks::spawn_nightshift_loop(
        rest_state.judge.load_full(),
        Arc::clone(&storage_port),
        Arc::clone(&task_health),
        shutdown.clone(),
        project_root.display().to_string(),
    );
    klog!(
        "[Ring 3] Nightshift loop started (every 4h, git lookback {})",
        crate::domain::constants::NIGHTSHIFT_GIT_LOOKBACK
    );

    // ─── RING 3: MCP Server (for AI agents via stdio) ────────
    if mcp_mode {
        use rmcp::ServiceExt;
        tracing::info!("MCP mode — serving over stdio (background tasks active)");
        let mcp_infer: Arc<dyn domain::inference::InferPort> = match build_summarizer(
            summarizer_backend_cfg.as_ref(),
        ) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                tracing::warn!(error = %e, "MCP inference unavailable — HTTP client init failed");
                Arc::new(domain::inference::NullInfer)
            }
        };
        let mcp_server = api::mcp::CynicMcp::new(
            Arc::clone(&judge),
            Arc::clone(&storage_port),
            Arc::clone(&coord),
            Arc::clone(&usage_tracker),
            Arc::clone(&embedding),
            Arc::clone(&verdict_cache),
            mcp_infer,
            Arc::clone(&metrics),
            Arc::clone(&environment),
            Arc::clone(&task_health),
            system_contract
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            Some(event_tx.clone()),
            project_root.display().to_string(),
            enricher.clone(),
        );

        // MCP signal handler — cancel background tasks on SIGTERM/SIGINT
        infra::tasks::spawn_signal_handler(shutdown.clone());

        let transport = rmcp::transport::io::stdio();
        let server = mcp_server
            .serve(transport)
            .await
            .map_err(|e| format!("MCP server error: {e}"))?;

        // Wait for MCP disconnect or shutdown signal
        tokio::select! {
            _ = server.waiting() => {} // ok: MCP client disconnected
            _ = shutdown.clone().cancelled_owned() => {
                tracing::info!("MCP shutting down");
            }
        }

        // Cancel background tasks and flush
        shutdown.cancel();
        infra::tasks::flush_usage_on_shutdown(&storage_port, &usage_tracker).await;
        return Ok(());
    }

    // ─── RING 3: REST Server ─────────────────────────────────
    let rest_addr = std::env::var("CYNIC_REST_ADDR")
        .unwrap_or_else(|_| domain::constants::DEFAULT_REST_ADDR.to_string());
    klog!("[Ring 3] REST API on http://{}", rest_addr);

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

    // Signal handler — fires the shutdown token
    infra::tasks::spawn_signal_handler(shutdown.clone());

    klog!("╔══════════════════════════════════════╗");
    klog!("║   CYNIC SOVEREIGN — ALL SYSTEMS GO   ║");
    klog!("║   REST: http://{}", rest_addr);
    klog!("║   Max confidence: phi^-1 = 0.618     ║");
    klog!("╚══════════════════════════════════════╝");

    // axum with graceful shutdown — stops accepting connections on token cancel,
    // drains in-flight requests (bounded by tower TimeoutLayer at 120s)
    // F2: into_make_service_with_connect_info injects ConnectInfo<SocketAddr>
    // into every request's extensions — rate limiter reads real peer addr, not X-Forwarded-For.
    if let Err(e) = axum::serve(
        rest_listener,
        rest_app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown.clone().cancelled_owned())
    .await
    {
        tracing::error!(error = %e, "REST server fatal error");
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
