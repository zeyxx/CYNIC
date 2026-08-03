//! Runtime transport adapters for the MCP projection.
//!
//! Stdio remains the compatibility default.  Streamable HTTP is an explicit,
//! stateless opt-in so transport selection is observable and never inferred
//! from a client cache.

use std::sync::Arc;

use axum::Router;
use rmcp::transport::{
    StreamableHttpServerConfig, StreamableHttpService,
    streamable_http_server::session::local::LocalSessionManager,
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::proxy::CynicMcpProxy;

pub const STREAMABLE_HTTP_MODE: &str = "streamable_http";
pub const STREAMABLE_HTTP_TRANSPORT_LABEL: &str = "streamable_http_stateless";

/// Serve the MCP endpoint with the 2026-compatible stateless HTTP mode.
///
/// The underlying rmcp compatibility layer still accepts the legacy
/// initialize message, so callers must use the discovery read-back to know
/// which protocol dimensions are actually proven.
pub async fn serve_streamable_http(
    proxy: CynicMcpProxy,
    listen: &str,
    shutdown: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let address = listen.parse::<std::net::SocketAddr>()?;
    let config = StreamableHttpServerConfig::default()
        .with_stateful_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .with_cancellation_token(shutdown.clone());
    let service = StreamableHttpService::new(
        move || Ok(proxy.clone()),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let router = Router::new().nest_service("/mcp", service);
    let listener = TcpListener::bind(address).await?;
    tracing::info!(%address, transport = STREAMABLE_HTTP_TRANSPORT_LABEL, "cynic-mcp Streamable HTTP endpoint ready");
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { shutdown.cancelled_owned().await })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_is_explicit_and_transport_label_is_distinct() {
        assert_eq!(STREAMABLE_HTTP_MODE, "streamable_http");
        assert_ne!(STREAMABLE_HTTP_MODE, STREAMABLE_HTTP_TRANSPORT_LABEL);
    }
}
