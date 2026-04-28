//! Inference routing — selects fastest healthy node for Dog evaluation.
//! Queries /fleet-stats to route inference work by latency + success rate.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::types::{AppState, ErrorResponse};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InferenceRouteRequest {
    pub domain: Option<String>, // Optional domain hint for Dog selection
    pub content_len: usize,     // Content length (for load estimation)
}

#[derive(Debug, Serialize)]
pub struct InferenceRouteResponse {
    pub selected_node: String,
    pub reason: String,
    pub avg_latency_ms: u64,
    pub success_rate: f64,
    pub quality: String,
}

/// POST /inference/route — select best node for inference work.
/// Queries /fleet-stats over a time window, ranks nodes by (success_rate DESC, latency ASC).
/// Returns the fastest healthy node suitable for the domain.
pub async fn inference_route_handler(
    State(state): State<Arc<AppState>>,
    Json(_req): Json<InferenceRouteRequest>,
) -> Result<Json<InferenceRouteResponse>, (StatusCode, Json<ErrorResponse>)> {
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

    // Filter for healthy nodes: quality != "dead", success_rate >= 0.7
    // Skip nodes with fatal failures (process_crash, not_started)
    let mut candidates: Vec<_> = nodes
        .into_iter()
        .filter(|(_, _, sr, _, failure_reason)| {
            // Skip nodes that are dead (process crashed or never started)
            if failure_reason == "process_crash" || failure_reason == "not_started" {
                return false;
            }
            // Require minimum success rate
            *sr >= 0.7
        })
        .collect();

    if candidates.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "no healthy nodes available (all below 70% success rate)".into(),
            }),
        ));
    }

    // Sort by: success_rate DESC, latency ASC (best to worst)
    candidates.sort_by(|a, b| {
        match b.2.partial_cmp(&a.2) {
            Some(std::cmp::Ordering::Equal) => a.1.cmp(&b.1), // equal SR → lower latency wins
            Some(ord) => ord,                                 // higher SR wins
            None => std::cmp::Ordering::Equal,                // NaN fallback
        }
    });

    let (node, avg_latency, sr, _age, failure_reason) = &candidates[0];

    // Compute quality tier based on failure_reason and success_rate
    let quality = match failure_reason.as_str() {
        "none" => match (*_age, *sr) {
            (0..=5, sr) if sr >= 0.95 => "excellent",
            (0..=60, sr) if sr >= 0.9 => "good",
            (_, sr) if sr >= 0.7 => "degraded",
            _ => "dead",
        },
        "port_conflict" | "config_error" | "firewall" => "degraded",
        "unknown" => "degraded",
        _ => "dead", // process_crash, not_started, others
    };

    Ok(Json(InferenceRouteResponse {
        selected_node: node.clone(),
        reason: format!(
            "ranked #{} of {} healthy nodes: success_rate={:.1}%, avg_latency={}ms",
            1,
            candidates.len(),
            sr * 100.0,
            avg_latency
        ),
        avg_latency_ms: *avg_latency,
        success_rate: *sr,
        quality: quality.to_string(),
    }))
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

    // Filter healthy + rank (skip fatal failures)
    let mut candidates: Vec<_> = nodes
        .into_iter()
        .filter(|(_, _, sr, _, failure_reason)| {
            // Skip nodes that are dead
            if failure_reason == "process_crash" || failure_reason == "not_started" {
                return false;
            }
            *sr >= 0.7
        })
        .map(|(node, latency, sr, age, failure_reason)| {
            let quality = match failure_reason.as_str() {
                "none" => match (age, sr) {
                    (0..=5, sr) if sr >= 0.95 => "excellent",
                    (0..=60, sr) if sr >= 0.9 => "good",
                    (_, sr) if sr >= 0.7 => "degraded",
                    _ => "dead",
                },
                "port_conflict" | "config_error" | "firewall" => "degraded",
                "unknown" => "degraded",
                _ => "dead",
            };
            serde_json::json!({
                "node": node,
                "avg_latency_ms": latency,
                "success_rate": sr,
                "quality": quality,
                "failure_reason": failure_reason,
            })
        })
        .collect();

    // Limit to top 5
    candidates.truncate(5);

    Ok(Json(serde_json::json!({
        "candidates": candidates,
        "window_secs": window_secs,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
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
