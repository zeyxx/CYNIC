pub mod probe;
pub mod supervisor;
pub mod hal;
pub mod pulse;
pub mod storage;
pub mod backend;
pub mod backend_llamacpp;
pub mod router;

use tonic::{transport::Server, Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use std::sync::Arc;
use backend::InferencePort;

pub mod cynic_v2 {
    tonic::include_proto!("cynic.v2");
}

use cynic_v2::vascular_system_server::{VascularSystem, VascularSystemServer};
use cynic_v2::k_pulse_server::KPulseServer;
use cynic_v2::cognitive_memory_server::CognitiveMemoryServer;
use cynic_v2::muscle_hal_server::MuscleHalServer;
use cynic_v2::{Event, PublishAck, SubscriptionFilter};

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
    let storage = match storage::CynicStorage::init().await {
        Ok(s) => {
            println!("[Ring 1] CognitiveMemory connected to SurrealDB");
            Some(Arc::new(s))
        }
        Err(e) => {
            println!("[Ring 1] WARNING: SurrealDB unavailable: {}. Kernel starts without memory.", e);
            None
        }
    };
    
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

    let pulse_service = pulse::PulseService::default();
    let muscle_service = hal::MuscleService::new(Arc::clone(&router));
    let cognitive_service = storage.as_ref().map(|s| storage::CognitiveMemoryService::new(Arc::clone(s)));

    let mut builder = Server::builder()
        .add_service(VascularSystemServer::new(VascularService::default()))
        .add_service(KPulseServer::new(pulse_service))
        .add_service(MuscleHalServer::new(muscle_service));

    if let Some(cog) = cognitive_service {
        builder = builder.add_service(CognitiveMemoryServer::new(cog));
        println!("[Ring 1] CognitiveMemory service registered");
    }

    builder.serve(addr).await?;

    Ok(())
}
