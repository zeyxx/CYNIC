//! REST API middleware — auth, rate limiting, audit logging.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};

/// Constant-time comparison to prevent timing attacks on API key.
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Bearer token authentication. Skipped for /health. Skipped if no CYNIC_API_KEY.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    // /health, /live, /ready are public — no auth required.
    // /metrics and /events require auth (KC3: leak Dog roster + operational state).
    let path = request.uri().path();
    if path == "/health" || path == "/live" || path == "/ready" {
        return next.run(request).await;
    }

    if let Some(ref key) = state.api_key {
        let token = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        match token {
            Some(t) if constant_time_eq(t.as_bytes(), key.as_bytes()) => {}
            _ => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "Invalid or missing Bearer token".into(),
                    }),
                )
                    .into_response();
            }
        }
    }
    next.run(request).await
}

/// Rate limiter — per-IP token bucket. /health, /live, /ready exempt. /judge has stricter limit.
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if path == "/health" || path == "/live" || path == "/ready" {
        return next.run(request).await;
    }

    // F2: Use real peer address, not spoofable X-Forwarded-For.
    // CYNIC runs on Tailscale (no reverse proxy) — X-Forwarded-For is untrusted.
    // axum::serve injects ConnectInfo<SocketAddr> automatically.
    let ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    // Global rate limit applies to ALL endpoints (including /judge)
    if !state.rate_limiter.check(ip).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "Rate limit exceeded".into(),
            }),
        )
            .into_response();
    }
    // /judge and /judge/async have an additional stricter per-IP limit (inference costs money)
    if (path == "/judge" || path == "/judge/async") && !state.judge_limiter.check(ip).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse {
                error: "Judge rate limit exceeded (inference is expensive)".into(),
            }),
        )
            .into_response();
    }
    next.run(request).await
}

/// Audit log — logs every request (except probes) to SurrealDB. Best-effort, non-blocking.
pub async fn audit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if path == "/health" || path == "/live" || path == "/ready" {
        return next.run(request).await;
    }

    let method = request.method().to_string();
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // Best-effort async audit — bounded (Semaphore) + tracked (TaskTracker) + timed out (5s)
    {
        let semaphore = Arc::clone(&state.bg_semaphore);
        if let Ok(permit) = semaphore.try_acquire_owned() {
            let coord = Arc::clone(&state.coord);
            let details = serde_json::json!({
                "method": method,
                "path": path,
                "status": status,
                "latency_ms": elapsed_ms,
            })
            .to_string();
            state.bg_tasks.spawn(async move {
                let _permit = permit; // held until task completes
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    coord.store_audit("rest_request", "rest", &details),
                )
                .await
                {
                    Ok(Err(e)) => tracing::warn!(error = %e, "audit store failed"),
                    Err(_) => tracing::warn!("audit store timed out (5s)"),
                    _ => {}
                }
            });
        }
        // If semaphore full: silently drop audit (non-critical, prevents task accumulation)
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── constant_time_eq ─────────────────────────────────

    #[test]
    fn constant_time_eq_identical() {
        assert!(constant_time_eq(b"secret-key-123", b"secret-key-123"));
    }

    #[test]
    fn constant_time_eq_different_same_len() {
        assert!(!constant_time_eq(b"secret-key-123", b"secret-key-124"));
    }

    #[test]
    fn constant_time_eq_different_len() {
        assert!(!constant_time_eq(b"short", b"longer-string"));
    }

    #[test]
    fn constant_time_eq_empty() {
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn constant_time_eq_one_empty() {
        assert!(!constant_time_eq(b"", b"x"));
        assert!(!constant_time_eq(b"x", b""));
    }

    #[test]
    fn constant_time_eq_single_bit_diff() {
        // Differ by exactly 1 bit — must still reject
        assert!(!constant_time_eq(b"\x00", b"\x01"));
        assert!(!constant_time_eq(b"A", b"a")); // 0x41 vs 0x61
    }
}
