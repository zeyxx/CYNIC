//! REST API — JSON interface for external clients (React, curl, etc.)
//! Runs alongside gRPC on a separate port.

pub mod types;
pub mod middleware;
pub mod judge;
pub mod response;
pub mod data;
pub mod health;

pub use types::*;
pub use response::compute_temporal_from_dogs;

use axum::{
    extract::DefaultBodyLimit,
    middleware as axum_mw,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use self::data::{crystals_handler, crystal_handler, usage_handler};
use self::health::{health_handler, dogs_handler, temporal_handler, agents_handler};
use self::judge::{judge_handler, get_verdict_handler, list_verdicts_handler};
use self::middleware::{auth_middleware, rate_limit_middleware, audit_middleware};

// ── ROUTER ─────────────────────────────────────────────────

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

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
        .layer(axum_mw::from_fn_with_state(state.clone(), audit_middleware))
        .layer(axum_mw::from_fn_with_state(state.clone(), auth_middleware))
        .layer(axum_mw::from_fn_with_state(state.clone(), rate_limit_middleware))
        .fallback_service(ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(64 * 1024)) // 64 KB — no multi-MB payloads
        .layer(cors)
        .with_state(state)
}
