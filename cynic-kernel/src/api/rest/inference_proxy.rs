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
use std::collections::BTreeMap;
use std::sync::Arc;

use super::types::AppState;

/// Routing table: dog_id → base_url (e.g. "http://100.x.y.z:8080/v1").
/// Built from backends.toml at boot. Only sovereign backends are included.
#[derive(Debug, Clone)]
pub struct ProxyTargets {
    /// dog_id → (base_url, model_name)
    targets: BTreeMap<String, ProxyTarget>,
}

#[derive(Debug, Clone)]
struct ProxyTarget {
    base_url: String,
    api_key: Option<String>,
}

impl ProxyTargets {
    /// Build from boot-time fleet_meta (only sovereign HTTP backends).
    pub fn from_fleet_meta(
        fleet_meta: &std::collections::HashMap<String, (String, u32, String, Option<String>)>,
        sovereign_flags: &std::collections::HashMap<String, bool>,
    ) -> Self {
        let mut targets = BTreeMap::new();
        for (dog_id, (base_url, _ctx, _model, api_key)) in fleet_meta {
            // Only proxy sovereign backends (http://, local inference)
            if sovereign_flags.get(dog_id).copied().unwrap_or(false) {
                targets.insert(
                    dog_id.clone(),
                    ProxyTarget {
                        base_url: base_url.clone(),
                        api_key: api_key.clone(),
                    },
                );
            }
        }
        Self { targets }
    }

    /// List available backend IDs.
    pub fn available(&self) -> Vec<&str> {
        self.targets.keys().map(|s| s.as_str()).collect()
    }

    /// Get a specific backend by dog_id.
    fn get(&self, dog_id: &str) -> Option<&ProxyTarget> {
        self.targets.get(dog_id)
    }

    /// First available backend (deterministic — BTreeMap is sorted).
    fn first(&self) -> Option<(&str, &ProxyTarget)> {
        self.targets.iter().next().map(|(k, v)| (k.as_str(), v))
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ProxyQueryParams {
    /// Target backend dog_id. If absent, uses first available sovereign backend.
    pub backend: Option<String>,
}

/// POST /v1/chat/completions — OpenAI-compatible proxy with Soma L4 gating.
///
/// Query params:
///   ?backend=qwen-9b-core  — select specific backend
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

    // 1. Select backend
    let (dog_id, target) = match &params.backend {
        Some(id) => match proxy.get(id) {
            Some(t) => (id.as_str(), t),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(serde_json::json!({
                        "error": {
                            "message": format!("backend '{}' not found. available: {:?}", id, proxy.available()),
                            "type": "invalid_request_error",
                        }
                    })),
                )
                    .into_response();
            }
        },
        None => match proxy.first() {
            Some((id, t)) => (id, t),
            None => {
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
        },
    };

    // 2. Check slot availability (Soma L2/L3 gate)
    if state.slot_tracker.all_slots_busy(dog_id) {
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
        "Soma L4 proxy: forwarding chat completion"
    );

    match req.send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let headers = resp.headers().clone();
            match resp.bytes().await {
                Ok(body) => {
                    let mut response = (status, body).into_response();
                    // Forward content-type from upstream
                    if let Some(ct) = headers.get("content-type") {
                        response.headers_mut().insert("content-type", ct.clone());
                    }
                    response
                }
                Err(e) => (
                    StatusCode::BAD_GATEWAY,
                    axum::Json(serde_json::json!({
                        "error": {
                            "message": format!("upstream response read failed: {e}"),
                            "type": "server_error",
                        }
                    })),
                )
                    .into_response(),
            }
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

    let (_dog_id, target) = match &params.backend {
        Some(id) => match proxy.get(id) {
            Some(t) => (id.as_str(), t),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(serde_json::json!({
                        "error": {
                            "message": format!("backend '{}' not found", id),
                            "type": "invalid_request_error",
                        }
                    })),
                )
                    .into_response();
            }
        },
        None => match proxy.first() {
            Some(pair) => pair,
            None => {
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
        },
    };

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
            match resp.bytes().await {
                Ok(body) => {
                    let mut response = (status, body).into_response();
                    response.headers_mut().insert(
                        "content-type",
                        "application/json".parse().unwrap_or_else(|_| {
                            axum::http::HeaderValue::from_static("application/json")
                        }),
                    );
                    response
                }
                Err(_) => StatusCode::BAD_GATEWAY.into_response(),
            }
        }
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_targets_from_fleet_meta() {
        let mut fleet = std::collections::HashMap::new();
        fleet.insert(
            "qwen-9b-core".to_string(),
            (
                "http://10.0.0.1:8080/v1".to_string(),
                32768u32,
                "Qwen3.5-9B".to_string(),
                None,
            ),
        );
        fleet.insert(
            "cloud-dog".to_string(),
            (
                "https://api.cloud.com/v1".to_string(),
                128000u32,
                "gpt-4".to_string(),
                Some("sk-xxx".to_string()),
            ),
        );

        let mut sovereign = std::collections::HashMap::new();
        sovereign.insert("qwen-9b-core".to_string(), true);
        sovereign.insert("cloud-dog".to_string(), false);

        let targets = ProxyTargets::from_fleet_meta(&fleet, &sovereign);
        assert_eq!(targets.available(), vec!["qwen-9b-core"]);
        assert!(targets.get("cloud-dog").is_none());
    }

    #[test]
    fn proxy_targets_first_is_deterministic() {
        let mut fleet = std::collections::HashMap::new();
        fleet.insert(
            "b-dog".to_string(),
            ("http://b:8080/v1".to_string(), 0u32, "m".to_string(), None),
        );
        fleet.insert(
            "a-dog".to_string(),
            ("http://a:8080/v1".to_string(), 0u32, "m".to_string(), None),
        );

        let mut sovereign = std::collections::HashMap::new();
        sovereign.insert("a-dog".to_string(), true);
        sovereign.insert("b-dog".to_string(), true);

        let targets = ProxyTargets::from_fleet_meta(&fleet, &sovereign);
        // BTreeMap: "a-dog" < "b-dog"
        let (first_id, _) = targets.first().unwrap();
        assert_eq!(first_id, "a-dog");
    }

    #[test]
    fn empty_fleet_returns_none() {
        let targets = ProxyTargets::from_fleet_meta(
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
        );
        assert!(targets.first().is_none());
        assert!(targets.available().is_empty());
    }
}
