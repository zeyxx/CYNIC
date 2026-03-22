//! REST API — JSON interface for external clients (React, curl, etc.)
//! Runs alongside gRPC on a separate port.

pub mod types;
pub mod middleware;
pub mod judge;
pub mod response;
pub mod data;
pub mod health;
pub mod observe;
pub mod coord;

pub use types::*;
pub use response::compute_temporal_from_dogs;

use axum::{
    extract::DefaultBodyLimit,
    middleware as axum_mw,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use self::coord::{coord_register_handler, coord_claim_handler, coord_claim_batch_handler, coord_release_handler};
use self::data::{crystals_handler, crystal_handler, usage_handler};
use self::health::{health_handler, dogs_handler, temporal_handler, agents_handler};
use self::judge::{judge_handler, get_verdict_handler, list_verdicts_handler};
use self::middleware::{auth_middleware, rate_limit_middleware, audit_middleware};
use self::observe::observe_handler;

// ── ROUTER ─────────────────────────────────────────────────

pub fn router(state: Arc<AppState>) -> Router {
    // CORS: restrict to known origins. CYNIC_CORS_ORIGINS env var for additional.
    // Default: localhost dev servers only.
    let mut origins: Vec<axum::http::HeaderValue> = vec![
        "http://localhost:5173".parse().unwrap(),
        "http://localhost:5000".parse().unwrap(),
        "http://localhost:3000".parse().unwrap(),
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
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    if state.api_key.is_some() {
        eprintln!("[Ring 3 / REST] Bearer auth ENABLED");
    } else {
        eprintln!("[Ring 3 / REST] WARNING: No CYNIC_API_KEY set — API is OPEN");
    }

    Router::new()
        .route("/health", get(health_handler))
        .route("/judge", post(judge_handler))
        .route("/dogs", get(dogs_handler))
        .route("/crystals", get(crystals_handler))
        .route("/crystal/{id}", get(crystal_handler))
        .route("/usage", get(usage_handler))
        .route("/temporal", get(temporal_handler))
        .route("/verdict/{id}", get(get_verdict_handler))
        .route("/verdicts", get(list_verdicts_handler))
        .route("/agents", get(agents_handler))
        .route("/observe", post(observe_handler))
        .route("/coord/register", post(coord_register_handler))
        .route("/coord/claim", post(coord_claim_handler))
        .route("/coord/claim-batch", post(coord_claim_batch_handler))
        .route("/coord/release", post(coord_release_handler))
        .layer(axum_mw::from_fn_with_state(state.clone(), audit_middleware))
        .layer(axum_mw::from_fn_with_state(state.clone(), auth_middleware))
        .layer(axum_mw::from_fn_with_state(state.clone(), rate_limit_middleware))
        .fallback_service(ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(64 * 1024)) // 64 KB — no multi-MB payloads
        .layer(cors)
        // Request timeout: outer safety net. Must exceed Judge wall-clock (max_dog_timeout + 5s).
        // Sovereign CPU Dogs can take 90s. Wall-clock = 95s. This is 120s for margin.
        // The Judge's own per-dog + wall-clock timeouts are the real enforcement.
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(axum::http::StatusCode::GATEWAY_TIMEOUT, std::time::Duration::from_secs(120)))
        .with_state(state)
}
