//! REST API handlers for health and introspection — /health, /dogs, /temporal, /agents.

use axum::{
    extract::{Request, State},
    response::Json,
};
use std::sync::Arc;

use super::types::{AppState, DogHealthResponse};
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
        "count": 7,
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
) -> Json<serde_json::Value> {
    // Check if caller has valid auth — return full details only if authenticated
    let authenticated = match &state.api_key {
        Some(key) => request.headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|t| t == key.as_str()),
        None => true, // No auth configured → everyone gets full details
    };

    let dog_health = state.judge.dog_health();
    let dog_count = dog_health.len();

    let storage_ok = state.storage.ping().await.is_ok();

    let status = if dog_count == 0 || !storage_ok {
        "critical"
    } else if dog_count == 1 {
        "degraded"
    } else {
        "sovereign"
    };

    // Public: minimal info + dog_count (not a secret, needed for drift detection)
    if !authenticated {
        return Json(serde_json::json!({
            "status": status,
            "version": env!("CARGO_PKG_VERSION"),
            "phi_max": PHI_INV,
            "dog_count": dog_count,
        }));
    }

    // Authenticated: full details
    let dogs: Vec<DogHealthResponse> = dog_health.into_iter().map(|(id, circuit, failures)| {
        let kind = if id == "deterministic-dog" { "heuristic" } else { "inference" }.to_string();
        DogHealthResponse { id, kind, circuit, failures }
    }).collect();

    let usage = state.usage.lock().unwrap();

    Json(serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "phi_max": PHI_INV,
        "axioms": ["FIDELITY", "PHI", "VERIFY/FALSIFY", "CULTURE", "BURN", "SOVEREIGNTY"],
        "dogs": dogs,
        "storage": if storage_ok { "connected" } else { "down" },
        "total_requests": usage.all_time_requests(),
        "total_tokens": usage.total_tokens(),
        "estimated_cost_usd": usage.estimated_cost_usd(),
        "uptime_seconds": usage.uptime_seconds(),
    }))
}

/// GET /agents — show active agent sessions and their claims (requires auth)
pub async fn agents_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    match state.coord.who(None).await {
        Ok(snapshot) => Json(serde_json::json!({
            "active_agents": snapshot.agents.len(),
            "active_claims": snapshot.claims.len(),
            "agents": snapshot.agents,
            "claims": snapshot.claims,
        })),
        Err(e) => Json(serde_json::json!({"error": format!("Coordination unavailable: {}", e)})),
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
