use std::sync::Arc;

use axum::{
    routing::{get, post},
    Json, Router, http::StatusCode,
};
use cynic_agent_bus::InMemoryLedger;

mod handlers;

pub fn create_app(ledger: Arc<InMemoryLedger>) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/tasks", post(handlers::create_task))
        .route("/tasks/:id", get(handlers::get_task))
        .with_state(ledger)
}

pub async fn run_server(port: u16, ledger: Arc<InMemoryLedger>) -> Result<(), anyhow::Error> {
    let app = create_app(ledger);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("cynic-agent-bus-server listening on :{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
