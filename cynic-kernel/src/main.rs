use cynic_kernel::*;
use cynic_kernel::cynic_v2::vascular_system_server::{VascularSystem, VascularSystemServer};
use cynic_kernel::cynic_v2::k_pulse_server::KPulseServer;
use cynic_kernel::cynic_v2::cognitive_memory_server::CognitiveMemoryServer;
use cynic_kernel::cynic_v2::muscle_hal_server::MuscleHalServer;
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
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════╗");
    println!("║       CYNIC OS V2 — SOVEREIGN BOOT    ║");
    println!("╚══════════════════════════════════════╝");

    // ─── RING 0: Omniscience & Probing ────────────────────────
    let force_reprobe = std::env::args().any(|a| a == "--reset");
    let node_config = probe::run(force_reprobe).await;

    println!("[Ring 0] Omniscience Active. Reality Mapped.");
    println!("[Ring 0] Host: {} | Compute: {:?} | VRAM: {}GB",
        std::env::consts::OS,
        node_config.compute.backend,
        node_config.compute.vram_gb
    );

    // ─── RING 1: Native Storage Client (UAL) ──────────────────
    // The Kernel connects to Sidecar Organs (Docker/Process)
    let storage = Arc::new(storage::CynicStorage::init().await?);
    
    // ─── RING 1: Vascular System (gRPC IPC) ──────────────────
    let addr = "[::1]:50051".parse()?;
    println!("[Ring 1] Vascular Law enforced on {}", addr);

    // ─── RING 1: Muscle HAL (Inference Router) ──────────────────
    let llama_endpoint = std::env::var("CYNIC_LLAMA_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());

    let router = {
        let r = Arc::new(router::BackendRouter::new(vec![]));
        let r_clone = Arc::clone(&r);
        let endpoint = llama_endpoint.clone();
        tokio::spawn(async move {
            match backend_llamacpp::LlamaCppBackend::connect(&endpoint, "local-llama").await {
                Ok(backend) => {
                    println!("[Ring 1] LlamaCpp connected: {} | models: {:?}",
                        endpoint, backend.capability().loaded_models);
                    r_clone.register(Arc::new(backend)).await;
                }
                Err(e) => {
                    println!("[Ring 1] WARNING: LlamaCpp unavailable: {}", e);
                    println!("[Ring 1] Set CYNIC_LLAMA_ENDPOINT to connect.");
                }
            }
        });
        r
    };

    // ─── Background: Periodic health probe (circuit breaker) ──────
    {
        let probe_router = Arc::clone(&router);
        tokio::spawn(async move {
            // Initial probe to transition backends from UNKNOWN → real state
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            loop {
                probe_router.probe_all().await;
                let statuses = probe_router.backend_statuses().await;
                for (id, status) in &statuses {
                    println!("[Health] {} → {:?}", id, status);
                }
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            }
        });
    }

    // ─── RING 2: Build Dogs (model-agnostic evaluators) ───────
    let mut dogs: Vec<Box<dyn dog::Dog>> = Vec::new();

    // Always add the deterministic Dog (free, fast)
    dogs.push(Box::new(deterministic_dog::DeterministicDog));
    println!("[Ring 2] DeterministicDog loaded");

    // Add GeminiDog if API key is available
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        let model = std::env::var("GEMINI_MODEL")
            .unwrap_or_else(|_| "gemini-2.5-flash".to_string());
        println!("[Ring 2] GeminiDog loaded (model: {})", model);
        dogs.push(Box::new(gemini_dog::GeminiDog::with_model(api_key, model)));
    } else {
        println!("[Ring 2] GEMINI_API_KEY not set — running deterministic-only mode");
    }

    println!("[Ring 2] {} Dog(s) active", dogs.len());

    // ─── RING 2: Build Judge ──────────────────────────────────
    let judge = judge::Judge::new(dogs);

    // ─── RING 3: REST API (for React/external clients) ────────
    let rest_state = Arc::new(rest::AppState {
        judge,
        storage: Arc::clone(&storage),
    });
    let rest_app = rest::router(rest_state);
    let rest_addr = "0.0.0.0:3000";
    println!("[Ring 3] REST API on http://{}", rest_addr);

    let rest_listener = tokio::net::TcpListener::bind(rest_addr).await?;
    let rest_server = tokio::spawn(async move {
        axum::serve(rest_listener, rest_app).await.unwrap();
    });

    // ─── gRPC services ────────────────────────────────────────
    let pulse_service = pulse::PulseService::default();
    let muscle_service = hal::MuscleService::new(Arc::clone(&router));
    let cognitive_service = storage::CognitiveMemoryService::new(Arc::clone(&storage));

    println!("╔══════════════════════════════════════╗");
    println!("║   CYNIC SOVEREIGN — ALL SYSTEMS GO   ║");
    println!("║   REST: http://{}          ║", rest_addr);
    println!("║   gRPC: {}                ║", addr);
    println!("║   Max confidence: phi^-1 = 0.618     ║");
    println!("╚══════════════════════════════════════╝");

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
