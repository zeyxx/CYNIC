//! REST API — JSON interface for external clients (React, curl, etc.)

pub mod agent_tasks;
pub mod coord;
pub mod data;
pub mod dispatch;
pub mod dogs;
pub mod event;
pub mod events;
pub mod health;
pub mod inference_proxy;
pub mod inference_router;
pub mod judge;
pub mod judge_job;
pub mod mail;
pub mod middleware;
pub mod mint_permit;
pub mod observe;
pub mod phone_numbers;
pub mod response;
pub mod soma;
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
    coord_claim_batch_handler, coord_claim_handler, coord_heartbeat_handler,
    coord_register_handler, coord_release_handler, coord_scope_handler, coord_who_handler,
};
use self::data::{
    audit_handler, compliance_handler, compliance_trend_handler, create_crystal_handler,
    crystal_handler, crystals_handler, delete_crystal_handler, observations_handler,
    observe_crystal_hypha_handler, sessions_handler, shatter_crystal_handler, usage_handler,
};
use self::dogs::{deregister_handler, dogs_handler, heartbeat_handler, register_dog_handler};
use self::event::{event_handler, fleet_stats_handler};
use self::health::{
    agents_handler, health_handler, liveness_handler, metrics_handler, readiness_handler,
    state_history_handler,
};
use self::inference_proxy::{proxy_chat_completions, proxy_models};
use self::inference_router::{
    inference_candidates_handler, inference_remediate_handler, inference_route_handler,
    inference_slots_handler, inference_start_handler, list_models_handler, remediate_handler,
};
use self::judge::{get_verdict_handler, judge_handler, list_verdicts_handler};
use self::judge_job::{judge_async_handler, judge_status_handler};
use self::mail::{
    fetch_message, health as mail_health, inbox, mark_read, search, send, sync, unread,
};
use self::middleware::{audit_middleware, auth_middleware, rate_limit_middleware};
use self::mint_permit::mint_permit_handler;
use self::observe::observe_handler;
use self::soma::soma_request_handler;
use crate::api::websocket::ws_handler;

// ── ROUTER ─────────────────────────────────────────────────

pub fn router(state: Arc<AppState>) -> Router {
    // CORS: restrict to known origins. CYNIC_CORS_ORIGINS env var for additional.
    // Default: localhost dev servers only.
    let mut origins: Vec<axum::http::HeaderValue> = vec![
        axum::http::HeaderValue::from_static("http://localhost:5173"),
        axum::http::HeaderValue::from_static("http://localhost:5002"),
        axum::http::HeaderValue::from_static("http://localhost:5000"),
        axum::http::HeaderValue::from_static("http://localhost:3000"),
    ];
    if let Ok(extra) = std::env::var("CYNIC_CORS_ORIGINS") {
        for o in extra.split(',') {
            if let Ok(v) = o.trim().parse() {
                origins.push(v);
            }
        }
    } else {
        tracing::warn!(
            "CORS: localhost-only origins active — set CYNIC_CORS_ORIGINS for remote frontends"
        );
    }
    let cors = CorsLayer::new()
        // Allow Any origin so local browser extensions (chrome-extension://...) can connect
        .allow_origin(tower_http::cors::Any)
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
        tracing::warn!("REST Bearer auth DISABLED via explicit CYNIC_ALLOW_OPEN_API=1");
    }

    Router::new()
        .route("/node/ws", axum::routing::get(ws_handler))
        .route("/health", get(health_handler))
        .route("/live", get(liveness_handler))
        .route("/ready", get(readiness_handler))
        .route("/metrics", get(metrics_handler))
        .route("/events", get(events::events_handler))
        .route("/judge", post(judge_handler))
        .route("/judge/async", post(judge_async_handler))
        .route("/judge/status/{id}", get(judge_status_handler))
        .route("/mint-permit", post(mint_permit_handler))
        .route("/dogs", get(dogs_handler))
        .route("/dogs/register", post(register_dog_handler))
        .route("/dogs/{id}", axum::routing::delete(deregister_handler))
        .route("/dogs/{id}/heartbeat", post(heartbeat_handler))
        .route("/crystals", get(crystals_handler))
        .route("/crystal", post(create_crystal_handler))
        .route(
            "/crystal/{id}",
            get(crystal_handler).delete(delete_crystal_handler),
        )
        .route("/crystal/{id}/observe", post(observe_crystal_hypha_handler))
        .route("/crystal/{id}/shatter", post(shatter_crystal_handler))
        .route("/usage", get(usage_handler))
        .route("/verdict/{id}", get(get_verdict_handler))
        .route("/verdicts", get(list_verdicts_handler))
        .route("/agents", get(agents_handler))
        .route("/observations", get(observations_handler))
        .route("/sessions", get(sessions_handler))
        .route("/session/{agent_id}/compliance", get(compliance_handler))
        .route("/compliance", get(compliance_trend_handler))
        .route("/audit", get(audit_handler))
        .route("/state-history", get(state_history_handler))
        .route("/agent-tasks", post(agent_tasks::dispatch_task_handler))
        .route("/agent-tasks", get(agent_tasks::list_tasks_handler))
        .route(
            "/agent-tasks/completed",
            get(agent_tasks::list_completed_tasks_handler),
        )
        .route(
            "/agent-tasks/{id}/result",
            post(agent_tasks::complete_task_handler),
        )
        .route("/observe", post(observe_handler))
        .route("/event", post(event_handler))
        .route("/fleet-stats", get(fleet_stats_handler))
        .route("/inference/route", post(inference_route_handler))
        .route("/inference/candidates", get(inference_candidates_handler))
        .route("/inference/remediate", get(remediate_handler))
        .route("/inference/start", post(inference_start_handler))
        .route(
            "/inference/remediate-dog",
            post(inference_remediate_handler),
        )
        .route("/inference/list-models", get(list_models_handler))
        .route("/inference/slots", get(inference_slots_handler))
        .route("/v1/chat/completions", post(proxy_chat_completions))
        .route("/v1/models", get(proxy_models))
        .route("/coord/register", post(coord_register_handler))
        .route("/coord/claim", post(coord_claim_handler))
        .route("/coord/claim-batch", post(coord_claim_batch_handler))
        .route("/coord/release", post(coord_release_handler))
        .route("/coord/heartbeat", post(coord_heartbeat_handler))
        .route("/coord/scope", post(coord_scope_handler))
        .route("/coord/who", get(coord_who_handler))
        .route(
            "/dispatch/zone-activity",
            get(dispatch::zone_activity_handler),
        )
        .route(
            "/phone-numbers/blocklist",
            get(phone_numbers::blocklist_handler),
        )
        .route(
            "/phone-numbers/reporter-stats",
            get(phone_numbers::reporter_stats_handler),
        )
        .route("/soma/request", post(soma_request_handler))
        .route("/mail/health", get(mail_health))
        .route("/mail/inbox", get(inbox))
        .route("/mail/messages/{id}", get(fetch_message))
        .route("/mail/send", post(send))
        .route("/mail/sync", post(sync))
        .route("/mail/mark-read/{id}", post(mark_read))
        .route("/mail/search", post(search))
        .route("/mail/unread", get(unread))
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
        // Private Network Access: MUST be outermost layer (wraps CORS preflight).
        // Chrome blocks HTTPS-public → local-network requests without this header
        // on BOTH the preflight OPTIONS response AND the actual response.
        // Tailscale Funnel resolves to a local address; Vercel UI needs this.
        .layer(axum::middleware::from_fn(
            |request: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut response = next.run(request).await;
                response.headers_mut().insert(
                    axum::http::HeaderName::from_static("access-control-allow-private-network"),
                    axum::http::HeaderValue::from_static("true"),
                );
                response
            },
        ))
        .with_state(state)
}
