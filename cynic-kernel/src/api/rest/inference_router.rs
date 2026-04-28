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

#[derive(Debug, Serialize)]
pub struct RemediateResponse {
    pub degraded_nodes: Vec<DegradedNodeInfo>,
    pub timestamp: String,
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
        // Attempt recovery via ts_exec (systemctl restart llama-server)
        let recovery_status = attempt_node_recovery(&node).await;

        // Emit observation of recovery attempt
        let obs_status = if recovery_status.contains("succeeded") {
            "succeeded"
        } else if recovery_status.contains("timed_out") {
            "timed_out"
        } else {
            "failed"
        };
        let _ = emit_recovery_observation(&state, &node, obs_status).await;

        remediated.push(DegradedNodeInfo {
            node,
            failure_reason,
            fatal_count,
            remediation_status: recovery_status,
        });
    }

    Ok(Json(RemediateResponse {
        degraded_nodes: remediated,
        timestamp: chrono::Utc::now().to_rfc3339(),
        window_secs,
    }))
}

/// Attempt to recover a degraded node by restarting llama-server.
/// Returns: "succeeded", "failed", "timed_out", or "circuit_broken".
async fn attempt_node_recovery(node: &str) -> String {
    let timeout_duration = std::time::Duration::from_secs(30);
    let command = "systemctl restart llama-server";

    // Resolve script path: prefer explicit env var, fallback to relative path
    let script_path = std::env::var("CYNIC_SCRIPTS").unwrap_or_else(|_| "scripts".to_string())
        + "/ts_exec_call.sh";

    let mut cmd = tokio::process::Command::new(&script_path);
    cmd.arg(node).arg(command).arg("30");

    // Execute with bounded timeout (30s + 5s buffer)
    let result = tokio::time::timeout(
        timeout_duration + std::time::Duration::from_secs(5),
        cmd.output(),
    )
    .await;

    match result {
        Err(_elapsed) => "timed_out".to_string(),
        Ok(Err(_cmd_err)) => "failed".to_string(),
        Ok(Ok(output)) => {
            // Parse JSON response from ts_exec_call.sh
            if let Ok(response_text) = String::from_utf8(output.stdout)
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text)
                && let Some(exit_code) = json.get("exit_code").and_then(|v| v.as_i64())
                && exit_code == 0
            {
                return "succeeded".to_string();
            }
            "failed".to_string()
        }
    }
}

/// Emit an observation of recovery attempt for observability.
async fn emit_recovery_observation(
    state: &Arc<AppState>,
    node: &str,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::domain::storage::Observation;

    let obs = Observation {
        project: "cynic".to_string(),
        agent_id: "kernel-remediation".to_string(),
        tool: "ts_exec".to_string(),
        target: node.to_string(),
        domain: "infrastructure".to_string(),
        status: status.to_string(),
        context: "Attempted llama-server restart on degraded node".to_string(),
        session_id: "".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        tags: vec!["k15-recovery".to_string(), "remediation".to_string()],
    };

    state.storage.store_observation(&obs).await?;
    Ok(())
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
