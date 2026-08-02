//! REST API — JSON interface for external clients (React, curl, etc.)

pub mod agent_session;
pub mod agent_tasks;
pub mod agent_workspace;
pub mod attention;
pub mod auth;
pub mod coord;
pub mod coord_lease;
pub mod data;
pub mod discovery;
pub mod dispatch;
pub mod event;
pub mod events;
pub mod health;
pub mod inference_observation;
pub mod inference_proxy;
pub mod inference_router;
pub mod local_peer;
pub mod mail;
pub mod mesh;
pub mod middleware;
pub mod mint_permit;
pub mod node_control;
pub mod node_enrollment;
pub mod observe;
pub mod phone_numbers;
pub mod response;
pub mod services;
pub mod soma;
pub mod types;
pub mod webhook;

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

use self::agent_workspace::{
    finalize_handler as workspace_finalize_handler, list_handler as workspace_list_handler,
    release_handler as workspace_release_handler, renew_handler as workspace_renew_handler,
    reserve_handler as workspace_reserve_handler,
};
use self::attention::attention_handler;
use self::auth::{auth_input_handler, auth_verify_handler};
use self::coord::{
    coord_claim_batch_handler, coord_claim_handler, coord_heartbeat_handler,
    coord_register_handler, coord_scope_handler, coord_topology_handler, coord_who_handler,
};
use self::coord_lease::{
    coord_finalize_handler, coord_release_handler, coord_renew_handler, coord_verify_handler,
};
use self::data::{
    audit_handler, compliance_handler, compliance_trend_handler, create_crystal_handler,
    crystal_handler, crystals_handler, delete_crystal_handler, observations_handler,
    observe_crystal_hypha_handler, proof_of_history_handler, public_observations_handler,
    sessions_handler, shatter_crystal_handler, usage_handler, verdicts_handler,
};
use self::discovery::{
    api_layer_alignment_handler, api_topology_handler, identity_handler, openapi_handler,
};
use self::event::{event_handler, fleet_stats_handler};
use self::health::{
    agents_handler, health_handler, liveness_handler, metrics_handler, readiness_handler,
    state_history_handler,
};
use self::inference_proxy::{proxy_chat_completions, proxy_models};
use self::inference_router::{
    dogs_handler, inference_candidates_handler, inference_remediate_handler,
    inference_route_handler, inference_slots_handler, inference_start_handler, list_models_handler,
    remediate_handler,
};
use self::mail::{
    fetch_message, health as mail_health, inbox, mark_read, search, send, sync, unread,
};
use self::mesh::{list_mesh_nodes_handler, list_mesh_routes_handler, mesh_publish_handler};
use self::middleware::{audit_middleware, auth_middleware, rate_limit_middleware};
use self::mint_permit::mint_permit_handler;
use self::observe::{judge_handler, observe_handler};
use self::services::{
    list_services_handler, register_service_handler, service_heartbeat_handler,
    unregister_service_handler,
};
use self::soma::soma_request_handler;

use self::agent_session::{
    renew_agent_session_handler, revoke_agent_session_handler, root_agent_launch_handler,
};
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

    if state.role_keys.has_any_authentication_authority() {
        tracing::info!("REST Bearer auth ENABLED");
    } else {
        tracing::warn!(
            "REST Bearer auth has no configured keys; unsigned requests will be rejected"
        );
    }

    Router::new()
        .route("/openapi.json", get(openapi_handler))
        .route("/identity", get(identity_handler))
        .route("/api/topology", get(api_topology_handler))
        .route("/api/layer-alignment", get(api_layer_alignment_handler))
        .route("/events/ws", axum::routing::get(ws_handler))
        .route("/health", get(health_handler))
        .route("/attention", get(attention_handler))
        .route("/auth/input", get(auth_input_handler))
        .route("/auth/verify", post(auth_verify_handler))
        .route("/auth/agent/root-launch", post(root_agent_launch_handler))
        .route("/node-enrollment", post(node_enrollment::enroll_handler))
        .route(
            "/node-enrollment/renew",
            post(node_enrollment::renew_handler),
        )
        .route(
            "/auth/agent/session/renew",
            post(renew_agent_session_handler),
        )
        .route(
            "/auth/agent/session/revoke",
            post(revoke_agent_session_handler),
        )
        .route("/live", get(liveness_handler))
        .route("/ready", get(readiness_handler))
        .route("/metrics", get(metrics_handler))
        .route("/events", get(events::events_handler))
        .route("/events", post(event_handler))
        .route("/permits/mint", post(mint_permit_handler))
        .route("/crystals", get(crystals_handler))
        .route("/crystals", post(create_crystal_handler))
        .route(
            "/crystals/{id}",
            get(crystal_handler).delete(delete_crystal_handler),
        )
        .route(
            "/crystals/{id}/observe",
            post(observe_crystal_hypha_handler),
        )
        .route("/crystals/{id}/shatter", post(shatter_crystal_handler))
        .route("/metrics/usage", get(usage_handler))
        .route("/coord/agents", get(agents_handler))
        .route("/observations/public", get(public_observations_handler))
        .route("/observations", get(observations_handler))
        .route("/sessions", get(sessions_handler))
        .route("/sessions/{agent_id}/compliance", get(compliance_handler))
        .route("/compliance", get(compliance_trend_handler))
        .route("/audit", get(audit_handler))
        .route("/proof-of-history", get(proof_of_history_handler))
        .route("/state-history", get(state_history_handler))
        .route(
            "/agent-proposals",
            get(agent_tasks::list_agent_proposals_handler),
        )
        .route(
            "/agent-proposals/{proposal_id}/review",
            post(agent_tasks::review_agent_proposal_handler),
        )
        .route("/agent-tasks", post(agent_tasks::dispatch_task_handler))
        .route(
            "/agent-workspace/claims/reserve",
            post(workspace_reserve_handler),
        )
        .route("/agent-workspace/claims/list", post(workspace_list_handler))
        .route(
            "/agent-workspace/claims/renew",
            post(workspace_renew_handler),
        )
        .route(
            "/agent-workspace/claims/release",
            post(workspace_release_handler),
        )
        .route(
            "/agent-workspace/claims/finalize",
            post(workspace_finalize_handler),
        )
        .route("/agent-tasks", get(agent_tasks::list_tasks_handler))
        .route("/agent-tasks/watch", get(agent_tasks::watch_tasks_handler))
        .route(
            "/agent-tasks/completed",
            get(agent_tasks::list_completed_tasks_handler),
        )
        .route(
            "/agent-tasks/{id}/result",
            post(agent_tasks::complete_task_handler),
        )
        .route(
            "/agent-tasks/{id}/claim",
            post(agent_tasks::claim_task_handler),
        )
        .route("/observe", post(observe_handler))
        .route("/judge", post(judge_handler))
        .route("/fleet/stats", get(fleet_stats_handler))
        .route("/inference/route", post(inference_route_handler))
        .route("/inference/candidates", get(inference_candidates_handler))
        .route("/inference/remediate", get(remediate_handler))
        .route("/lifecycle/inference/start", post(inference_start_handler))
        .route(
            "/lifecycle/inference/remediate-dog",
            post(inference_remediate_handler),
        )
        .route("/lifecycle/upgrade", post(inference_start_handler))
        .route("/dogs", get(dogs_handler))
        .route("/verdicts", get(verdicts_handler))
        .route("/inference/list-models", get(list_models_handler))
        .route("/inference/slots", get(inference_slots_handler))
        .route("/v1/chat/completions", post(proxy_chat_completions))
        .route("/v1/models", get(proxy_models))
        .route("/coord/register", post(coord_register_handler))
        .route("/coord/claim", post(coord_claim_handler))
        .route("/coord/claim-batch", post(coord_claim_batch_handler))
        .route("/coord/verify", post(coord_verify_handler))
        .route("/coord/renew", post(coord_renew_handler))
        .route("/coord/release", post(coord_release_handler))
        .route("/coord/finalize", post(coord_finalize_handler))
        .route("/coord/heartbeat", post(coord_heartbeat_handler))
        .route("/coord/scope", post(coord_scope_handler))
        .route("/coord/who", get(coord_who_handler))
        .route("/coord/topology", get(coord_topology_handler))
        .route(
            "/node-control/{node_id}/desired-state",
            get(node_control::desired_state_handler),
        )
        .route(
            "/node-control/{node_id}/status",
            get(node_control::status_handler).post(node_control::publish_status_handler),
        )
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
        .route(
            "/webhook/github/release",
            post(webhook::github_release_handler),
        )
        .route("/mesh/publish", post(mesh_publish_handler))
        .route("/mesh/nodes", get(list_mesh_nodes_handler))
        .route("/mesh/routes", get(list_mesh_routes_handler))
        .route("/mail/health", get(mail_health))
        .route("/mail/inbox", get(inbox))
        .route("/mail/messages/{id}", get(fetch_message))
        .route("/mail/send", post(send))
        .route("/mail/sync", post(sync))
        .route("/mail/mark-read/{id}", post(mark_read))
        .route("/mail/search", post(search))
        .route("/mail/unread", get(unread))
        // ── OS Service Process Table ──────────────────────────────────
        // Services register/heartbeat/unregister from their own code (no scripts).
        // GET  /services              → observed process table (all registered services)
        // POST /services/register     → announce on startup
        // POST /services/{id}/heartbeat → TTL refresh (from native heartbeat task)
        // DELETE /services/{id}       → explicit shutdown announce
        .route("/services", get(list_services_handler))
        .route("/services/register", post(register_service_handler))
        .route("/services/{id}/heartbeat", post(service_heartbeat_handler))
        .route(
            "/services/{id}",
            axum::routing::delete(unregister_service_handler),
        )
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
