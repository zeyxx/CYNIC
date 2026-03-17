//! REST API middleware — auth, rate limiting, audit logging.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{Json, IntoResponse, Response},
};
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};

/// Constant-time comparison to prevent timing attacks on API key.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Bearer token authentication. Skipped for /health. Skipped if no CYNIC_API_KEY.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    // /health is public — no auth required
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }

    if let Some(ref key) = state.api_key {
        let token = request.headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        match token {
            Some(t) if constant_time_eq(t.as_bytes(), key.as_bytes()) => {},
            _ => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse { error: "Invalid or missing Bearer token".into() }),
                ).into_response();
            }
        }
    }
    next.run(request).await
}

/// Rate limiter — per-IP token bucket. /health exempt. /judge has stricter limit.
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if path == "/health" {
        return next.run(request).await;
    }

    // Extract client IP from X-Forwarded-For or peer address
    let ip = request.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    // /judge has its own stricter per-IP rate limit
    if path == "/judge" {
        if !state.judge_limiter.check(ip) {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse { error: "Judge rate limit exceeded (inference is expensive)".into() }),
            ).into_response();
        }
        // /judge only checks judge_limiter, not global — no double counting
        return next.run(request).await;
    }
    if !state.rate_limiter.check(ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse { error: "Rate limit exceeded".into() }),
        ).into_response();
    }
    next.run(request).await
}

/// Audit log — logs every request (except /health) to SurrealDB. Best-effort, non-blocking.
pub async fn audit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if path == "/health" {
        return next.run(request).await;
    }

    let method = request.method().to_string();
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // Best-effort async audit — don't block the response
    {
        let coord = Arc::clone(&state.coord);
        let details = serde_json::json!({
            "method": method,
            "path": path,
            "status": status,
            "latency_ms": elapsed_ms,
        });
        tokio::spawn(async move {
            let _ = coord.store_audit("rest_request", "rest", &details).await;
        });
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
