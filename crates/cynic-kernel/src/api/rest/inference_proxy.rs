//! Soma L4: OpenAI-compatible inference proxy.
//!
//! `POST /v1/chat/completions` — gates inference requests through SlotTracker
//! before forwarding to the actual llama-server. Hermes (and any external consumer)
//! points its `base_url` here instead of directly at llama-server.
//!
//! Gate sequence:
//! 1. Select backend (query param `?backend=X` or first available)
//! 2. Check SlotTracker — if all slots busy, 503 + Retry-After
//! 3. Check per-slot context vs prompt size estimate
//! 4. Forward request to the real llama-server
//! 5. Return response verbatim

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use cynic_inference::application::proxy_routing::ProxyTargetSelectionError;
use std::sync::Arc;

use super::types::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct ProxyQueryParams {
    /// Target backend dog_id. If absent, uses first available sovereign backend.
    #[serde(
        default,
        deserialize_with = "super::types::deserialize_optional_dog_id"
    )]
    pub backend: Option<String>,
}

/// POST /v1/chat/completions — OpenAI-compatible proxy with Soma L4 gating.
///
/// Query params:
///   ?backend=qwen25-7b-core  — select specific backend
///
/// Returns:
///   200 + llama-server response (forwarded verbatim)
///   503 + Retry-After if all slots busy
///   502 if upstream unreachable
///   404 if backend not found
pub async fn proxy_chat_completions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ProxyQueryParams>,
    body: axum::body::Bytes,
) -> Response {
    let proxy = &state.proxy_targets;
    let json_body = match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(value) => Some(value),
        Err(err) => {
            tracing::debug!(error = %err, "ignoring non-JSON inference proxy body");
            None
        }
    };
    let requires_tools = if let Some(json) = &json_body {
        json.get("tools").is_some() || json.get("functions").is_some()
    } else {
        false
    };
    let json_model = json_body
        .as_ref()
        .and_then(|j| j.get("model"))
        .and_then(|m| m.as_str());

    // 1. Select backend
    let selection = match proxy.select(params.backend.as_deref(), json_model, requires_tools) {
        Ok(selection) => selection,
        Err(ProxyTargetSelectionError::BackendNotFound {
            requested,
            available,
        }) => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": {
                        "message": format!("backend '{}' not found. available: {:?}", requested, available),
                        "type": "invalid_request_error",
                    }
                })),
            )
                .into_response();
        }
        Err(ProxyTargetSelectionError::NoSovereignBackends) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(serde_json::json!({
                    "error": {
                        "message": "no sovereign backends configured",
                        "type": "server_error",
                    }
                })),
            )
                .into_response();
        }
    };
    let dog_id = selection.backend_id;
    let target = selection.target;

    let start_time = std::time::Instant::now();

    // 2. Check slot availability (Soma L2/L3 gate)
    if state.slot_tracker.all_slots_busy(&dog_id) {
        tracing::info!(
            dog_id = %dog_id,
            "Soma L4 proxy: all slots busy — returning 503"
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [("Retry-After", "5")],
            axum::Json(serde_json::json!({
                "error": {
                    "message": format!("all inference slots busy on '{dog_id}' — retry in 5s"),
                    "type": "server_error",
                }
            })),
        )
            .into_response();
    }

    // 3. Forward to the real llama-server
    let url = format!("{}/chat/completions", target.base_url.trim_end_matches('/'));

    let client = reqwest::Client::builder() // K2-exempt: HTTP proxy (REST→REST), not domain logic using reqwest
        .timeout(std::time::Duration::from_secs(120))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "error": {
                        "message": format!("proxy client init failed: {e}"),
                        "type": "server_error",
                    }
                })),
            )
                .into_response();
        }
    };

    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body);

    // Forward API key if the upstream expects one
    if let Some(ref key) = target.api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    tracing::info!(
        dog_id = %dog_id,
        upstream = %url,
        requires_tools = %requires_tools,
        "Soma L4 proxy: forwarding chat completion"
    );

    match req.send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let headers = resp.headers().clone();
            let stream = resp.bytes_stream();
            let body = axum::body::Body::from_stream(stream);
            let mut response = (status, body).into_response();
            // Forward content-type from upstream
            if let Some(ct) = headers.get("content-type") {
                response.headers_mut().insert("content-type", ct.clone());
            }

            // Inject observability headers (X-Cynic-*)
            if let Ok(hv) = axum::http::HeaderValue::from_str(&dog_id) {
                response.headers_mut().insert("x-cynic-backend-id", hv);
            }
            if let Ok(hv) = axum::http::HeaderValue::from_str(&target.model_name) {
                response.headers_mut().insert("x-cynic-model", hv);
            }
            if let Ok(hv) =
                axum::http::HeaderValue::from_str(&start_time.elapsed().as_millis().to_string())
            {
                response.headers_mut().insert("x-cynic-latency-ms", hv);
            }

            tracing::info!(
                dog_id = %dog_id,
                model = %target.model_name,
                latency_ms = start_time.elapsed().as_millis(),
                status = status.as_u16(),
                "Soma L4 proxy: completion stream started"
            );

            response
        }
        Err(e) => {
            let (status, msg) = if e.is_timeout() {
                (
                    StatusCode::GATEWAY_TIMEOUT,
                    format!("upstream timeout: {e}"),
                )
            } else {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("upstream unreachable: {e}"),
                )
            };
            (
                status,
                axum::Json(serde_json::json!({
                    "error": {
                        "message": msg,
                        "type": "server_error",
                    }
                })),
            )
                .into_response()
        }
    }
}

/// GET /v1/models — forward to upstream so Hermes can discover available models.
pub async fn proxy_models(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ProxyQueryParams>,
) -> Response {
    let proxy = &state.proxy_targets;

    let selection = match proxy.select(params.backend.as_deref(), None, false) {
        Ok(selection) => selection,
        Err(ProxyTargetSelectionError::BackendNotFound { requested, .. }) => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": {
                        "message": format!("backend '{}' not found", requested),
                        "type": "invalid_request_error",
                    }
                })),
            )
                .into_response();
        }
        Err(ProxyTargetSelectionError::NoSovereignBackends) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(serde_json::json!({
                    "error": {
                        "message": "no sovereign backends configured",
                        "type": "server_error",
                    }
                })),
            )
                .into_response();
        }
    };
    let target = selection.target;

    let url = format!("{}/models", target.base_url.trim_end_matches('/'));
    let Ok(client) = reqwest::Client::builder() // K2-exempt: HTTP proxy (REST→REST)
        .timeout(std::time::Duration::from_secs(5))
        .build()
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "proxy client init failed",
        )
            .into_response();
    };

    let mut req = client.get(&url);
    if let Some(ref key) = target.api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    match req.send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let stream = resp.bytes_stream();
            let body = axum::body::Body::from_stream(stream);
            let mut response = (status, body).into_response();
            response.headers_mut().insert(
                "content-type",
                "application/json"
                    .parse()
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/json")),
            );
            response
        }
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}
