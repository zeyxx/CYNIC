use cynic_kernel::*;
use cynic_kernel::domain::chat::ChatPort;
use cynic_kernel::cynic_v2::vascular_system_server::VascularSystemServer;
use cynic_kernel::cynic_v2::k_pulse_server::KPulseServer;
use cynic_kernel::cynic_v2::muscle_hal_server::MuscleHalServer;

use tonic::transport::Server;
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

    // ─── RING 1: Vascular System (gRPC IPC) ──────────────────
    let addr = "[::1]:50051".parse()?;
    klog!("[Ring 1] Vascular Law enforced on {}", addr);

    // ─── RING 1: Muscle HAL (BackendRouter for gRPC) ──────────
    let router = Arc::new(backends::router::BackendRouter::new(vec![]));

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
    for cfg in backend_configs {
        let backend = Arc::new(backends::openai::OpenAiCompatBackend::new(cfg.clone()));
        let health = ChatPort::health(backend.as_ref()).await;
        match health {
            domain::inference::BackendStatus::Healthy | domain::inference::BackendStatus::Degraded { .. } => {
                klog!("[Ring 2] InferenceDog '{}' loaded (model: {}, health: {:?})", cfg.name, cfg.model, health);
                dogs.push(Box::new(dogs::inference::InferenceDog::new(backend, cfg.name.clone(), cfg.context_size)));
            }
            _ => {
                klog!("[Ring 2] WARNING: Backend '{}' unreachable, skipping", cfg.name);
            }
        }
    }

    klog!("[Ring 2] {} Dog(s) active", dogs.len());

    // ─── RING 2: Build Judge ──────────────────────────────────
    let judge = judge::Judge::new(dogs);

    // ─── RING 3: REST API (for React/external clients) ────────
    let judge = Arc::new(judge);
    let usage_tracker = Arc::new(std::sync::Mutex::new(domain::usage::DogUsageTracker::new()));
    let api_key = std::env::var("CYNIC_API_KEY").ok();
    let rest_state = Arc::new(api::rest::AppState {
        judge: Arc::clone(&judge),
        storage: Arc::clone(&storage_port),
        coord: Arc::clone(&coord),
        usage: Arc::clone(&usage_tracker),
        api_key,
        rate_limiter: api::rest::PerIpRateLimiter::new(30),   // 30 requests/minute global
        judge_limiter: api::rest::PerIpRateLimiter::new(10),   // 10 /judge per minute (inference costs money)
    });
    let rest_app = api::rest::router(rest_state);

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
        let _ = server.waiting().await;
        return Ok(());
    }

    let rest_addr = std::env::var("CYNIC_REST_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3030".to_string());
    klog!("[Ring 3] REST API on http://{}", rest_addr);

    // ─── Coordination expiry (background, every 60s) ──────────
    {
        let expiry_coord = Arc::clone(&coord);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await; // Skip first immediate tick
            loop {
                interval.tick().await;
                let _ = expiry_coord.expire_stale().await;
            }
        });
        klog!("[Ring 2] Coordination expiry task started (every 60s)");
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
            interval.tick().await; // Skip first immediate tick
            loop {
                interval.tick().await;
                domain::ccm::aggregate_observations(agg_storage.as_ref(), "CYNIC").await;
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

    // ─── gRPC services ────────────────────────────────────────
    let pulse_service = grpc::pulse::PulseService::default();
    let muscle_service = grpc::hal::MuscleService::new(Arc::clone(&router) as Arc<dyn domain::inference::InferenceRouter>);
    klog!("╔══════════════════════════════════════╗");
    klog!("║   CYNIC SOVEREIGN — ALL SYSTEMS GO   ║");
    klog!("║   REST: http://{}",  rest_addr);
    klog!("║   gRPC: {}",  addr);
    klog!("║   Max confidence: phi^-1 = 0.618     ║");
    klog!("╚══════════════════════════════════════╝");

    let grpc_server = Server::builder()
        .add_service(VascularSystemServer::new(grpc::vascular::VascularService::default()))
        .add_service(KPulseServer::new(pulse_service))
        .add_service(MuscleHalServer::new(muscle_service))
        .serve(addr);

    // Run both servers concurrently
    tokio::select! {
        _ = rest_server => eprintln!("[FATAL] REST server stopped"),
        r = grpc_server => {
            if let Err(e) = r {
                eprintln!("[FATAL] gRPC server error: {}", e);
            }
        }
    }

    Ok(())
}
