//! REST API middleware — auth, rate limiting, audit logging.

mod agent_session;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use http_body_util::BodyExt;
use std::sync::Arc;

use super::auth::auth_sessions;
use super::types::{
    AppState, AuthenticatedPrincipal, AuthenticationMechanism, ErrorResponse,
    authorize_role_request,
};
use crate::infra::crypto;

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

/// Unified authentication middleware.
/// Supports:
/// 1. Ed25519 Cryptographic Signatures (X-Cynic-* headers)
/// 2. Compatibility bearer tokens while workload identity migration is active.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().as_str().to_string();
    if path == "/live"
        || path == "/identity"
        || path == "/ready"
        || path == "/openapi.json"
        || path == "/api/topology"
        || path == "/api/layer-alignment"
        || path == "/auth/input"
        || path == "/auth/verify"
        || path == "/state-history"
    {
        return next.run(request).await;
    }

    if path == "/auth/agent/root-launch"
        && request
            .extensions()
            .get::<super::local_peer::LocalPeerCredentials>()
            .is_some()
    {
        return next.run(request).await;
    }

    if path == "/node-enrollment" || path == "/node-enrollment/renew" {
        return next.run(request).await;
    }

    let (request, agent_session_authenticated) =
        match agent_session::authenticate(&state, request, &method, &path).await {
            Ok(outcome) => outcome,
            Err(response) => return response,
        };
    if agent_session_authenticated {
        return next.run(request).await;
    }

    // 1. Wallet session (preferred human path)
    if let Some(session_id) = request
        .headers()
        .get("X-Cynic-Session")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        let now = chrono::Utc::now().timestamp().max(0) as u64;
        let session = auth_sessions()
            .read()
            .ok() // WHY: ignore lock poisoning
            .and_then(|sessions| sessions.get(&session_id).cloned());

        if let Some(session) = session.filter(|s| s.expires_at >= now) {
            let mut request = request;
            request.extensions_mut().insert(session.role);
            request.extensions_mut().insert(AuthenticatedPrincipal {
                subject: None,
                role: session.role,
                mechanism: AuthenticationMechanism::WalletSession,
            });
            return next.run(request).await;
        }

        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid or expired wallet session".into(),
            }),
        )
            .into_response();
    }

    // 1. Try Cryptographic Signature first (Zero-Trust path)
    let signature = request
        .headers()
        .get("X-Cynic-Signature")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let pubkey = request
        .headers()
        .get("X-Cynic-Public-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let timestamp = request
        .headers()
        .get("X-Cynic-Timestamp")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let (Some(sig), Some(pk), Some(ts)) = (signature, pubkey, timestamp) {
        // We need the body to verify the signature
        let (parts, body) = request.into_parts();
        let body_bytes = match body.collect().await {
            Ok(c) => c.to_bytes(),
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
            }
        };

        let method = parts.method.as_str();
        let path = parts.uri.path();
        let payload = crypto::build_rest_payload(method, path, &body_bytes);

        match crypto::verify_signature(&pk, &sig, &ts, &payload) {
            Ok(_) => {
                let Some(principal) = state.role_keys.resolve_signer(&pk) else {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorResponse {
                            error: "Valid signature belongs to an unregistered workload identity"
                                .into(),
                        }),
                    )
                        .into_response();
                };
                let role = principal.role;
                if !authorize_role_request(role, &method, &path) {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(ErrorResponse {
                            error: format!("principal role is not authorized for {method} {path}"),
                        }),
                    )
                        .into_response();
                }
                let mut request = Request::from_parts(parts, Body::from(body_bytes));
                request.extensions_mut().insert(role);
                request.extensions_mut().insert(principal);
                return next.run(request).await;
            }
            Err(e) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: format!("Signature verification failed: {e}"),
                    }),
                )
                    .into_response();
            }
        }
    }

    // 2. Fallback to Bearer Token (Backward compatibility)
    if !state.role_keys.has_any_authentication_authority() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "No workload identity or compatibility bearer authority configured".into(),
            }),
        )
            .into_response();
    }

    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let Some(token) = token else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Missing Bearer token or Signature headers".into(),
            }),
        )
            .into_response();
    };

    let Some(principal) = state.role_keys.resolve(&token) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid Bearer token".into(),
            }),
        )
            .into_response();
    };

    if !authorize_role_request(principal.role, &method, &path) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: format!("principal role is not authorized for {method} {path}"),
            }),
        )
            .into_response();
    }

    let mut request = request;
    request.extensions_mut().insert(principal.role);
    request.extensions_mut().insert(principal);
    next.run(request).await
}

/// Rate limiter — per-IP token bucket. Exempt: health probes, observe, coord.
/// /judge has stricter limit (inference costs money).
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    // Exempt: health probes (monitoring), observe (fire-and-forget organ data),
    // coord (agent coordination). These are high-frequency, low-cost endpoints
    // that organs and hooks call continuously. Rate-limiting them blocks organ
    // lifecycle (Hermes X 429 incident 2026-04-26).
    if path == "/health"
        || path == "/identity"
        || path == "/live"
        || path == "/ready"
        || path == "/observe"
        || path.starts_with("/coord/")
    {
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
    if path == "/health" || path == "/identity" || path == "/live" || path == "/ready" {
        return next.run(request).await;
    }

    let method = request.method().to_string();
    let role = request
        .extensions()
        .get::<super::types::Role>()
        .map(|r| r.to_string())
        .unwrap_or_else(|| "anon".to_string());
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
                "role": role,
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
