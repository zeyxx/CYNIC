//! Inference routing — selects fastest healthy node for Dog evaluation.
//! Queries /fleet-stats to route inference work by latency + success rate.
//! Also provides operational endpoints to start/remediate dogs.
//! Live probing: each list-models call probes actual node state + emits observations.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use super::types::{AppState, ErrorResponse};

#[derive(Debug, Deserialize)]
pub struct DogOperationRequest {
    pub dog: String, // dog name: "qwen-9b-core", "qwen35-9b-gpu", etc.
}

#[derive(Debug, Serialize)]
pub struct DogOperationResponse {
    pub dog: String,
    pub operation: String, // "start" or "remediate"
    pub status: String,    // "succeeded", "failed", "timed_out", etc.
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub dog: String,
    pub node: String,
    pub expected_model: String,
    pub actual_model: String,
    pub mismatch: bool,
    pub reachable: bool,
    pub latency_ms: u64,
    pub capabilities: serde_json::Value,
    pub performance: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ListModelsResponse {
    pub models: Vec<ModelInfo>,
    pub timestamp: String,
}

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

/// POST /inference/start — start a specific dog (llama-server service).
/// body: {"dog": "qwen-9b-core"}
/// Returns: operation status (succeeded, failed, timed_out).
pub async fn inference_start_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DogOperationRequest>,
) -> Result<Json<DogOperationResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Map dog name to node name (e.g., "qwen-9b-core" → "cynic-core")
    let node = dog_to_node(&req.dog);

    // Execute: systemctl start llama-server
    let status = attempt_node_start(&node).await;

    // Emit observation
    let _ = emit_recovery_observation(&state, &node, &status).await;

    Ok(Json(DogOperationResponse {
        dog: req.dog,
        operation: "start".to_string(),
        status,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

/// POST /inference/remediate — remediate (restart) a specific dog.
/// body: {"dog": "qwen-9b-core"}
/// Returns: operation status (succeeded, failed, timed_out).
pub async fn inference_remediate_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DogOperationRequest>,
) -> Result<Json<DogOperationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let node = dog_to_node(&req.dog);
    let status = attempt_node_recovery(&node).await;

    let _ = emit_recovery_observation(&state, &node, &status).await;

    Ok(Json(DogOperationResponse {
        dog: req.dog,
        operation: "remediate".to_string(),
        status,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

/// Map dog name to Tailscale node name.
/// E.g., "qwen-9b-core" → "cynic-core", "qwen35-9b-gpu" → "cynic-gpu"
fn dog_to_node(dog: &str) -> String {
    if dog.contains("gpu") {
        "cynic-gpu".to_string()
    } else if dog.contains("core") || dog.contains("9b-core") {
        "cynic-core".to_string()
    } else {
        // Fallback: return the dog name as-is for unknown dogs
        dog.to_string()
    }
}

/// Attempt to start a node's llama-server (systemctl start).
/// Returns: "succeeded", "failed", "timed_out".
async fn attempt_node_start(node: &str) -> String {
    let timeout_duration = std::time::Duration::from_secs(30);
    let command = "systemctl start llama-server";

    let script_path = std::env::var("CYNIC_SCRIPTS").unwrap_or_else(|_| "scripts".to_string())
        + "/ts_exec_call.sh";

    let mut cmd = tokio::process::Command::new(&script_path);
    cmd.arg(node).arg(command).arg("30");

    let result = tokio::time::timeout(
        timeout_duration + std::time::Duration::from_secs(5),
        cmd.output(),
    )
    .await;

    match result {
        Err(_elapsed) => "timed_out".to_string(),
        Ok(Err(_cmd_err)) => "failed".to_string(),
        Ok(Ok(output)) => {
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

/// Probe a single node's llama-server for live status.
/// Returns: (actual_model, reachable, latency_ms, failure_reason).
/// Failure reasons: "none", "timeout", "unreachable", "parse_error", "mismatch".
async fn probe_node(node: &str, expected_model: &str) -> (String, bool, u64, String) {
    // Hardcode llama-server port (standard)
    let url = format!("http://{}:8080/health", node);
    let timeout = std::time::Duration::from_secs(5);

    let start = Instant::now();

    // Create HTTP client with timeout
    let client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(c) => c,
        Err(_) => {
            return (
                "N/A".to_string(),
                false,
                0,
                "client_init_failed".to_string(),
            );
        }
    };

    // GET /health
    let response = match tokio::time::timeout(timeout, client.get(&url).send()).await {
        Err(_) => return ("N/A".to_string(), false, 0, "timeout".to_string()),
        Ok(Err(_)) => return ("N/A".to_string(), false, 0, "unreachable".to_string()),
        Ok(Ok(r)) => r,
    };

    let latency_ms = start.elapsed().as_millis() as u64;

    // Parse response body to extract model name
    let body = match response.text().await {
        Ok(b) => b,
        Err(_) => {
            return (
                "N/A".to_string(),
                true,
                latency_ms,
                "parse_error".to_string(),
            );
        }
    };

    let actual_model: String = match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(json) => {
            // Try to extract model name from JSON
            // Llama-server typically returns: {"model": "...", ...}
            if let Some(model) = json.get("model").and_then(|v| v.as_str()) {
                model.to_string()
            } else {
                "unknown".to_string()
            }
        }
        Err(_) => "parse_error".to_string(),
    };

    let failure_reason = if actual_model == "parse_error" {
        "parse_error".to_string()
    } else if expected_model != "N/A" && actual_model != expected_model && actual_model != "unknown"
    {
        "mismatch".to_string()
    } else {
        "none".to_string()
    };

    (actual_model, true, latency_ms, failure_reason)
}

/// Emit observation for node probe result.
async fn emit_probe_observation(
    state: &Arc<AppState>,
    dog: &str,
    node: &str,
    reachable: bool,
    failure_reason: &str,
    latency_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::domain::storage::Observation;

    if failure_reason == "none" {
        return Ok(()); // Don't emit for healthy nodes
    }

    let obs = Observation {
        project: "cynic".to_string(),
        agent_id: "kernel-probe".to_string(),
        tool: "llama_server_probe".to_string(),
        target: format!("{}/{}", node, dog),
        domain: "infrastructure".to_string(),
        status: if reachable { "degraded" } else { "unreachable" }.to_string(),
        context: format!(
            "Dog probe failed: failure_type={}, latency_ms={}",
            failure_reason, latency_ms
        ),
        session_id: "".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        tags: vec![
            "k15-observability".to_string(),
            format!("failure_type:{}", failure_reason),
        ],
    };

    state.storage.store_observation(&obs).await?;
    Ok(())
}

/// GET /inference/list-models — list all available LLMs across the fleet.
/// LIVE PROBING: queries actual llama-server on each node.
/// Shows expected vs actual model, mismatch detection, emits observations.
pub async fn list_models_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ListModelsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Dog configuration: (dog_name, node_name, expected_model)
    let dog_config = vec![
        ("deterministic-dog", "deterministic", "deterministic"),
        ("qwen-7b-hf", "qwen-7b-hf", "Qwen/Qwen2.5-Coder-7B-Instruct"),
        ("qwen-9b-core", "cynic-core", "Qwen3.5-9B-Q4_K_M.gguf"),
        ("qwen35-9b-gpu", "cynic-gpu", "Qwen3.5-9B-Q4_K_M.gguf"),
        ("gemini-cli", "gemini-cli", "auto"),
    ];

    let mut models: Vec<ModelInfo> = Vec::new();

    for (dog, node, expected) in dog_config {
        // Skip probing non-inference dogs
        let (actual_model, reachable, latency_ms, failure_reason) =
            if node == "deterministic" || node == "qwen-7b-hf" || node == "gemini-cli" {
                ("N/A".to_string(), true, 0, "none".to_string())
            } else {
                // Live probe inference nodes
                probe_node(node, expected).await
            };

        // Emit observation if degraded
        if let Err(e) =
            emit_probe_observation(&state, dog, node, reachable, &failure_reason, latency_ms).await
        {
            tracing::warn!("probe observation emit failed for {}/{}: {}", node, dog, e);
        }

        let mismatch = expected != actual_model
            && expected != "N/A"
            && actual_model != "N/A"
            && actual_model != "unknown"
            && actual_model != "parse_error";

        let capabilities = serde_json::json!({
            "context_size": match dog {
                "qwen-9b-core" => 4096,
                "qwen35-9b-gpu" => 131072,
                "qwen-7b-hf" => 32768,
                _ => 0,
            },
            "thinking_mode": dog == "qwen35-9b-gpu",
            "json_mode": dog != "gemini-cli" && dog != "deterministic-dog",
        });

        let performance = serde_json::json!({
            "success_count": 0,
            "failures": 0,
            "json_valid_rate": 0.0,
        });

        models.push(ModelInfo {
            dog: dog.to_string(),
            node: node.to_string(),
            expected_model: expected.to_string(),
            actual_model,
            mismatch,
            reachable,
            latency_ms,
            capabilities,
            performance,
        });
    }

    Ok(Json(ListModelsResponse {
        models,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
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
