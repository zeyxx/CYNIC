//! REST API — JSON interface for external clients (React, curl, etc.)

pub mod coord;
pub mod data;
pub mod events;
pub mod health;
pub mod judge;
pub mod middleware;
pub mod observe;
pub mod response;
pub mod types;

pub use types::*;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware as axum_mw,
    routing::{get, post},
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use self::coord::{
    coord_claim_batch_handler, coord_claim_handler, coord_register_handler, coord_release_handler,
};
use self::data::{
    audit_handler, compliance_handler, compliance_trend_handler, create_crystal_handler,
    crystal_handler, crystals_handler, delete_crystal_handler, observations_handler,
    observe_crystal_handler, sessions_handler, usage_handler,
};
use self::health::{
    agents_handler, dogs_handler, health_handler, liveness_handler, metrics_handler,
    readiness_handler,
};
use self::judge::{get_verdict_handler, judge_handler, list_verdicts_handler};
use self::middleware::{audit_middleware, auth_middleware, rate_limit_middleware};
use self::observe::observe_handler;

// ── ROUTER ─────────────────────────────────────────────────

pub fn router(state: Arc<AppState>) -> Router {
    // CORS: restrict to known origins. CYNIC_CORS_ORIGINS env var for additional.
    // Default: localhost dev servers only.
    let mut origins: Vec<axum::http::HeaderValue> = vec![
        axum::http::HeaderValue::from_static("http://localhost:5173"),
        axum::http::HeaderValue::from_static("http://localhost:5000"),
        axum::http::HeaderValue::from_static("http://localhost:3000"),
    ];
    if let Ok(extra) = std::env::var("CYNIC_CORS_ORIGINS") {
        for o in extra.split(',') {
            if let Ok(v) = o.trim().parse() {
                origins.push(v);
            }
        }
    }
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ]);

    if state.api_key.is_some() {
        tracing::info!("REST Bearer auth ENABLED");
    } else {
        tracing::warn!("No CYNIC_API_KEY set — REST API is OPEN");
    }

    Router::new()
        .route("/health", get(health_handler))
        .route("/live", get(liveness_handler))
        .route("/ready", get(readiness_handler))
        .route("/metrics", get(metrics_handler))
        .route("/events", get(events::events_handler))
        .route("/judge", post(judge_handler))
        .route("/dogs", get(dogs_handler))
        .route("/crystals", get(crystals_handler))
        .route("/crystal", post(create_crystal_handler))
        .route(
            "/crystal/{id}",
            get(crystal_handler).delete(delete_crystal_handler),
        )
        .route("/crystal/{id}/observe", post(observe_crystal_handler))
        .route("/usage", get(usage_handler))
        .route("/verdict/{id}", get(get_verdict_handler))
        .route("/verdicts", get(list_verdicts_handler))
        .route("/agents", get(agents_handler))
        .route("/observations", get(observations_handler))
        .route("/sessions", get(sessions_handler))
        .route("/session/{agent_id}/compliance", get(compliance_handler))
        .route("/compliance", get(compliance_trend_handler))
        .route("/audit", get(audit_handler))
        .route("/observe", post(observe_handler))
        .route("/coord/register", post(coord_register_handler))
        .route("/coord/claim", post(coord_claim_handler))
        .route("/coord/claim-batch", post(coord_claim_batch_handler))
        .route("/coord/release", post(coord_release_handler))
        .layer(axum_mw::from_fn_with_state(state.clone(), audit_middleware))
        .layer(axum_mw::from_fn_with_state(state.clone(), auth_middleware))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .fallback_service(ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(64 * 1024)) // 64 KB — no multi-MB payloads
        .layer(cors)
        // Request timeout: outer safety net. Must exceed Judge wall-clock (max_dog_timeout + 5s).
        // Sovereign CPU Dogs can take 90s. Wall-clock = 95s. This is 120s for margin.
        // The Judge's own per-dog + wall-clock timeouts are the real enforcement.
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(
            axum::http::StatusCode::GATEWAY_TIMEOUT,
            std::time::Duration::from_secs(120),
        ))
        .with_state(state)
}
