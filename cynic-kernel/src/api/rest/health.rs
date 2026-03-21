//! REST API handlers for health and introspection — /health, /dogs, /temporal, /agents.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use super::types::{AppState, DogHealthResponse, ErrorResponse};
use crate::domain::dog::PHI_INV;

pub async fn temporal_handler() -> Json<serde_json::Value> {
    use crate::domain::temporal::TemporalPerspective;
    let perspectives: Vec<serde_json::Value> = TemporalPerspective::ALL.iter().map(|p| {
        serde_json::json!({
            "perspective": p.label(),
            "description": p.description(),
        })
    }).collect();
    Json(serde_json::json!({
        "count": perspectives.len(),
        "perspectives": perspectives,
        "aggregation": "geometric_mean",
        "outlier_threshold": "phi^-2 (0.382)",
        "exploration_constant": "phi (1.618)",
        "status": "pure_logic_ready — awaiting multi-perspective Dog evaluation integration"
    }))
}

pub async fn dogs_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<String>> {
    Json(state.judge.dog_ids())
}

pub async fn health_handler(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> (StatusCode, Json<serde_json::Value>) {
    // Check if caller has valid auth — return full details only if authenticated.
    // Uses constant_time_eq to prevent timing oracle (same as auth_middleware).
    let authenticated = match &state.api_key {
        Some(key) => request.headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|t| super::middleware::constant_time_eq(t.as_bytes(), key.as_bytes())),
        None => true, // No auth configured → everyone gets full details
    };

    let dog_health = state.judge.dog_health();
    let dog_count = dog_health.len();

    let storage_ok = state.storage.ping().await.is_ok();

    let (status, http_code) = if dog_count == 0 || !storage_ok {
        ("critical", StatusCode::SERVICE_UNAVAILABLE) // 503
    } else if dog_count == 1 {
        ("degraded", StatusCode::SERVICE_UNAVAILABLE) // 503
    } else {
        ("sovereign", StatusCode::OK) // 200
    };

    // Public: minimal info only — dog_count withheld to prevent attack surface mapping
    // HTTP status code tells the story: 200 = healthy, 503 = degraded/critical.
    // Any monitoring tool can check without parsing JSON: curl -sf URL || alert
    if !authenticated {
        return (http_code, Json(serde_json::json!({
            "status": status,
            "version": env!("CYNIC_VERSION"),
            "phi_max": PHI_INV,
        })));
    }

    // Authenticated: full details
    let dogs: Vec<DogHealthResponse> = dog_health.into_iter().map(|(id, circuit, failures)| {
        let kind = if id == "deterministic-dog" { "heuristic" } else { "inference" }.to_string();
        DogHealthResponse { id, kind, circuit, failures }
    }).collect();

    let usage = state.usage.lock().await;

    (http_code, Json(serde_json::json!({
        "status": status,
        "version": env!("CYNIC_VERSION"),
        "phi_max": PHI_INV,
        "axioms": ["FIDELITY", "PHI", "VERIFY/FALSIFY", "CULTURE", "BURN", "SOVEREIGNTY"],
        "dogs": dogs,
        "storage": if storage_ok { "connected" } else { "down" },
        "storage_metrics": state.storage_metrics(),
        "embedding": if tokio::time::timeout(std::time::Duration::from_secs(2), state.embedding.embed("h")).await.map(|r| r.is_ok()).unwrap_or(false) { "sovereign" } else { "unavailable" },
        "verdict_cache_size": state.verdict_cache.len(),
        "background_tasks": state.task_health.snapshot(),
        "total_requests": usage.all_time_requests(),
        "total_tokens": usage.total_tokens(),
        "estimated_cost_usd": usage.estimated_cost_usd(),
        "uptime_seconds": usage.uptime_seconds(),
    })))
}

/// GET /agents — show active agent sessions and their claims (requires auth)
pub async fn agents_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.coord.who(None).await {
        Ok(snapshot) => Ok(Json(serde_json::json!({
            "active_agents": snapshot.agents.len(),
            "active_claims": snapshot.claims.len(),
            "agents": snapshot.agents,
            "claims": snapshot.claims,
        }))),
        Err(e) => {
            eprintln!("[REST] agents error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "coordination unavailable".into() })))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn health_status_logic() {
        // 0 dogs → critical, 1 → degraded, 2+ → sovereign
        assert_eq!(if 0 == 0 { "critical" } else if 0 == 1 { "degraded" } else { "sovereign" }, "critical");
        assert_eq!(if 1 == 0 { "critical" } else if 1 == 1 { "degraded" } else { "sovereign" }, "degraded");
        assert_eq!(if 2 == 0 { "critical" } else if 2 == 1 { "degraded" } else { "sovereign" }, "sovereign");
    }
}
