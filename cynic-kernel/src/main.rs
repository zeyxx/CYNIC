use cynic_kernel::*;
use cynic_kernel::domain::inference::BackendPort;
use std::sync::Arc;

// ============================================================
// BOOT SEQUENCE
// ============================================================
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse flags early — MCP mode needs stderr-only logging from the start.
    let force_reprobe = std::env::args().any(|a| a == "--reset");
    let mcp_mode = std::env::args().any(|a| a == "--mcp");

    // Set global MCP flag BEFORE any module has a chance to println! to stdout.
    if mcp_mode {
        cynic_kernel::MCP_MODE.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    klog!("╔══════════════════════════════════════╗");
    klog!("║       CYNIC OS V2 — SOVEREIGN BOOT    ║");
    klog!("╚══════════════════════════════════════╝");
    let node_config = probe::run(force_reprobe).await;

    klog!("[Ring 0] Omniscience Active. Reality Mapped.");
    klog!("[Ring 0] Host: {} | Compute: {:?} | VRAM: {}GB",
        std::env::consts::OS,
        node_config.compute.backend,
        node_config.compute.vram_gb
    );

    // ─── RING 1: Native Storage Client (UAL) ──────────────────
    // HTTP adapter to SurrealDB 3.x — graceful degradation if unavailable.
    let (storage_port, coord, has_db): (Arc<dyn domain::storage::StoragePort>, Arc<dyn domain::coord::CoordPort>, bool) =
        match storage::SurrealHttpStorage::init().await {
            Ok(s) => {
                klog!("[Ring 1] Storage: HEALTHY (SurrealDB HTTP)");
                let db = Arc::new(s);
                (Arc::clone(&db) as _, Arc::clone(&db) as _, true)
            }
            Err(e) => {
                eprintln!("[Ring 1] Storage: DEGRADED — {} (verdicts will not persist)", e);
                (Arc::new(domain::storage::NullStorage), Arc::new(domain::coord::NullCoord), false)
            }
        };

    // ─── RING 2: Load Backend Configs ──────────────────────────
    let backends_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cynic")
        .join("backends.toml");

    let backend_configs = if backends_path.exists() {
        klog!("[Ring 2] Loading backends from {}", backends_path.display());
        infra::config::load_backends(&backends_path)
    } else {
        klog!("[Ring 2] No backends.toml found, using env var fallback");
        infra::config::load_backends_from_env()
    };

    // Validate config — probe health URLs, log warnings (non-blocking)
    infra::config::validate_config(&backend_configs).await;

    // ─── RING 2: Build Dogs (model-agnostic evaluators) ───────
    let mut dogs: Vec<Box<dyn domain::dog::Dog>> = Vec::new();

    // Always add the deterministic Dog (free, fast)
    dogs.push(Box::new(dogs::deterministic::DeterministicDog));
    klog!("[Ring 2] DeterministicDog loaded");

    // Create InferenceDog per configured backend
    // Also collect health URLs and remediation configs from the SoT (backends.toml)
    let mut cost_rates: Vec<(String, f64, f64)> = Vec::new();
    let mut remediation_configs: std::collections::HashMap<String, infra::config::BackendRemediation> = std::collections::HashMap::new();
    let mut health_urls: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
    for cfg in backend_configs {
        let backend = Arc::new(backends::openai::OpenAiCompatBackend::new(cfg.clone()));
        let health = BackendPort::health(backend.as_ref()).await;
        // Always load the Dog — health loop will recover it if unreachable.
        // Skipping at boot prevents the health loop from ever probing it.
        match health {
            domain::inference::BackendStatus::Healthy | domain::inference::BackendStatus::Degraded { .. } => {
                klog!("[Ring 2] InferenceDog '{}' loaded (model: {}, health: {:?})", cfg.name, cfg.model, health);
            }
            _ => {
                klog!("[Ring 2] InferenceDog '{}' loaded (model: {}, health: {:?}) — health loop will recover", cfg.name, cfg.model, health);
            }
        }
        cost_rates.push((cfg.name.clone(), cfg.cost_input_per_mtok, cfg.cost_output_per_mtok));
        health_urls.insert(cfg.name.clone(), cfg.health_url.clone());
        if let Some(rem) = cfg.remediation.clone() {
            remediation_configs.insert(cfg.name.clone(), rem);
        }
        dogs.push(Box::new(dogs::inference::InferenceDog::new(backend, cfg.name.clone(), cfg.context_size)));
    }

    klog!("[Ring 2] {} Dog(s) active", dogs.len());

    // ─── RING 2: Health Loop + Remediation ──────────────────────
    // Config comes from backends.toml (SoT) — no separate remediation.toml needed.
    if !remediation_configs.is_empty() {
        klog!("[Ring 2] Remediation: {} Dogs configured for auto-restart", remediation_configs.len());
    }

    // ─── RING 2: Embedding backend (sovereign, graceful degrade) ─
    let embed_backend = backends::embedding::EmbeddingBackend::from_env();
    let embed_health = embed_backend.health().await;
    let embedding: Arc<dyn domain::embedding::EmbeddingPort> = if embed_health.is_available() {
        klog!("[Ring 2] Embedding: {:?} (sovereign)", embed_health);
        Arc::new(embed_backend)
    } else {
        klog!("[Ring 2] Embedding: {:?} — degrading to NullEmbedding", embed_health);
        Arc::new(domain::embedding::NullEmbedding)
    };

    // ─── RING 2: Build Judge ──────────────────────────────────
    let judge = judge::Judge::new(dogs);
    // Background task health tracker — updated by each spawned task, exposed in /health
    let task_health = Arc::new(infra::task_health::TaskHealth::new());

    // Seed integrity hash chain from last stored verdict
    if let Ok(Some(hash)) = storage_port.last_integrity_hash().await {
        judge.seed_chain(Some(hash.clone()));
        klog!("[Ring 2] Integrity chain seeded: {}…", &hash[..16.min(hash.len())]);
    }

    // ─── RING 2: Spawn health loop ────────────────────────────
    {
        let probe_configs: Vec<infra::health_loop::DogProbeConfig> = judge
            .dog_ids()
            .iter()
            .filter(|id| *id != "deterministic-dog") // in-process, always healthy
            .filter_map(|id| {
                // Health URL from backends.toml SoT — None for cloud APIs (no health endpoint)
                health_urls.get(id.as_str())
                    .and_then(|opt| opt.as_ref())
                    .map(|url| infra::health_loop::DogProbeConfig {
                        dog_id: id.clone(),
                        health_url: url.clone(),
                    })
            })
            .collect();

        if !probe_configs.is_empty() {
            let breaker_map: std::collections::HashMap<String, Arc<infra::circuit_breaker::CircuitBreaker>> = judge
                .breakers()
                .iter()
                .map(|cb| (cb.dog_id().to_string(), Arc::clone(cb)))
                .collect();

            let probe_breakers: Vec<Arc<infra::circuit_breaker::CircuitBreaker>> = probe_configs
                .iter()
                .filter_map(|pc| breaker_map.get(&pc.dog_id).cloned())
                .collect();

            infra::health_loop::spawn_health_loop(probe_configs, probe_breakers, Arc::clone(&task_health));
            klog!("[Ring 2] Health loop started (every {}s)", infra::circuit_breaker::PROBE_INTERVAL.as_secs());
        }
    }

    // ─── RING 2: Spawn remediation watcher ───────────────────
    if !remediation_configs.is_empty() {
        let rem_configs = remediation_configs.clone();
        let rem_breakers: Vec<Arc<infra::circuit_breaker::CircuitBreaker>> = judge
            .breakers()
            .iter()
            .map(Arc::clone)
            .collect();

        tokio::spawn(async move {
            let tracker = infra::remediation::RecoveryTracker::new();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await; // skip first tick

            loop {
                interval.tick().await;
                for cb in &rem_breakers {
                    let dog_id = cb.dog_id();
                    if let Some(open_duration) = cb.opened_since() {
                        // Remediation threshold: wait before attempting restart
                        const REMEDIATION_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(90);
                        if open_duration > REMEDIATION_THRESHOLD
                            && let Some(config) = rem_configs.get(dog_id)
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
                                    infra::remediation::ssh_restart(&node, &cmd)
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
                        // Dog is healthy — reset recovery tracker
                        tracker.reset(dog_id);
                    }
                }
            }
        });
        klog!("[Ring 2] Remediation watcher started (90s threshold, {} Dogs)", remediation_configs.len());
    }

    // ─── RING 3: REST API (for React/external clients) ────────
    let judge = Arc::new(judge);
    let usage_tracker = Arc::new(tokio::sync::Mutex::new(domain::usage::DogUsageTracker::new()));
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
            klog!("[Ring 2] Usage: loaded {} Dog histories ({} all-time requests)",
                rows.len(), usage.all_time_requests());
        }
        Err(e) => klog!("[Ring 2] Usage: failed to load history (non-fatal): {}", e),
        _ => {}
    }
    let api_key = std::env::var("CYNIC_API_KEY").ok();
    // Single VerdictCache shared by REST and MCP — avoids duplicate caches (T4 fix)
    let verdict_cache = Arc::new(domain::verdict_cache::VerdictCache::new());
    let rest_state = Arc::new(api::rest::AppState {
        judge: Arc::clone(&judge),
        storage: Arc::clone(&storage_port),
        coord: Arc::clone(&coord),
        embedding: Arc::clone(&embedding),
        usage: Arc::clone(&usage_tracker),
        verdict_cache: Arc::clone(&verdict_cache),
        task_health: Arc::clone(&task_health),
        api_key,
        rate_limiter: api::rest::PerIpRateLimiter::new(30),   // 30 requests/minute global
        judge_limiter: api::rest::PerIpRateLimiter::new(10),   // 10 /judge per minute (inference costs money)
    });
    let rest_app = api::rest::router(Arc::clone(&rest_state));

    // ─── Background tasks (universal — run in BOTH MCP and REST modes) ──
    // These MUST be spawned before the MCP/REST branch.

    // Coordination expiry (every 60s)
    {
        let expiry_coord = Arc::clone(&coord);
        let th = Arc::clone(&task_health);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await;
            loop {
                interval.tick().await;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    expiry_coord.expire_stale(),
                ).await {
                    Ok(Err(e)) => eprintln!("[coord] expire_stale failed: {}", e),
                    Err(_) => eprintln!("[coord] expire_stale timed out (10s)"),
                    _ => {}
                }
                th.touch_coord_expiry();
            }
        });
        klog!("[Ring 2] Coordination expiry task started (every 60s)");
    }

    // Usage flush (every 60s)
    if has_db {
        let flush_storage = Arc::clone(&storage_port);
        let flush_usage = Arc::clone(&usage_tracker);
        let th = Arc::clone(&task_health);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await;
            let mut tick_count: u64 = 0;
            loop {
                interval.tick().await;
                tick_count += 1;
                let snapshot = {
                    let usage = flush_usage.lock().await;
                    usage.snapshot()
                };
                if !snapshot.is_empty() {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        flush_storage.flush_usage(&snapshot),
                    ).await {
                        Ok(Ok(_)) => {
                            let mut usage = flush_usage.lock().await;
                            usage.absorb_flush();
                        }
                        Ok(Err(e)) => eprintln!("[flush] DB write failed, will retry next tick: {}", e),
                        Err(_) => eprintln!("[flush] DB write timed out (10s), will retry next tick"),
                    }
                }
                if tick_count.is_multiple_of(60)
                    && let Err(e) = flush_storage.cleanup_ttl().await
                {
                    eprintln!("[flush] TTL cleanup failed (non-fatal): {}", e);
                }
                th.touch_usage_flush();
            }
        });
        klog!("[Ring 2] Usage flush task started (every 60s, TTL cleanup every 1h)");
    }

    // CCM Workflow Aggregator (every 5 min)
    {
        let agg_storage = Arc::clone(&storage_port);
        let th = Arc::clone(&task_health);
        let interval_secs: u64 = std::env::var("CYNIC_AGGREGATE_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await;
            loop {
                interval.tick().await;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    domain::ccm::aggregate_observations(agg_storage.as_ref(), "CYNIC"),
                ).await {
                    Ok(_) => { th.touch_ccm_aggregate(); }
                    Err(_) => eprintln!("[CCM] aggregate_observations timed out (30s)"),
                }
            }
        });
        klog!("[Ring 2] CCM workflow aggregator started (every {}s)", interval_secs);
    }

    // ─── RING 2: Session summarizer (sovereign inference, background) ──
    // Self-recovering: checks availability each cycle, not just at boot (T6a fix)
    {
        let sum_storage = Arc::clone(&storage_port);
        let summarizer = backends::summarizer::SovereignSummarizer::from_env();
        let th = Arc::clone(&task_health);
        klog!("[Ring 2] Session summarizer task started (checks LLM availability each cycle)");
        tokio::spawn(async move {
            // Wait for first CCM cycle to populate observations
            tokio::time::sleep(std::time::Duration::from_secs(600)).await;
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                if !summarizer.is_available().await {
                    th.touch_summarizer(); // Task alive, LLM just unavailable
                    continue;
                }
                match tokio::time::timeout(
                    std::time::Duration::from_secs(120),
                    pipeline::summarize_pending_sessions(sum_storage.as_ref(), &summarizer),
                ).await {
                    Ok(count) => {
                        if count > 0 {
                            klog!("[CCM/summarizer] {} sessions summarized", count);
                        }
                        th.touch_summarizer();
                    }
                    Err(_) => eprintln!("[CCM/summarizer] timed out (120s)"),
                }
            }
        });
    }

    // ─── RING 3: MCP Server (for AI agents via stdio) ────────
    if mcp_mode {
        use rmcp::ServiceExt;
        eprintln!("[Ring 3] MCP mode — serving over stdio (background tasks active)");
        let mcp_infer: Arc<dyn domain::inference::InferPort> = Arc::new(
            backends::summarizer::SovereignSummarizer::from_env(),
        );
        let mcp_server = api::mcp::CynicMcp::new(
            Arc::clone(&judge),
            Arc::clone(&storage_port),
            Arc::clone(&coord),
            Arc::clone(&usage_tracker),
            Arc::clone(&embedding),
            Arc::clone(&verdict_cache),
            mcp_infer,
        );
        let transport = rmcp::transport::io::stdio();
        let server = mcp_server.serve(transport).await
            .map_err(|e| format!("MCP server error: {}", e))?;
        let _ = server.waiting().await; // ok: MCP server lifecycle

        // Flush usage on MCP shutdown
        if has_db {
            let snapshot = {
                let usage = usage_tracker.lock().await;
                usage.snapshot()
            };
            if !snapshot.is_empty() {
                match storage_port.flush_usage(&snapshot).await {
                    Ok(_) => eprintln!("[SHUTDOWN] MCP usage flushed"),
                    Err(e) => eprintln!("[SHUTDOWN] MCP usage flush failed: {}", e),
                }
            }
        }
        return Ok(());
    }

    // ─── RING 3: REST Server ─────────────────────────────────
    let rest_addr = std::env::var("CYNIC_REST_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3030".to_string());
    klog!("[Ring 3] REST API on http://{}", rest_addr);

    // Rate limiter eviction (REST-only, every 60s)
    {
        let evict_state = Arc::clone(&rest_state);
        let th = Arc::clone(&task_health);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await;
            loop {
                interval.tick().await;
                evict_state.rate_limiter.evict_stale().await;
                evict_state.judge_limiter.evict_stale().await;
                th.touch_rate_eviction();
            }
        });
    }

    let rest_listener = tokio::net::TcpListener::bind(&rest_addr).await?;
    let rest_server = tokio::spawn(async move {
        if let Err(e) = axum::serve(rest_listener, rest_app).await {
            eprintln!("[FATAL] REST server error: {}", e);
        }
    });

    klog!("╔══════════════════════════════════════╗");
    klog!("║   CYNIC SOVEREIGN — ALL SYSTEMS GO   ║");
    klog!("║   REST: http://{}",  rest_addr);
    klog!("║   Max confidence: phi^-1 = 0.618     ║");
    klog!("╚══════════════════════════════════════╝");

    tokio::select! {
        result = rest_server => {
            if let Err(e) = result {
                eprintln!("[FATAL] REST server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            klog!("[SHUTDOWN] SIGINT received — graceful shutdown");
        }
    }

    // Flush usage on shutdown — prevents up to 60s of data loss
    if has_db {
        let (snapshot, dog_count) = {
            let usage = usage_tracker.lock().await;
            (usage.snapshot(), usage.dogs.len())
        };
        if !snapshot.is_empty() {
            match storage_port.flush_usage(&snapshot).await {
                Ok(_) => klog!("[SHUTDOWN] Usage flushed ({} dogs)", dog_count),
                Err(e) => eprintln!("[SHUTDOWN] Usage flush failed: {}", e),
            }
        } else {
            klog!("[SHUTDOWN] No pending usage to flush");
        }
    }

    Ok(())
}

