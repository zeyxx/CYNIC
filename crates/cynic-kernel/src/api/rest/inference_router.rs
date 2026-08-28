//! Inference routing — selects fastest healthy node for Dog evaluation.
//! Queries fleet stats to route inference work by latency + success rate.
//! Also provides operational endpoints to start/remediate dogs.
//! Live probing: each list-models call probes actual node state + emits observations.

use axum::{extract::State, http::StatusCode, response::Json};
use cynic_inference::{DefaultInferenceRouter, InferenceRoutingPort};
use cynic_inference::application::fleet_routing::{FleetNodeStat, InferenceRouteDecisionView};
use cynic_inference::application::model_catalog::{
    ListModelsResponse, ModelInfo, ModelProbeObservation, deterministic_model_info,
    project_model_info, should_probe_model,
};
use cynic_inference::application::node_runtime_ops::{
    probe_model_runtime, remediate_dog_runtime, start_dog_runtime,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::inference_observation::{store_probe_observation, store_recovery_observation};
use super::types::{AppState, ErrorResponse};

#[derive(Debug, Deserialize)]
pub struct DogOperationRequest {
    #[serde(deserialize_with = "super::types::deserialize_dog_id")]
    pub dog: String, // dog name: "qwen25-7b-core", "qwen35-9b-gpu", etc.
}

#[derive(Debug, Serialize)]
pub struct DogOperationResponse {
    pub dog: String,
    pub operation: String, // "start" or "remediate"
    pub status: String,    // "succeeded", "failed", "timed_out", etc.
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InferenceRouteRequest {
    pub domain: Option<String>, // Optional domain hint for Dog selection
    pub content_len: usize,     // Content length (for load estimation)
}

/// POST /inference/route — select best node for inference work.
/// Queries fleet stats over a time window, ranks nodes by (success_rate DESC, latency ASC).
/// Returns the fastest healthy node suitable for the domain.
pub async fn inference_route_handler(
    State(state): State<Arc<AppState>>,
    Json(_req): Json<InferenceRouteRequest>,
) -> Result<Json<InferenceRouteDecisionView>, (StatusCode, Json<ErrorResponse>)> {
    // Query fleet stats: 1-hour window, top 20 nodes
    let window_secs = 3600u64;
    let limit = 20u32;

    let nodes = state
        .storage
        .fleet_stats(window_secs, limit)
        .await
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: format!("fleet_stats unavailable: {e}"),
                }),
            )
        })?;

    let router = DefaultInferenceRouter;
    let Some(route) = router.select_route(storage_fleet_stats_to_domain(nodes)) else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "no healthy nodes available (all below 70% success rate)".into(),
            }),
        ));
    };

    Ok(Json(route))
}

/// GET /inference/candidates?domain=token — list candidate nodes ranked by health.
/// Returns up to 5 best nodes for a domain, sorted by quality.
pub async fn inference_candidates_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(_params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let window_secs = 3600u64;
    let limit = 20u32;

    let nodes = state
        .storage
        .fleet_stats(window_secs, limit)
        .await
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: format!("fleet_stats unavailable: {e}"),
                }),
            )
        })?;

    let router = DefaultInferenceRouter;
    let candidates = router.candidate_views(storage_fleet_stats_to_domain(nodes), 5);

    Ok(Json(serde_json::json!({
        "candidates": candidates,
        "window_secs": window_secs,
        "timestamp": chrono::Utc::now().timestamp_millis(),
    })))
}

fn storage_fleet_stats_to_domain(
    nodes: Vec<(String, u64, f64, u64, String)>,
) -> Vec<FleetNodeStat> {
    nodes
        .into_iter()
        .map(
            |(node, avg_latency_ms, success_rate, age_secs, failure_reason)| FleetNodeStat {
                node,
                avg_latency_ms,
                success_rate,
                age_secs,
                failure_reason,
            },
        )
        .collect()
}

#[derive(Debug, Serialize)]
pub struct RemediateResponse {
    pub degraded_nodes: Vec<DegradedNodeInfo>,
    pub timestamp: i64,
    pub window_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct DegradedNodeInfo {
    pub node: String,
    pub failure_reason: String,
    pub fatal_count: u64,
    pub remediation_status: String, // "attempted" | "skipped" | "unknown"
}

/// GET /inference/remediate — detect and attempt recovery of degraded nodes.
/// Queries last 30 minutes of events, identifies nodes with >80% fatal failures,
/// and attempts recovery (restart service via MCP using ts_exec).
/// Timeout: 30s per node, circuit-break on 3 consecutive failures.
pub async fn remediate_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<RemediateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let window_secs = 1800u64; // 30 minutes
    let fatal_threshold = 0.8; // 80% fatal failures

    let degraded_nodes = state
        .storage
        .list_degraded_nodes(window_secs, fatal_threshold)
        .await
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: format!("list_degraded_nodes unavailable: {e}"),
                }),
            )
        })?;

    let mut remediated: Vec<DegradedNodeInfo> = Vec::new();

    for (node, failure_reason, fatal_count, _last_fatal_secs) in degraded_nodes {
        let recovery = remediate_dog_runtime(&node).await;

        // Emit observation of recovery attempt
        let obs_status = if recovery.status.contains("succeeded") {
            "succeeded"
        } else if recovery.status.contains("timed_out") {
            "timed_out"
        } else {
            "failed"
        };
        let _ = store_recovery_observation(state.storage.as_ref(), &node, obs_status).await;

        remediated.push(DegradedNodeInfo {
            node,
            failure_reason,
            fatal_count,
            remediation_status: recovery.status,
        });
    }

    Ok(Json(RemediateResponse {
        degraded_nodes: remediated,
        timestamp: chrono::Utc::now().timestamp_millis(),
        window_secs,
    }))
}

/// Attempt to recover a degraded node by restarting llama-server.
/// Returns: "succeeded", "failed", "timed_out", or "circuit_broken".
/// Emit an observation of recovery attempt for observability.
/// POST /lifecycle/inference/start — start a specific dog (llama-server service).
/// body: {"dog": "qwen25-7b-core"}
/// Returns: operation status (succeeded, failed, timed_out).
pub async fn inference_start_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DogOperationRequest>,
) -> Result<Json<DogOperationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let node = dog_to_node(&state.backend_configs, &req.dog);

    let outcome = start_dog_runtime(&node).await;

    // Emit observation
    let _ =
        store_recovery_observation(state.storage.as_ref(), &outcome.node, &outcome.status).await;

    Ok(Json(DogOperationResponse {
        dog: req.dog,
        operation: "start".to_string(),
        status: outcome.status,
        timestamp: chrono::Utc::now().timestamp_millis(),
    }))
}

/// POST /inference/remediate — remediate (restart) a specific dog.
/// body: {"dog": "qwen25-7b-core"}
/// Returns: operation status (succeeded, failed, timed_out).
pub async fn inference_remediate_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DogOperationRequest>,
) -> Result<Json<DogOperationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let node = dog_to_node(&state.backend_configs, &req.dog);
    let outcome = remediate_dog_runtime(&node).await;

    let _ =
        store_recovery_observation(state.storage.as_ref(), &outcome.node, &outcome.status).await;

    Ok(Json(DogOperationResponse {
        dog: req.dog,
        operation: "remediate".to_string(),
        status: outcome.status,
        timestamp: chrono::Utc::now().timestamp_millis(),
    }))
}

/// Returns the node assigned to a Dog.
/// Dynamically queries the fleet configuration rather than hardcoding names.
pub(crate) fn dog_to_node(backends: &[crate::infra::config::BackendConfig], dog: &str) -> String {
    crate::infra::dog_node_resolver::resolve_dog_node(dog, backends)
}

/// LIVE PROBING: queries actual llama-server on each node.
/// Shows expected vs actual model, mismatch detection, emits observations.
pub async fn list_models_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ListModelsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut models: Vec<ModelInfo> = vec![deterministic_model_info()];

    for cfg in &state.backend_configs {
        let dog = &cfg.name;
        let node = cfg.fleet_node.as_deref().unwrap_or(dog);
        let expected = &cfg.model;
        let inference_config = cfg.to_inference_backend_config();

        let probe = if should_probe_model(&inference_config) {
            let probe = probe_model_runtime(node, expected).await;
            ModelProbeObservation {
                actual_model: probe.actual_model,
                reachable: probe.reachable,
                latency_ms: probe.latency_ms,
                failure_reason: probe.failure_reason,
            }
        } else {
            ModelProbeObservation::skipped()
        };

        if let Err(e) = store_probe_observation(state.storage.as_ref(), dog, node, &probe).await {
            tracing::warn!("probe observation emit failed for {}/{}: {}", node, dog, e);
        }

        models.push(project_model_info(&inference_config, node, probe));
    }

    Ok(Json(ListModelsResponse {
        models,
        timestamp: chrono::Utc::now().timestamp_millis(),
    }))
}

/// GET /inference/slots — aggregated slot state per sovereign backend.
/// Soma L2: external dispatchers (Hermes, nightshift) query this before spawning.
/// Returns per-dog slot counts, utilization, per-slot context, and freshness.
pub async fn inference_slots_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let snapshot = state.slot_tracker.snapshot();
    let backends: Vec<serde_json::Value> = snapshot
        .into_iter()
        .map(|(dog_id, slots)| {
            serde_json::json!({
                "dog_id": dog_id,
                "total_slots": slots.total,
                "busy_slots": slots.busy,
                "free_slots": slots.total.saturating_sub(slots.busy),
                "utilization": (slots.utilization() * 100.0).round() / 100.0,
                "per_slot_ctx": slots.per_slot_ctx,
                "slots": slots.slots,
            })
        })
        .collect();

    Json(serde_json::json!({
        "backends": backends,
        "timestamp": chrono::Utc::now().timestamp_millis(),
    }))
}

pub async fn dogs_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, (StatusCode, Json<ErrorResponse>)> {
    let mut dogs = vec!["deterministic-dog".to_string()];
    for cfg in &state.backend_configs {
        dogs.push(cfg.name.clone());
    }
    Ok(Json(dogs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_route_request_deserializes() {
        let json = r#"{"domain":"token","content_len":512}"#;
        let req: InferenceRouteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.domain, Some("token".into()));
        assert_eq!(req.content_len, 512);
    }
}
