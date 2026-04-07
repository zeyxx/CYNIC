// Verify concern: health check, identity check

// WHY: IdentityResult + probe functions consumed by the watch loop (Task 5).
// Until the loop wires these, they appear unused to the compiler.
#![allow(dead_code)]

use reqwest::Client;
use tokio::time::{Duration, timeout};

// ── IdentityResult ────────────────────────────────────────────────────────────

/// Result of a model-identity probe against an inference backend.
#[derive(Debug)]
pub(crate) enum IdentityResult {
    /// At least one loaded model id contains the expected string.
    Match,
    /// Models responded but none matched; carries expected vs actual.
    Mismatch { expected: String, actual: String },
    /// The response arrived but could not be parsed as the expected shape.
    Unknown,
    /// The request timed out or the backend was unreachable.
    Unreachable,
}

// ── check_health ─────────────────────────────────────────────────────────────

/// Probe `health_url` with a GET request.
///
/// Returns `true` only if the backend responds with a 2xx status within
/// `timeout_secs`. Returns `false` on timeout, connection error, or non-2xx.
pub(crate) async fn check_health(client: &Client, health_url: &str, timeout_secs: u64) -> bool {
    let fut = client.get(health_url).send();
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
pub(crate) async fn check_identity(
    client: &Client,
    models_url: &str,
    expected_model: &str,
    timeout_secs: u64,
) -> IdentityResult {
    let fut = client.get(models_url).send();
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

    if ids.iter().any(|id| id.contains(expected_model)) {
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    // ── Mock backend ─────────────────────────────────────────────────────────

    /// Spawn a minimal Axum mock on `127.0.0.1:0` and return its base URL.
    ///
    /// - `GET /health` → 200 OK or 503 depending on `health_ok`
    /// - `GET /v1/models` → `{"data":[{"id":"<model_id>"}]}`
    async fn mock_backend(
        health_ok: bool,
        model_id: &str,
    ) -> (String, tokio::task::JoinHandle<()>) {
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

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (base_url, handle)
    }

    // ── health_check_ok ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn health_check_ok() {
        let (base, _handle) = mock_backend(true, "any-model").await;
        let client = Client::new();
        let url = format!("{base}/health");
        let result = check_health(&client, &url, 5).await;
        assert!(result, "expected true for 200 OK");
    }

    // ── health_check_fail ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn health_check_fail() {
        let (base, _handle) = mock_backend(false, "any-model").await;
        let client = Client::new();
        let url = format!("{base}/health");
        let result = check_health(&client, &url, 5).await;
        assert!(!result, "expected false for 503");
    }

    // ── health_check_unreachable ──────────────────────────────────────────────

    #[tokio::test]
    async fn health_check_unreachable() {
        let client = Client::new();
        // Port 1 is reserved/privileged — connection will be refused immediately.
        let result = check_health(&client, "http://127.0.0.1:1/health", 1).await;
        assert!(!result, "expected false for unreachable host");
    }

    // ── identity_match ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn identity_match() {
        let (base, _handle) = mock_backend(true, "qwen-7b-hf").await;
        let client = Client::new();
        let url = format!("{base}/v1/models");
        let result = check_identity(&client, &url, "qwen-7b", 5).await;
        assert!(
            matches!(result, IdentityResult::Match),
            "expected Match, got {result:?}"
        );
    }

    // ── identity_mismatch ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn identity_mismatch() {
        let (base, _handle) = mock_backend(true, "gemma-4b-core").await;
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

    // ── identity_unreachable ──────────────────────────────────────────────────

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
