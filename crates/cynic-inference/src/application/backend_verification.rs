//! Runtime verification helpers for inference backend endpoints.

use reqwest::Client;
use tokio::time::{Duration, timeout};

// ── IdentityResult ────────────────────────────────────────────────────────────

/// Result of a model-identity probe against an inference backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityResult {
    /// At least one loaded model id contains the expected string.
    Match,
    /// Models responded but none matched; carries expected vs actual.
    Mismatch { expected: String, actual: String },
    /// The response arrived but could not be parsed as the expected shape.
    Unknown,
    /// The request timed out or the backend was unreachable.
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextResult {
    Observed { n_ctx: u32 },
    Unknown,
    Unreachable,
}

// ── check_health ─────────────────────────────────────────────────────────────

/// Probe `health_url` with a GET request.
///
/// Returns `true` only if the backend responds with a 2xx status within
/// `timeout_secs`. Returns `false` on timeout, connection error, or non-2xx.
pub async fn check_health(client: &Client, health_url: &str, timeout_secs: u64) -> bool {
    check_health_with_bearer(client, health_url, timeout_secs, None).await
}

/// Same observation as [`check_health`], but authenticated.
///
/// A backend that requires a bearer token answers an unauthenticated GET with
/// 401, which `status().is_success()` reads as unreachable. Callers observing a
/// secured backend must pass the token or they will report a false outage.
pub async fn check_health_with_bearer(
    client: &Client,
    health_url: &str,
    timeout_secs: u64,
    bearer_token: Option<&str>,
) -> bool {
    let request = match bearer_token {
        Some(token) => client.get(health_url).bearer_auth(token),
        None => client.get(health_url),
    };
    let fut = request.send();
    match timeout(Duration::from_secs(timeout_secs), fut).await {
        Ok(Ok(resp)) => resp.status().is_success(),
        Ok(Err(_)) | Err(_) => false,
    }
}

// ── check_identity ───────────────────────────────────────────────────────────

/// Probe `models_url` and verify that at least one model id contains
/// `expected_model` (substring match).
///
/// Response shape expected:
/// ```json
/// { "data": [ { "id": "model-name" }, … ] }
/// ```
///
/// Returns:
/// - `Match`       — at least one id contains `expected_model`
/// - `Mismatch`    — models responded but none matched
/// - `Unknown`     — response arrived but JSON shape was unexpected
/// - `Unreachable` — request timed out or connection failed
pub async fn check_identity(
    client: &Client,
    models_url: &str,
    expected_model: &str,
    timeout_secs: u64,
) -> IdentityResult {
    check_identity_with_bearer(client, models_url, expected_model, timeout_secs, None).await
}

pub async fn check_identity_with_bearer(
    client: &Client,
    models_url: &str,
    expected_model: &str,
    timeout_secs: u64,
    bearer_token: Option<&str>,
) -> IdentityResult {
    let request = match bearer_token {
        Some(token) => client.get(models_url).bearer_auth(token),
        None => client.get(models_url),
    };
    let fut = request.send();
    let Ok(Ok(resp)) = timeout(Duration::from_secs(timeout_secs), fut).await else {
        return IdentityResult::Unreachable;
    };

    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return IdentityResult::Unknown,
    };

    let Some(data) = json.get("data").and_then(|d| d.as_array()) else {
        return IdentityResult::Unknown;
    };

    let ids: Vec<String> = data
        .iter()
        .filter_map(|entry| entry.get("id")?.as_str().map(String::from))
        .collect();

    if ids
        .iter()
        .any(|id| model_identity_matches(id, expected_model))
    {
        IdentityResult::Match
    } else if ids.is_empty() {
        IdentityResult::Unknown
    } else {
        IdentityResult::Mismatch {
            expected: expected_model.to_string(),
            actual: ids.join(", "),
        }
    }
}

pub fn model_identity_matches(actual_model: &str, expected_model: &str) -> bool {
    if actual_model.is_empty() || expected_model.is_empty() {
        return false;
    }
    let actual_lower = actual_model.to_lowercase();
    let expected_lower = expected_model.to_lowercase();
    actual_lower.contains(&expected_lower) || expected_lower.contains(&actual_lower)
}

pub async fn check_context(client: &Client, props_url: &str, timeout_secs: u64) -> ContextResult {
    check_context_with_bearer(client, props_url, timeout_secs, None).await
}

pub async fn check_context_with_bearer(
    client: &Client,
    props_url: &str,
    timeout_secs: u64,
    bearer_token: Option<&str>,
) -> ContextResult {
    let request = match bearer_token {
        Some(token) => client.get(props_url).bearer_auth(token),
        None => client.get(props_url),
    };
    let fut = request.send();
    let Ok(Ok(resp)) = timeout(Duration::from_secs(timeout_secs), fut).await else {
        return ContextResult::Unreachable;
    };

    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return ContextResult::Unknown,
    };

    json.pointer("/default_generation_settings/n_ctx")
        .and_then(|value| value.as_u64())
        .map(|n_ctx| ContextResult::Observed {
            n_ctx: n_ctx as u32,
        })
        .unwrap_or(ContextResult::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    /// Spawn a minimal Axum mock on `127.0.0.1:0` and return its base URL.
    ///
    /// - `GET /health` → 200 OK or 503 depending on `health_ok`
    /// - `GET /v1/models` → `{"data":[{"id":"<model_id>"}]}`
    async fn mock_backend(
        health_ok: bool,
        model_id: &str,
    ) -> Option<(String, tokio::task::JoinHandle<()>)> {
        let model_id = model_id.to_string();

        let health_handler: axum::routing::MethodRouter = if health_ok {
            get(|| async { axum::http::StatusCode::OK })
        } else {
            get(|| async { axum::http::StatusCode::SERVICE_UNAVAILABLE })
        };

        let model_id_clone = model_id.clone();
        let models_handler = get(move || {
            let id = model_id_clone.clone();
            async move {
                let body = format!(r#"{{"data":[{{"id":"{id}"}}]}}"#);
                (
                    axum::http::StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    body,
                )
            }
        });

        let app = Router::new()
            .route("/health", health_handler)
            .route("/v1/models", models_handler);

        let listener = match TcpListener::bind("127.0.0.1:0").await {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("skip mocked backend test: cannot bind loopback listener: {err}");
                return None;
            }
        };
        let addr: SocketAddr = match listener.local_addr() {
            Ok(addr) => addr,
            Err(err) => {
                eprintln!("skip mocked backend test: cannot query listener address: {err}");
                return None;
            }
        };
        let base_url = format!("http://{addr}");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Some((base_url, handle))
    }

    /// A backend that demands `Authorization: Bearer <token>` on /health and
    /// answers 401 otherwise. Proves the bearer is actually transmitted.
    async fn mock_secured_backend(token: &str) -> Option<(String, tokio::task::JoinHandle<()>)> {
        let expected = format!("Bearer {token}");
        let app = axum::Router::new().route(
            "/health",
            get(move |headers: axum::http::HeaderMap| {
                let expected = expected.clone();
                async move {
                    let presented = headers
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or_default()
                        .to_string();
                    if presented == expected {
                        axum::http::StatusCode::OK
                    } else {
                        axum::http::StatusCode::UNAUTHORIZED
                    }
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.ok()?;
        let addr = listener.local_addr().ok()?;
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Some((format!("http://{addr}"), handle))
    }

    #[tokio::test]
    async fn health_check_with_bearer_is_reachable_on_a_secured_backend() {
        let Some((base, _handle)) = mock_secured_backend("secret-token").await else {
            return;
        };
        let client = Client::new();
        let url = format!("{base}/health");

        assert!(
            check_health_with_bearer(&client, &url, 5, Some("secret-token")).await,
            "a secured backend must be observed as reachable when the bearer is sent"
        );
        // Regression guard: this is exactly what the kernel fleet probe used to
        // hit before it delegated health observation to the node facade.
        assert!(
            !check_health_with_bearer(&client, &url, 5, None).await,
            "without the bearer the backend answers 401, which is not reachable"
        );
        assert!(
            !check_health(&client, &url, 5).await,
            "the unauthenticated helper must stay unauthenticated"
        );
    }

    #[tokio::test]
    async fn health_check_ok() {
        let Some((base, _handle)) = mock_backend(true, "any-model").await else {
            return;
        };
        let client = Client::new();
        let url = format!("{base}/health");
        let result = check_health(&client, &url, 5).await;
        assert!(result, "expected true for 200 OK");
    }

    #[tokio::test]
    async fn health_check_fail() {
        let Some((base, _handle)) = mock_backend(false, "any-model").await else {
            return;
        };
        let client = Client::new();
        let url = format!("{base}/health");
        let result = check_health(&client, &url, 5).await;
        assert!(!result, "expected false for 503");
    }

    #[tokio::test]
    async fn health_check_unreachable() {
        let client = Client::new();
        let result = check_health(&client, "http://127.0.0.1:1/health", 1).await;
        assert!(!result, "expected false for unreachable host");
    }

    #[test]
    fn model_identity_matching_is_case_insensitive_and_bidirectional() {
        assert!(model_identity_matches(
            "Qwen/Qwen2.5-7B-Instruct",
            "qwen2.5-7b"
        ));
        assert!(model_identity_matches(
            "qwen2.5-7b",
            "Qwen/Qwen2.5-7B-Instruct"
        ));
        assert!(!model_identity_matches("gemma-3-4b-it", "Mistral-7B-v0.3"));
        assert!(!model_identity_matches("", "Mistral-7B-v0.3"));
    }

    #[test]
    fn context_result_shape_is_stable() {
        assert_eq!(
            ContextResult::Observed { n_ctx: 32768 },
            ContextResult::Observed { n_ctx: 32768 }
        );
        assert_ne!(
            ContextResult::Observed { n_ctx: 2048 },
            ContextResult::Unknown
        );
    }

    #[tokio::test]
    async fn identity_match() {
        let Some((base, _handle)) = mock_backend(true, "qwen-7b-hf").await else {
            return;
        };
        let client = Client::new();
        let url = format!("{base}/v1/models");
        let result = check_identity(&client, &url, "qwen-7b", 5).await;
        assert!(
            matches!(result, IdentityResult::Match),
            "expected Match, got {result:?}"
        );
    }

    #[tokio::test]
    async fn identity_mismatch() {
        let Some((base, _handle)) = mock_backend(true, "gemma-4b-core").await else {
            return;
        };
        let client = Client::new();
        let url = format!("{base}/v1/models");
        let result = check_identity(&client, &url, "qwen-7b", 5).await;
        match result {
            IdentityResult::Mismatch { expected, actual } => {
                assert_eq!(expected, "qwen-7b");
                assert!(actual.contains("gemma-4b-core"), "actual was: {actual}");
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn identity_unreachable() {
        let client = Client::new();
        let result = check_identity(&client, "http://127.0.0.1:1/v1/models", "qwen-7b", 1).await;
        assert!(
            matches!(result, IdentityResult::Unreachable),
            "expected Unreachable, got {result:?}"
        );
    }
}
