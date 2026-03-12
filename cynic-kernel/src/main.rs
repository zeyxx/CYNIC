pub mod probe;
pub mod supervisor;
pub mod hal;
pub mod pulse;
pub mod storage;
pub mod backend;
pub mod backend_llamacpp;

use tonic::{transport::Server, Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use std::sync::Arc;

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
    // The Kernel connects to Sidecar Organs (Docker/Process)
    let storage = Arc::new(storage::CynicStorage::init().await?);
    
    // ─── RING 1: Vascular System (gRPC IPC) ──────────────────
    let addr = "[::1]:50051".parse()?;
    println!("[Ring 1] Vascular Law enforced on {}", addr);

    // TODO: Initialize real Pulse and HAL services with the new message types
    let pulse_service = pulse::PulseService::default();
    let muscle_service = hal::MuscleService::new(Arc::clone(&storage));
    let cognitive_service = storage::CognitiveMemoryService::new(Arc::clone(&storage));

    Server::builder()
        .add_service(VascularSystemServer::new(VascularService::default()))
        .add_service(KPulseServer::new(pulse_service))
        .add_service(MuscleHalServer::new(muscle_service))
        .add_service(CognitiveMemoryServer::new(cognitive_service))
        .serve(addr)
        .await?;

    Ok(())
}
