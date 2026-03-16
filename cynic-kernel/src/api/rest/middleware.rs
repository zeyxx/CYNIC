//! REST API middleware — auth, rate limiting, audit logging.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{Json, IntoResponse, Response},
};
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};

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
            Some(t) if t == *key => {},
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

/// Rate limiter — rejects excess requests with 429. /health exempt.
/// /judge has a stricter limit (costs inference tokens).
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if path == "/health" {
        return next.run(request).await;
    }
    // /judge has its own stricter rate limit
    if path == "/judge" && !state.judge_limiter.check() {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse { error: "Judge rate limit exceeded (inference is expensive)".into() }),
        ).into_response();
    }
    if !state.rate_limiter.check() {
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
