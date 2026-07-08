pub mod domain;
pub mod ports;
// pub mod adapters;
pub mod core;

use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("CYNIC Agent Gateway (Hexagonal Architecture) Starting...");

    // TODO: Instancier les adaptateurs concrets (SurrealDbMempoolAdapter, HermesCliAdapter, etc.)
    // let mempool = Arc::new(SurrealDbMempoolAdapter::new());
    // let observability = Arc::new(KernelObservabilityAdapter::new());
    // 
    // let mut orchestrator = core::GatewayOrchestrator::new(mempool, observability);
    // orchestrator.register_provider(Arc::new(HermesAdapter::new()));
    // orchestrator.register_provider(Arc::new(OpenCodeAdapter::new()));
    //
    // orchestrator.run_daemon().await;
}
