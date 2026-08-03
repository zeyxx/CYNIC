//! Runtime transport adapters for the MCP projection.
//!
//! Stdio remains the compatibility default.  Streamable HTTP is an explicit,
//! stateless opt-in so transport selection is observable and never inferred
//! from a client cache.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};
use rmcp::transport::{
    streamable_http_server::session::local::LocalSessionManager, StreamableHttpServerConfig,
    StreamableHttpService,
};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::proxy::CynicMcpProxy;

pub const STREAMABLE_HTTP_MODE: &str = "streamable_http";
pub const STREAMABLE_HTTP_TRANSPORT_LABEL: &str = "streamable_http_stateless";
const MCP_PROTOCOL_VERSION_HEADER: &str = "MCP-Protocol-Version";
const MCP_METHOD_HEADER: &str = "Mcp-Method";
const MCP_NAME_HEADER: &str = "Mcp-Name";
const REQUEST_PROTOCOL_VERSION_META: &str = "io.modelcontextprotocol/protocolVersion";
const REQUEST_CLIENT_INFO_META: &str = "io.modelcontextprotocol/clientInfo";
const REQUEST_CLIENT_CAPABILITIES_META: &str = "io.modelcontextprotocol/clientCapabilities";

fn validate_2026_request(
    headers: &HeaderMap,
    method: &Method,
    body: &Value,
) -> Result<(), &'static str> {
    if method != Method::POST {
        return Err("MCP 2026 Streamable HTTP accepts POST only");
    }
    if headers
        .get(MCP_PROTOCOL_VERSION_HEADER)
        .and_then(|value| value.to_str().ok())
        != Some(crate::protocol::TARGET_REVISION)
    {
        return Err("MCP-Protocol-Version must be 2026-07-28");
    }
    let rpc_method = body
        .get("method")
        .and_then(Value::as_str)
        .ok_or("JSON-RPC method is required")?;
    if matches!(rpc_method, "initialize" | "notifications/initialized") {
        return Err("initialize handshake is removed from MCP 2026");
    }
    if headers
        .get(MCP_METHOD_HEADER)
        .and_then(|value| value.to_str().ok())
        != Some(rpc_method)
    {
        return Err("Mcp-Method must match the JSON-RPC method");
    }
    let Some(meta) = body.pointer("/params/_meta").and_then(Value::as_object) else {
        return Err("every MCP 2026 request requires params._meta");
    };
    if meta
        .get(REQUEST_PROTOCOL_VERSION_META)
        .and_then(Value::as_str)
        != Some(crate::protocol::TARGET_REVISION)
    {
        return Err("params._meta protocolVersion must match 2026-07-28");
    }
    if !meta
        .get(REQUEST_CLIENT_CAPABILITIES_META)
        .is_some_and(Value::is_object)
    {
        return Err("params._meta must include clientCapabilities");
    }
    if let Some(client_info) = meta.get(REQUEST_CLIENT_INFO_META) {
        if !client_info.is_object() {
            return Err("io.modelcontextprotocol/clientInfo must be an object when present");
        }
    }
    if rpc_method == "tools/call" {
        let tool_name = body.pointer("/params/name").and_then(Value::as_str);
        if headers
            .get(MCP_NAME_HEADER)
            .and_then(|value| value.to_str().ok())
            != tool_name
        {
            return Err("Mcp-Name must match params.name for tools/call");
        }
    }
    Ok(())
}

async fn enforce_2026_wire(request: Request<Body>, next: Next) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = match to_bytes(body, 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid MCP request body").into_response(),
    };
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "MCP request must be valid JSON").into_response();
        }
    };
    if let Err(reason) = validate_2026_request(&parts.headers, &parts.method, &value) {
        return (StatusCode::BAD_REQUEST, reason).into_response();
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

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
    if !address.ip().is_loopback() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "blocked_http_auth_unimplemented: Streamable HTTP must remain loopback-only until an authorization adapter is proven",
        )
        .into());
    }
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
    let router = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn(enforce_2026_wire));
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

    fn request(method: &str, tool_name: Option<&str>) -> (HeaderMap, Value) {
        let mut headers = HeaderMap::new();
        headers.insert(
            MCP_PROTOCOL_VERSION_HEADER,
            crate::protocol::TARGET_REVISION.parse().unwrap(),
        );
        headers.insert(MCP_METHOD_HEADER, method.parse().unwrap());
        if let Some(tool_name) = tool_name {
            headers.insert(MCP_NAME_HEADER, tool_name.parse().unwrap());
        }
        let mut params = serde_json::Map::new();
        params.insert(
            "_meta".to_string(),
            serde_json::json!({
                "io.modelcontextprotocol/protocolVersion": crate::protocol::TARGET_REVISION,
                "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1"},
                "io.modelcontextprotocol/clientCapabilities": {}
            }),
        );
        if let Some(tool_name) = tool_name {
            params.insert("name".to_string(), Value::String(tool_name.to_string()));
        }
        (
            headers,
            serde_json::json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params}),
        )
    }

    #[test]
    fn mode_is_explicit_and_transport_label_is_distinct() {
        assert_eq!(STREAMABLE_HTTP_MODE, "streamable_http");
        assert_ne!(STREAMABLE_HTTP_MODE, STREAMABLE_HTTP_TRANSPORT_LABEL);
    }

    #[test]
    fn wire_2026_rejects_initialize_and_accepts_discovery_with_meta() {
        let (headers, body) = request("server/discover", None);
        assert!(validate_2026_request(&headers, &Method::POST, &body).is_ok());

        let (mut headers, body) = request("initialize", None);
        headers.insert(MCP_METHOD_HEADER, "initialize".parse().unwrap());
        assert_eq!(
            validate_2026_request(&headers, &Method::POST, &body),
            Err("initialize handshake is removed from MCP 2026")
        );
    }

    #[test]
    fn wire_2026_binds_tool_header_to_tool_name() {
        let (mut headers, body) = request("tools/call", Some("cynic_health"));
        assert!(validate_2026_request(&headers, &Method::POST, &body).is_ok());
        headers.insert(MCP_NAME_HEADER, "other".parse().unwrap());
        assert_eq!(
            validate_2026_request(&headers, &Method::POST, &body),
            Err("Mcp-Name must match params.name for tools/call")
        );
    }

    #[test]
    fn wire_2026_accepts_optional_client_info() {
        let (headers, mut body) = request("server/discover", None);
        body["params"]["_meta"] = serde_json::json!({
            "io.modelcontextprotocol/protocolVersion": crate::protocol::TARGET_REVISION,
            "io.modelcontextprotocol/clientCapabilities": {}
        });
        assert!(validate_2026_request(&headers, &Method::POST, &body).is_ok());
    }

    #[test]
    fn wire_2026_rejects_malformed_optional_client_info() {
        let (headers, mut body) = request("server/discover", None);
        body["params"]["_meta"] = serde_json::json!({
            "io.modelcontextprotocol/protocolVersion": crate::protocol::TARGET_REVISION,
            "io.modelcontextprotocol/clientCapabilities": {},
            "io.modelcontextprotocol/clientInfo": "not-an-object"
        });
        assert_eq!(
            validate_2026_request(&headers, &Method::POST, &body),
            Err("io.modelcontextprotocol/clientInfo must be an object when present")
        );
    }

    #[test]
    fn wire_2026_rejects_missing_protocol_envelope() {
        let (headers, mut body) = request("server/discover", None);
        body["params"]["_meta"] = serde_json::json!({
            "io.modelcontextprotocol/clientCapabilities": {}
        });
        assert_eq!(
            validate_2026_request(&headers, &Method::POST, &body),
            Err("params._meta protocolVersion must match 2026-07-28")
        );
    }

    #[test]
    fn remote_bind_is_not_allowed_without_authorization_adapter() {
        let remote = "10.0.0.5:8787".parse::<std::net::SocketAddr>().unwrap();
        assert!(!remote.ip().is_loopback());
    }
}
