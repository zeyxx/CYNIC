// Announce concern: register, heartbeat, deregister

// WHY: RegisterResponse, HeartbeatResult, RegisterError, and all three
// pub(crate) functions are consumed by the watch loop (Task 5).
// Until that loop wires them, they appear unused to the compiler.
#![allow(dead_code)]

use reqwest::Client;
use serde_json::Value;
use std::fmt;
use tokio::time::{Duration, timeout};

// ── RegisterResponse ──────────────────────────────────────────────────────────

/// Successful response from POST /dogs/register.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct RegisterResponse {
    pub(crate) dog_id: String,
    pub(crate) roster_size: usize,
}

// ── HeartbeatResult ───────────────────────────────────────────────────────────

/// Result of a single heartbeat POST to the kernel.
#[derive(Debug)]
pub(crate) enum HeartbeatResult {
    /// Kernel acknowledged the heartbeat (200).
    Alive,
    /// Kernel no longer knows this dog — TTL expired (404).
    Expired,
    /// Network error, timeout, or unexpected status.
    Error(String),
}

// ── RegisterError ─────────────────────────────────────────────────────────────

/// Failure modes for POST /dogs/register.
#[derive(Debug)]
pub(crate) enum RegisterError {
    /// A dog with the same name/id is already registered (409).
    Collision,
    /// The registration payload failed kernel validation (422).
    CalibrationFail(String),
    /// Network error, timeout, or unexpected status.
    Transient(String),
}

impl fmt::Display for RegisterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegisterError::Collision => write!(f, "registration collision: dog already registered"),
            RegisterError::CalibrationFail(body) => {
                write!(f, "calibration failed: {body}")
            }
            RegisterError::Transient(msg) => write!(f, "transient registration error: {msg}"),
        }
    }
}

impl std::error::Error for RegisterError {}

// ── try_register ──────────────────────────────────────────────────────────────

/// Attempt to register this node as a Dog with the kernel.
///
/// - 200 → Ok(RegisterResponse)
/// - 409 → Err(RegisterError::Collision)
/// - 422 → Err(RegisterError::CalibrationFail(body))
/// - other → Err(RegisterError::Transient)
pub(crate) async fn try_register(
    client: &Client,
    kernel_url: &str,
    api_key: &str,
    payload: &Value,
) -> Result<RegisterResponse, RegisterError> {
    let url = format!("{kernel_url}/dogs/register");

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(payload)
        .send()
        .await
        .map_err(|e| RegisterError::Transient(e.to_string()))?;

    match resp.status().as_u16() {
        200 => {
            let reg: RegisterResponse = resp
                .json()
                .await
                .map_err(|e| RegisterError::Transient(format!("failed to parse response: {e}")))?;
            Ok(reg)
        }
        409 => Err(RegisterError::Collision),
        422 => {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| String::from("<unreadable body>"));
            Err(RegisterError::CalibrationFail(body))
        }
        other => {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| String::from("<unreadable body>"));
            Err(RegisterError::Transient(format!(
                "unexpected status {other}: {body}"
            )))
        }
    }
}

// ── send_heartbeat ────────────────────────────────────────────────────────────

/// POST a heartbeat for `dog_id` to the kernel.
///
/// - 200 → HeartbeatResult::Alive
/// - 404 → HeartbeatResult::Expired
/// - other / error → HeartbeatResult::Error
pub(crate) async fn send_heartbeat(
    client: &Client,
    kernel_url: &str,
    api_key: &str,
    dog_id: &str,
) -> HeartbeatResult {
    let url = format!("{kernel_url}/dogs/{dog_id}/heartbeat");

    let result = client.post(&url).bearer_auth(api_key).send().await;

    match result {
        Err(e) => HeartbeatResult::Error(e.to_string()),
        Ok(resp) => match resp.status().as_u16() {
            200 => HeartbeatResult::Alive,
            404 => HeartbeatResult::Expired,
            other => HeartbeatResult::Error(format!("unexpected status {other}")),
        },
    }
}

// ── try_deregister ────────────────────────────────────────────────────────────

/// Best-effort DELETE of `dog_id` from the kernel roster.
///
/// Uses a 5-second timeout. Logs result with tracing. Never fails the caller.
pub(crate) async fn try_deregister(client: &Client, kernel_url: &str, api_key: &str, dog_id: &str) {
    let url = format!("{kernel_url}/dogs/{dog_id}");

    let fut = client.delete(&url).bearer_auth(api_key).send();

    match timeout(Duration::from_secs(5), fut).await {
        Ok(Ok(resp)) if resp.status().is_success() => {
            tracing::info!(dog_id, "deregistered from kernel");
        }
        Ok(Ok(resp)) => {
            tracing::warn!(
                dog_id,
                status = resp.status().as_u16(),
                "deregister returned non-success status"
            );
        }
        Ok(Err(e)) => {
            tracing::warn!(dog_id, error = %e, "deregister request failed");
        }
        Err(_) => {
            tracing::warn!(dog_id, "deregister timed out after 5 seconds");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        extract::Path,
        routing::{delete, post},
    };
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    // ── Mock kernel ───────────────────────────────────────────────────────────

    /// Spawn a minimal Axum mock on `127.0.0.1:0` and return its base URL.
    ///
    /// Routes:
    /// - POST /dogs/register      → 200 {dog_id, calibration, roster_size}
    /// - POST /dogs/{id}/heartbeat → 200 {dog_id, status, ttl_remaining_secs}
    /// - DELETE /dogs/{id}         → 200 {dog_id, status, roster_size}
    async fn mock_kernel() -> (String, tokio::task::JoinHandle<()>) {
        let register_handler = post(|_body: axum::body::Bytes| async {
            let body = r#"{"dog_id":"test-dog-1","calibration":"passed","roster_size":1}"#;
            (
                axum::http::StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                body,
            )
        });

        let heartbeat_handler = post(|Path(id): Path<String>| async move {
            let body = format!(r#"{{"dog_id":"{id}","status":"alive","ttl_remaining_secs":120}}"#);
            (
                axum::http::StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                body,
            )
        });

        let deregister_handler = delete(|Path(id): Path<String>| async move {
            let body = format!(r#"{{"dog_id":"{id}","status":"deregistered","roster_size":0}}"#);
            (
                axum::http::StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                body,
            )
        });

        let app = Router::new()
            .route("/dogs/register", register_handler)
            .route("/dogs/{id}/heartbeat", heartbeat_handler)
            .route("/dogs/{id}", deregister_handler);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (base_url, handle)
    }

    // ── register_success ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn register_success() {
        let (base, _handle) = mock_kernel().await;
        let client = Client::new();
        let payload = serde_json::json!({
            "name": "test-dog-1",
            "model": "qwen-7b",
            "endpoint": "http://127.0.0.1:8080"
        });

        let result = try_register(&client, &base, "test-key", &payload).await;
        let reg = result.unwrap();
        assert_eq!(reg.dog_id, "test-dog-1");
        assert_eq!(reg.roster_size, 1);
    }

    // ── heartbeat_alive ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn heartbeat_alive() {
        let (base, _handle) = mock_kernel().await;
        let client = Client::new();

        let result = send_heartbeat(&client, &base, "test-key", "test-dog-1").await;
        assert!(
            matches!(result, HeartbeatResult::Alive),
            "expected Alive, got {result:?}"
        );
    }

    // ── heartbeat_unreachable ─────────────────────────────────────────────────

    #[tokio::test]
    async fn heartbeat_unreachable() {
        let client = Client::new();
        // Port 1 is reserved/privileged — connection will be refused immediately.
        let result = send_heartbeat(&client, "http://127.0.0.1:1", "test-key", "test-dog-1").await;
        assert!(
            matches!(result, HeartbeatResult::Error(_)),
            "expected Error, got {result:?}"
        );
    }

    // ── deregister_success ────────────────────────────────────────────────────

    #[tokio::test]
    async fn deregister_success() {
        let (base, _handle) = mock_kernel().await;
        let client = Client::new();
        // Best-effort: must not panic regardless of outcome.
        try_deregister(&client, &base, "test-key", "test-dog-1").await;
    }
}
