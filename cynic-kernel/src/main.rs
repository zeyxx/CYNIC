use cynic_kernel::*;
use cynic_kernel::chat_port::ChatPort;
use cynic_kernel::cynic_v2::vascular_system_server::{VascularSystem, VascularSystemServer};
use cynic_kernel::cynic_v2::k_pulse_server::KPulseServer;
use cynic_kernel::cynic_v2::cognitive_memory_server::CognitiveMemoryServer;
use cynic_kernel::cynic_v2::muscle_hal_server::MuscleHalServer;
use cynic_kernel::storage_http;
use cynic_kernel::cynic_v2::{Event, PublishAck, SubscriptionFilter};

use tonic::{transport::Server, Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use std::sync::Arc;

// ============================================================
// VASCULAR SYSTEM — gRPC IPC Event Bus
// ============================================================
#[derive(Debug, Default)]
pub struct VascularService {}

#[tonic::async_trait]
impl VascularSystem for VascularService {
    type SubscribeEventsStream = ReceiverStream<Result<Event, Status>>;

    async fn publish_event(
        &self,
        request: Request<Event>,
    ) -> Result<Response<PublishAck>, Status> {
        let event = request.into_inner();
        let meta = event.meta.as_ref();
        
        println!("[Vascular] Event [{}] from {} | Trace: {}",
            event.topic,
            meta.map(|m| m.node_id.as_str()).unwrap_or("unknown"),
            meta.map(|m| m.trace_id.as_str()).unwrap_or("none")
        );

        Ok(Response::new(PublishAck {
            meta: event.meta,
            success: true,
        }))
    }

    async fn subscribe_events(
        &self,
        request: Request<SubscriptionFilter>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let filter = request.into_inner();
        println!("[Vascular] New subscriber for topics: {:?}", filter.topics);
        let (_, rx) = mpsc::channel(4);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

// ============================================================
// BOOT SEQUENCE
// ============================================================
/// Log macro that routes to stderr in MCP mode (stdout is reserved for JSON-RPC).
macro_rules! log {
    ($mcp:expr, $($arg:tt)*) => {
        if $mcp {
            eprintln!($($arg)*);
        } else {
            println!($($arg)*);
        }
    };
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse flags early — MCP mode needs stderr-only logging from the start.
    let force_reprobe = std::env::args().any(|a| a == "--reset");
    let mcp_mode = std::env::args().any(|a| a == "--mcp");

    // Set global MCP flag BEFORE any module has a chance to println! to stdout.
    if mcp_mode {
        cynic_kernel::MCP_MODE.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    log!(mcp_mode, "╔══════════════════════════════════════╗");
    log!(mcp_mode, "║       CYNIC OS V2 — SOVEREIGN BOOT    ║");
    log!(mcp_mode, "╚══════════════════════════════════════╝");
    let node_config = probe::run(force_reprobe).await;

    log!(mcp_mode, "[Ring 0] Omniscience Active. Reality Mapped.");
    log!(mcp_mode, "[Ring 0] Host: {} | Compute: {:?} | VRAM: {}GB",
        std::env::consts::OS,
        node_config.compute.backend,
        node_config.compute.vram_gb
    );

    // ─── RING 1: Native Storage Client (UAL) ──────────────────
    // HTTP adapter to SurrealDB 3.x — graceful degradation if unavailable.
    let storage: Option<Arc<storage_http::SurrealHttpStorage>> = match storage_http::SurrealHttpStorage::init().await {
        Ok(s) => {
            log!(mcp_mode, "[Ring 1] Storage: HEALTHY (SurrealDB HTTP)");
            Some(Arc::new(s))
        }
        Err(e) => {
            eprintln!("[Ring 1] Storage: DEGRADED — {} (verdicts will not persist)", e);
            None
        }
    };
    
    // ─── RING 1: Vascular System (gRPC IPC) ──────────────────
    let addr = "[::1]:50051".parse()?;
    log!(mcp_mode, "[Ring 1] Vascular Law enforced on {}", addr);

    // ─── RING 1: Muscle HAL (BackendRouter for gRPC) ──────────
    let router = Arc::new(router::BackendRouter::new(vec![]));

    // ─── RING 2: Load Backend Configs ──────────────────────────
    let backends_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cynic")
        .join("backends.toml");

    let backend_configs = if backends_path.exists() {
        log!(mcp_mode, "[Ring 2] Loading backends from {}", backends_path.display());
        config::load_backends(&backends_path)
    } else {
        log!(mcp_mode, "[Ring 2] No backends.toml found, using env var fallback");
        config::load_backends_from_env()
    };

    // ─── RING 2: Build Dogs (model-agnostic evaluators) ───────
    let mut dogs: Vec<Box<dyn dog::Dog>> = Vec::new();

    // Always add the deterministic Dog (free, fast)
    dogs.push(Box::new(deterministic_dog::DeterministicDog));
    log!(mcp_mode, "[Ring 2] DeterministicDog loaded");

    // Create InferenceDog per configured backend
    for cfg in backend_configs {
        let backend = Arc::new(backend_openai::OpenAiCompatBackend::new(cfg.clone()));
        let health = ChatPort::health(backend.as_ref()).await;
        match health {
            backend::BackendStatus::Healthy | backend::BackendStatus::Degraded { .. } => {
                log!(mcp_mode, "[Ring 2] InferenceDog '{}' loaded (model: {}, health: {:?})", cfg.name, cfg.model, health);
                dogs.push(Box::new(inference_dog::InferenceDog::new(backend, cfg.name.clone())));
            }
            _ => {
                log!(mcp_mode, "[Ring 2] WARNING: Backend '{}' unreachable, skipping", cfg.name);
            }
        }
    }

    log!(mcp_mode, "[Ring 2] {} Dog(s) active", dogs.len());

    // ─── RING 2: Build Judge ──────────────────────────────────
    let judge = judge::Judge::new(dogs);

    // ─── RING 3: REST API (for React/external clients) ────────
    let storage_port: Arc<dyn storage_port::StoragePort> = match &storage {
        Some(s) => Arc::clone(s) as Arc<dyn storage_port::StoragePort>,
        None => Arc::new(storage_port::NullStorage),
    };
    let judge = Arc::new(judge);
    let usage_tracker = Arc::new(std::sync::Mutex::new(rest::DogUsageTracker::new()));
    let rest_state = Arc::new(rest::AppState {
        judge: Arc::clone(&judge),
        storage: Arc::clone(&storage_port),
        usage: std::sync::Mutex::new(rest::DogUsageTracker::new()),
    });
    let rest_app = rest::router(rest_state);

    // ─── RING 3: MCP Server (for AI agents via stdio) ────────
    if mcp_mode {
        use rmcp::ServiceExt;
        eprintln!("[Ring 3] MCP mode — serving over stdio");
        let mcp_server = mcp::CynicMcp::new(
            Arc::clone(&judge),
            Arc::clone(&storage_port),
            Arc::clone(&usage_tracker),
        );
        let transport = rmcp::transport::io::stdio();
        let server = mcp_server.serve(transport).await
            .map_err(|e| format!("MCP server error: {}", e))?;
        let _ = server.waiting().await;
        return Ok(());
    }

    let rest_addr = std::env::var("CYNIC_REST_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3030".to_string());
    log!(mcp_mode, "[Ring 3] REST API on http://{}", rest_addr);

    let rest_listener = tokio::net::TcpListener::bind(&rest_addr).await?;
    let rest_server = tokio::spawn(async move {
        axum::serve(rest_listener, rest_app).await.unwrap();
    });

    // ─── gRPC services ────────────────────────────────────────
    let pulse_service = pulse::PulseService::default();
    let muscle_service = hal::MuscleService::new(Arc::clone(&router));
    let cognitive_service = storage_http::CognitiveMemoryService::new(storage);

    log!(mcp_mode, "╔══════════════════════════════════════╗");
    log!(mcp_mode, "║   CYNIC SOVEREIGN — ALL SYSTEMS GO   ║");
    log!(mcp_mode, "║   REST: http://{}",  rest_addr);
    log!(mcp_mode, "║   gRPC: {}",  addr);
    log!(mcp_mode, "║   Max confidence: phi^-1 = 0.618     ║");
    log!(mcp_mode, "╚══════════════════════════════════════╝");

    let grpc_server = Server::builder()
        .add_service(VascularSystemServer::new(VascularService::default()))
        .add_service(KPulseServer::new(pulse_service))
        .add_service(MuscleHalServer::new(muscle_service))
        .add_service(CognitiveMemoryServer::new(cognitive_service))
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
