use cynic_kernel::domain::inference::BackendPort;
use cynic_kernel::domain::probe::Probe;
use cynic_kernel::infra::alerts::SlackAlerter;
use cynic_kernel::*;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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

    // ─── MCP PROXY EARLY EXIT ─────────────────────────────────
    // When --mcp is set, skip the entire kernel init (Rings 0-2).
    // Forward all tool calls to the running REST kernel via HTTP.
    // This is the monolith fix: MCP clients get zero local state.
    if mcp_mode {
        tracing::warn!(
            "DEPRECATED: --mcp flag will be removed. Use the standalone cynic-mcp binary instead."
        );
        use rmcp::ServiceExt;

        let raw_addr =
            std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| "http://127.0.0.1:3030".into());
        // Env may contain bare "IP:PORT" — prepend http:// if missing.
        let rest_addr = if raw_addr.starts_with("http://") || raw_addr.starts_with("https://") {
            raw_addr
        } else {
            format!("http://{raw_addr}")
        };
        let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_default();
        let project_root = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .display()
            .to_string();

        tracing::info!(
            rest_addr,
            "MCP proxy mode — forwarding to REST kernel (zero local state)"
        );

        let mcp_proxy = api::mcp::proxy::CynicMcpProxy::new(rest_addr, api_key, project_root);

        let shutdown = CancellationToken::new();
        infra::tasks::spawn_signal_handler(shutdown.clone());

        let transport = rmcp::transport::io::stdio();
        let server = mcp_proxy
            .serve(transport)
            .await
            .map_err(|e| format!("MCP proxy error: {e}"))?;

        tokio::select! {
            _ = server.waiting() => {}
            _ = shutdown.clone().cancelled_owned() => {
                tracing::info!("MCP proxy shutting down");
            }
        }

        shutdown.cancel();
        return Ok(());
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
    let summarizer_backend_cfg = infra::boot::select_summarizer_backend(&backend_configs);
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

    // Load Dog thresholds and error detection patterns from backends.toml.
    // These drive circuit breaker behavior, error classification, and monitoring alerts.
    let dog_thresholds = Arc::new(infra::config::load_dog_thresholds(&backends_path));
    klog!(
        "[Ring 2] Dog thresholds: circuit open={} failure_threshold={} error patterns: quota({}) transient({}) critical({})",
        dog_thresholds.circuit.open_duration_secs,
        dog_thresholds.circuit.open_on_consecutive_failures,
        dog_thresholds.error_detection.quota_patterns.len(),
        dog_thresholds.error_detection.transient_patterns.len(),
        dog_thresholds.error_detection.critical_patterns.len()
    );

    // Filter out disabled backends — don't register Dogs that are disabled in backends.toml.
    // Before this fix, disabled Dogs were still registered, counted in expected_dog_count,
    // and caused permanent jury gate downgrades (expected=5, actual=3).
    let backend_configs: Vec<_> = backend_configs
        .into_iter()
        .filter(|c| {
            if let Some(threshold) = dog_thresholds.dogs.get(&c.name).filter(|t| !t.enabled) {
                klog!(
                    "[Ring 2] Backend '{}' DISABLED — skipping (reason: {:?})",
                    c.name,
                    threshold.skip_reason
                );
                return false;
            }
            true
        })
        .collect();

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
        slots_urls,
        fleet_meta,
        remediation_configs,
        dog_to_fleet_node,
    } = infra::boot::build_dogs_and_organ(backend_configs.clone(), &domain_prompts, &storage_port)
        .await;

    // Soma L2: Slot tracker — shared between health loop (writer) and Judge + REST (readers).
    let slot_tracker = Arc::new(domain::slot_tracker::SlotTracker::new());

    // ─── RING 2: Health Loop + Remediation ──────────────────────
    // Config comes from backends.toml (SoT) — no separate remediation.toml needed.
    if !remediation_configs.is_empty() {
        klog!(
            "[Ring 2] Remediation: {} Dogs configured for auto-restart",
            remediation_configs.len()
        );
    }

    // ─── RING 2: Embedding backend (sovereign, auto-recovery) ────
    let initial_embed: Arc<dyn domain::embedding::EmbeddingPort> = {
        if let Ok(url) = std::env::var("CYNIC_EMBED_URL") {
            let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
            let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".into());
            match backends::embedding::EmbeddingBackend::new(&url, api_key, &model) {
                Ok(b) if b.health().await.is_available() => {
                    klog!("[Ring 2] Embedding: explicit URL {} (sovereign)", url);
                    Arc::new(b)
                }
                _ => {
                    klog!("[Ring 2] Embedding: explicit URL {} unavailable", url);
                    Arc::new(domain::embedding::NullEmbedding)
                }
            }
        } else {
            let host = std::env::var("CYNIC_REST_ADDR")
                .unwrap_or_else(|_| domain::constants::DEFAULT_REST_ADDR.into())
                .split(':')
                .next()
                .unwrap_or("127.0.0.1")
                .to_string();
            let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
            let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".into());
            match backends::embedding::EmbeddingBackend::discover(&host, api_key, &model).await {
                Some(b) => {
                    klog!("[Ring 2] Embedding: discovered at {}", b.base_url());
                    Arc::new(b)
                }
                None => {
                    klog!(
                        "[Ring 2] Embedding: no server found — NullEmbedding, will auto-discover"
                    );
                    Arc::new(domain::embedding::NullEmbedding)
                }
            }
        }
    };
    let embedding: Arc<backends::auto_embed::AutoRecoveryEmbedding> = Arc::new(
        backends::auto_embed::AutoRecoveryEmbedding::new(initial_embed),
    );

    // ─── RING 2: Build Judge ──────────────────────────────────
    let breakers: Vec<Arc<dyn domain::health_gate::HealthGate>> = dogs
        .iter()
        .map(|d| {
            let dog_name = d.id().to_string();
            let mut breaker = infra::circuit_breaker::CircuitBreaker::new(dog_name.clone());

            // Apply data-driven thresholds from backends.toml if configured for this Dog
            if let Some(_threshold_cfg) = dog_thresholds.dogs.get(&dog_name) {
                breaker = breaker.with_thresholds(
                    dog_thresholds.circuit.open_on_consecutive_failures,
                    dog_thresholds.circuit.open_duration_secs,
                );
            } else {
                // Use global defaults from backends.toml
                breaker = breaker.with_thresholds(
                    dog_thresholds.circuit.open_on_consecutive_failures,
                    dog_thresholds.circuit.open_duration_secs,
                );
            }

            Arc::new(breaker) as Arc<dyn domain::health_gate::HealthGate>
        })
        .collect();
    // Daily budgets from backends.toml: restrict quota-constrained Dogs
    let budget_limits: Vec<(String, u32)> = backend_configs
        .iter()
        .filter(|c| c.daily_budget > 0)
        .map(|c| (c.name.clone(), c.daily_budget))
        .collect();
    let judge = judge::Judge::new(dogs, breakers)
        .with_organ_handles(organ_handles)
        .with_budgets(&budget_limits)
        .with_slot_tracker(Arc::clone(&slot_tracker));
    // Background task health tracker — updated by each spawned task, exposed in /health
    let task_health = Arc::new(infra::task_health::TaskHealth::new());
    // Lifecycle orchestration — all background tasks select! on this token.
    // Signal handler cancels it → tasks break at safe boundaries → drain → flush → exit.
    let shutdown = CancellationToken::new();

    // Seed integrity hash chain from last stored verdict + verify integrity
    let chain_verified = infra::boot::seed_integrity_chain(&storage_port, &judge).await;

    // Sense registry — best-effort organ readers (RTK, Hermes X, Tailscale)
    // Built early so health_loop can use Tailscale sense for fleet awareness.
    let sense_root = match std::env::current_dir() {
        Ok(p) => p.display().to_string(),
        Err(_) => String::new(),
    };
    let senses = senses::build_sense_registry(&sense_root);
    if !senses.is_empty() {
        klog!("[senses] {} organ(s) registered", senses.len());
        for s in &senses {
            klog!("[senses]   → {}", s.name());
        }
    }

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
                    .map(|url| {
                        let api_key = backend_configs
                            .iter()
                            .find(|c| c.name == *id)
                            .and_then(|c| c.api_key.clone());
                        infra::health_loop::DogProbeConfig {
                            dog_id: id.clone(),
                            health_url: url.clone(),
                            slots_url: slots_urls.get(id.as_str()).and_then(|opt| opt.clone()),
                            api_key,
                        }
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
                Arc::clone(&storage_port),
                senses.clone(),
                dog_to_fleet_node.clone(),
                Arc::clone(&slot_tracker),
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

    let probes: Vec<Arc<dyn domain::probe::Probe>> = vec![
        Arc::new(infra::probes::ResourceProbe::default()),
        Arc::new(infra::probes::BackupProbe::new(backup_dir)),
        Arc::new(infra::probes::ProcessProbe),
        Arc::new(infra::probes::PressureProbe),
        Arc::new(infra::probes::NetworkProbe),
        Arc::new(infra::probes::ProxyProbe),
        Arc::new(infra::probes::SomaProbe),
        Arc::new(fleet_probe),
    ];

    // ── Domain curations (D1-D6) for wisdom enrichment ──
    let curation_dir = std::env::var("CYNIC_CURATION_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("cynic-python/curation"));
    let domain_curations = match domain::wisdom::DomainCurations::load_from_path(&curation_dir) {
        Ok(curations) => {
            klog!(
                "[Boot] Domain curations loaded from {:?} ({} domains)",
                curation_dir,
                curations.loaded_domains.len()
            );
            Arc::new(curations)
        }
        Err(e) => {
            klog!(
                "[Boot] Domain curations failed to load: {} — wisdom enrichment will be empty",
                e
            );
            Arc::new(domain::wisdom::DomainCurations::new())
        }
    };

    // ── Token enricher (Helius) — optional, graceful degradation ──
    let enricher: Option<Arc<dyn domain::enrichment::TokenEnricherPort>> =
        match backends::helius::HeliusEnricher::from_env() {
            Some(h) => {
                let h = h.with_kscore_config(dog_thresholds.kscore.clone());
                klog!(
                    "[Boot] Helius enricher configured — token-analysis with K-Score behavioral analysis"
                );
                Some(Arc::new(h))
            }
            None => {
                klog!(
                    "[Boot] Helius enricher not configured — token-analysis will use raw addresses"
                );
                None
            }
        };

    // ─── Domain router — data-centric Dog selection ──────────────────
    // Initialized from backend_configs. Maps domain → suitable Dogs.
    let domain_router = Arc::new(infra::domain_router::DomainRouter::from_backends(
        &backend_configs,
    ));

    // ─── Routing calculator — dynamic Dog selection based on observed latencies ──
    // Consumes dog_performance observations and adapts routing in real-time.
    // Data-centric: routing reflects current performance, not static config.
    let routing_calc = Arc::new(infra::routing_calc::RoutingCalculator::new());

    // ─── Dog performance collector — K15 seam 3 producer ──
    // Aggregates latency/success observations from pipeline.on_dog callbacks.
    // Periodically flushes to routing_calc for live routing adaptation.
    let dog_perf_collector = Arc::new(infra::dog_performance::DogPerformanceCollector::new());

    // Soma L2: Slot semaphore map — per-Dog permit coordination.
    // Initialized with zero permits at boot; health loop upserts real slot counts.
    let slot_semaphores = Arc::new(domain::slot_semaphore::SlotSemaphoreMap::new());

    // Soma L4: Inference proxy routing table — sovereign backends only.
    let sovereign_flags: std::collections::HashMap<String, bool> = backend_configs
        .iter()
        .map(|c| (c.name.clone(), c.sovereign))
        .collect();
    let proxy_targets = Arc::new(api::rest::inference_proxy::ProxyTargets::from_fleet_meta(
        &fleet_meta,
        &sovereign_flags,
    ));
    if !proxy_targets.available().is_empty() {
        klog!(
            "[Ring 2] Soma L4 proxy: {} sovereign backend(s): {:?}",
            proxy_targets.available().len(),
            proxy_targets.available()
        );
    }

    // Event bus — broadcast channel for SSE/WebSocket subscribers.
    // Capacity 256: events are small JSON, subscribers should keep up.
    let (event_tx, _) = tokio::sync::broadcast::channel::<domain::events::KernelEvent>(256);

    // DORMANT: Organism recovery (K15 consumer: state history → crystal cache)
    // state history consumer not yet complete — deferred to next phase

    // ─── RING 2: Mail service (agentmail.to REST API) ──────────────────
    // K15 producer: syncs inbox every 5 minutes, emits observations for recovery emails
    let mail_backend: Option<Arc<dyn domain::mail::MailPort>> = {
        // Try to load agentmail config from env
        match backends::mail::AgentmailConfig::from_env("MAIL_USERNAME") {
            Some(config) => {
                let backend = backends::mail::AgentmailBackend::new(config);
                klog!("[Ring 2] Mail service: agentmail.to configured");
                Some(Arc::new(backend) as Arc<dyn domain::mail::MailPort>)
            }
            None => {
                klog!(
                    "[Ring 2] Mail service: AGENTMAIL_API_KEY or MAIL_USERNAME not configured — disabled"
                );
                None
            }
        }
    };

    let rest_state = Arc::new(api::rest::AppState {
        judge: judge_swap,
        storage: Arc::clone(&storage_port),
        coord: Arc::clone(&coord),
        embedding: Arc::clone(&embedding) as Arc<dyn domain::embedding::EmbeddingPort>,
        usage: Arc::clone(&usage_tracker),
        verdict_cache: Arc::clone(&verdict_cache),
        task_health: Arc::clone(&task_health),
        metrics: Arc::clone(&metrics),
        api_key,
        role_keys,
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
        senses,
        domain_curations: Arc::clone(&domain_curations),
        domain_router: Arc::clone(&domain_router),
        routing_calc: Arc::clone(&routing_calc),
        dog_perf_collector: Arc::clone(&dog_perf_collector),
        slot_semaphores: Arc::clone(&slot_semaphores),
        slot_tracker: Arc::clone(&slot_tracker),
        proxy_targets: Arc::clone(&proxy_targets),
        project_root: project_root.display().to_string(),
        mail: mail_backend.clone(),
    });
    let rest_app = api::rest::router(Arc::clone(&rest_state));

    // ─── Background tasks (REST-only — MCP is a thin tool-dispatch layer) ──
    // MCP mode skips all background tasks: the REST kernel already runs them.
    // Spawning duplicates causes SurrealDB contention, double nightshifts,
    // and 60s+ MCP startup delay that kills Hermes sessions.
    if !mcp_mode {
        // Background embedding discovery — retries every 60s if currently NullEmbedding
        {
            // Cast to dyn EmbeddingPort so the trait method is in scope inside the async block.
            let embed_ref: Arc<backends::auto_embed::AutoRecoveryEmbedding> =
                Arc::clone(&embedding);
            let embed_probe: Arc<dyn domain::embedding::EmbeddingPort> =
                Arc::clone(&embedding) as Arc<dyn domain::embedding::EmbeddingPort>;
            let host = std::env::var("CYNIC_REST_ADDR")
                .unwrap_or_else(|_| domain::constants::DEFAULT_REST_ADDR.into())
                .split(':')
                .next()
                .unwrap_or("127.0.0.1")
                .to_string();
            let api_key = std::env::var("SOVEREIGN_API_KEY").ok();
            let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".into());
            let shutdown = shutdown.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                interval.tick().await; // skip first tick
                loop {
                    tokio::select! {
                        _ = shutdown.cancelled() => break,
                        _ = interval.tick() => {
                            if embed_probe.embed("health-probe").await.is_err()
                                && let Some(backend) =
                                    backends::embedding::EmbeddingBackend::discover(
                                        &host,
                                        api_key.clone(),
                                        &model,
                                    )
                                    .await
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
            Some(Arc::clone(&organ)),
        );
        klog!("[Ring 2] Usage + organ stats flush task started (every 60s, TTL cleanup every 1h)");

        // ─── RING 2: Session summarizer (sovereign inference, background) ──
        if let Ok(summarizer) = infra::boot::build_summarizer(summarizer_backend_cfg.as_ref()) {
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
            Arc::clone(&embedding) as Arc<dyn domain::embedding::EmbeddingPort>,
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
            shutdown.clone(),
        );

        // ─── State log (hash-chained organism state, every 5min) ────
        infra::tasks::spawn_state_log(
            Arc::clone(&rest_state),
            Arc::clone(&task_health),
            shutdown.clone(),
        );

        // ─── Verdict Submission Queue (K15: auto-anchor to Pinocchio, every 5min) ────
        infra::tasks::spawn_submission_queue(
            Arc::clone(&storage_port),
            Arc::clone(&task_health),
            shutdown.clone(),
        );
        klog!("[Ring 2] Verdict submission queue task started (every 5min)");

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
            kernel_addr.clone(),
            api_key.clone(),
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
            slack.clone(),
        );

        // ─── Layer 4 Slack alerter: polls /observations?consumer=pattern_healing ────
        // K15 feedback loop: anomaly detected → observation emitted → alerter sends Slack
        infra::tasks::spawn_pattern_healing_alerter(
            kernel_addr,
            api_key,
            Arc::clone(&task_health),
            shutdown.clone(),
            slack,
        );
        klog!("[Ring 2] Pattern healing alerter started (K15 Layer 4 consumer)");

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

        infra::tasks::spawn_dog_perf_flush_loop(
            Arc::clone(&rest_state),
            Arc::clone(&task_health),
            shutdown.clone(),
        );
        klog!("[Ring 2] Dog performance flush loop started (every 60s, K15 seam 3 producer)");

        infra::tasks::spawn_discovery_loop(
            Arc::clone(&rest_state),
            fleet_meta,
            Arc::clone(&task_health),
            shutdown.clone(),
        );
        klog!("[Ring 2] Discovery loop started (every 60s, organism-agnostic)");

        // ─── Crystal immune system: DISABLED ──
        // Runs every 5min, re-judges crystals — same slot starvation as nightshift.
        // With only 1 CPU slot on qwen25-7b-core, any background loop blocks user /judge.
        // Re-enable when Soma L2 has priority queuing.
        klog!("[Ring 2] Crystal challenge loop DISABLED — GPU slots reserved for user requests");

        // ─── Nightshift: autonomous dev judgment (every 4h, Soma L3 gated) ───────
        // Was disabled 2026-05-11 (PR#135) due to slot starvation from Phase 2
        // (observations) bypassing the soma gate. Now both phases are gated.
        let _nightshift_handle = infra::tasks::spawn_nightshift_loop(
            rest_state.judge.load_full(),
            Arc::clone(&storage_port),
            Arc::clone(&task_health),
            shutdown.clone(),
            project_root.display().to_string(),
        );
        klog!(
            "[Ring 3] Nightshift loop started (every 4h, git lookback {}, Soma L3 gated)",
            crate::domain::constants::NIGHTSHIFT_GIT_LOOKBACK
        );
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
                "version={} dogs={}",
                env!("CARGO_PKG_VERSION"),
                rest_state.judge.load_full().dog_ids().len()
            ),
            session_id: String::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tags: vec!["kernel-self-obs".into()],
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
