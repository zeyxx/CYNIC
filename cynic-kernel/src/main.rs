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
    let raw_db: Option<Arc<storage::SurrealHttpStorage>> = match storage::SurrealHttpStorage::init().await {
        Ok(s) => {
            klog!("[Ring 1] Storage: HEALTHY (SurrealDB HTTP)");
            Some(Arc::new(s))
        }
        Err(e) => {
            eprintln!("[Ring 1] Storage: DEGRADED — {} (verdicts will not persist)", e);
            None
        }
    };
    let storage_port: Arc<dyn domain::storage::StoragePort> = match &raw_db {
        Some(s) => Arc::clone(s) as Arc<dyn domain::storage::StoragePort>,
        None => Arc::new(domain::storage::NullStorage),
    };
    let coord: Arc<dyn domain::coord::CoordPort> = match &raw_db {
        Some(s) => Arc::clone(s) as Arc<dyn domain::coord::CoordPort>,
        None => Arc::new(domain::coord::NullCoord),
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

    // ─── RING 2: Build Dogs (model-agnostic evaluators) ───────
    let mut dogs: Vec<Box<dyn domain::dog::Dog>> = Vec::new();

    // Always add the deterministic Dog (free, fast)
    dogs.push(Box::new(dogs::deterministic::DeterministicDog));
    klog!("[Ring 2] DeterministicDog loaded");

    // Create InferenceDog per configured backend
    let mut cost_rates: Vec<(String, f64, f64)> = Vec::new();
    for cfg in backend_configs {
        let backend = Arc::new(backends::openai::OpenAiCompatBackend::new(cfg.clone()));
        let health = BackendPort::health(backend.as_ref()).await;
        match health {
            domain::inference::BackendStatus::Healthy | domain::inference::BackendStatus::Degraded { .. } => {
                klog!("[Ring 2] InferenceDog '{}' loaded (model: {}, health: {:?})", cfg.name, cfg.model, health);
                cost_rates.push((cfg.name.clone(), cfg.cost_input_per_mtok, cfg.cost_output_per_mtok));
                dogs.push(Box::new(dogs::inference::InferenceDog::new(backend, cfg.name.clone(), cfg.context_size)));
            }
            _ => {
                klog!("[Ring 2] WARNING: Backend '{}' unreachable, skipping", cfg.name);
            }
        }
    }

    klog!("[Ring 2] {} Dog(s) active", dogs.len());

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

    // Seed integrity hash chain from last stored verdict
    if let Some(db) = &raw_db {
        let chain_hash = db.query_one("SELECT integrity_hash FROM verdict ORDER BY created_at DESC LIMIT 1;").await
            .ok()
            .and_then(|rows| rows.first().and_then(|r| r["integrity_hash"].as_str().filter(|s| !s.is_empty()).map(|s| s.to_string())).clone());
        if let Some(ref hash) = chain_hash {
            judge.seed_chain(chain_hash.clone());
            klog!("[Ring 2] Integrity chain seeded: {}…", &hash[..16.min(hash.len())]);
        }
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
    if let Some(db) = &raw_db {
        match db.query_one("SELECT * FROM dog_usage;").await {
            Ok(rows) => {
                let mut usage = usage_tracker.lock().await;
                usage.load_historical(&rows);
                klog!("[Ring 2] Usage: loaded {} Dog histories ({} all-time requests)",
                    rows.len(), usage.all_time_requests());
            }
            Err(e) => klog!("[Ring 2] Usage: failed to load history (non-fatal): {}", e),
        }
    }
    let api_key = std::env::var("CYNIC_API_KEY").ok();
    let rest_state = Arc::new(api::rest::AppState {
        judge: Arc::clone(&judge),
        storage: Arc::clone(&storage_port),
        coord: Arc::clone(&coord),
        embedding: Arc::clone(&embedding),
        usage: Arc::clone(&usage_tracker),
        verdict_cache: domain::verdict_cache::VerdictCache::new(),
        api_key,
        rate_limiter: api::rest::PerIpRateLimiter::new(30),   // 30 requests/minute global
        judge_limiter: api::rest::PerIpRateLimiter::new(10),   // 10 /judge per minute (inference costs money)
    });
    let rest_app = api::rest::router(Arc::clone(&rest_state));

    // ─── RING 3: MCP Server (for AI agents via stdio) ────────
    if mcp_mode {
        use rmcp::ServiceExt;
        eprintln!("[Ring 3] MCP mode — serving over stdio");
        let mcp_server = api::mcp::CynicMcp::new(
            Arc::clone(&judge),
            Arc::clone(&storage_port),
            Arc::clone(&coord),
            Arc::clone(&usage_tracker),
        );
        let transport = rmcp::transport::io::stdio();
        let server = mcp_server.serve(transport).await
            .map_err(|e| format!("MCP server error: {}", e))?;
        let _ = server.waiting().await; // ok: MCP server lifecycle
        return Ok(());
    }

    let rest_addr = std::env::var("CYNIC_REST_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3030".to_string());
    klog!("[Ring 3] REST API on http://{}", rest_addr);

    // ─── Coordination expiry + rate limiter eviction (background, every 60s) ──
    {
        let expiry_coord = Arc::clone(&coord);
        let evict_state = Arc::clone(&rest_state);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await; // Skip first immediate tick
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
                evict_state.rate_limiter.evict_stale().await;
                evict_state.judge_limiter.evict_stale().await;
            }
        });
        klog!("[Ring 2] Coordination expiry task started (every 60s)");
    }

    // ─── Usage flush (background, every 60s) ──────────────────
    if let Some(db) = &raw_db {
        let flush_storage = Arc::clone(&storage_port);
        let flush_raw = Arc::clone(db); // raw access for TTL cleanup (infrastructure, not domain)
        let flush_usage = Arc::clone(&usage_tracker);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await; // Skip first immediate tick
            let mut tick_count: u64 = 0;
            loop {
                interval.tick().await;
                tick_count += 1;
                // Snapshot + flush via StoragePort (no SQL in domain)
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
                // Periodic cleanup every hour (60 ticks × 60s)
                // Simple date-based DELETEs — no subqueries, no transaction timeouts.
                // Previous subquery (SELECT ... LIMIT 1 START 10000) caused 10s timeouts
                // and transaction drops in SurrealDB, which cascaded into 401 errors.
                // Periodic TTL cleanup every hour — infrastructure concern, uses raw DB
                if tick_count.is_multiple_of(60) {
                    let _ = flush_raw.query_one( // ok: TTL cleanup, best-effort
                        "DELETE observation WHERE created_at < time::now() - 30d;"
                    ).await;
                    let _ = flush_raw.query_one( // ok: TTL cleanup, best-effort
                        "DELETE mcp_audit WHERE ts < time::now() - 7d;"
                    ).await;
                }
            }
        });
        klog!("[Ring 2] Usage flush task started (every 60s, obs cleanup every 1h)");
    }

    // ─── CCM Workflow Aggregator (periodic, every 5 min) ──────
    {
        let agg_storage = Arc::clone(&storage_port);
        let interval_secs: u64 = std::env::var("CYNIC_AGGREGATE_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300); // 5 minutes default
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await; // Skip first immediate tick
            loop {
                interval.tick().await;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    domain::ccm::aggregate_observations(agg_storage.as_ref(), "CYNIC"),
                ).await {
                    Ok(_) => {}
                    Err(_) => eprintln!("[CCM] aggregate_observations timed out (30s)"),
                }
            }
        });
        klog!("[Ring 2] CCM workflow aggregator started (every {}s)", interval_secs);
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
    if raw_db.is_some() {
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
