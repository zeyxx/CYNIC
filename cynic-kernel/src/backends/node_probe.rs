//! Runtime node health probing — K15 observability.
//!
//! Lives in the backends adapter layer (K2): the only place `reqwest::Client`
//! is allowed to be constructed for runtime probing of inference nodes.

use std::time::Instant;

/// Probe a single node's llama-server for live status.
/// Returns: (actual_model, reachable, latency_ms, failure_reason).
/// Failure reasons: "none", "timeout", "unreachable", "parse_error", "mismatch".
pub async fn probe_node(node: &str, expected_model: &str) -> (String, bool, u64, String) {
    // K11: Hardcoded port 8080. Extract to config on 2nd consumer (remediate_handler).
    let url = format!("http://{node}:8080/health");
    let timeout = std::time::Duration::from_secs(5);

    let start = Instant::now();

    // Create HTTP client with timeout
    let Ok(client) = reqwest::Client::builder().timeout(timeout).build() else {
        return (
            "N/A".to_string(),
            false,
            0,
            "client_init_failed".to_string(),
        );
    };

    // GET /health
    let response = match tokio::time::timeout(timeout, client.get(&url).send()).await {
        Err(_) => return ("N/A".to_string(), false, 0, "timeout".to_string()),
        Ok(Err(_)) => return ("N/A".to_string(), false, 0, "unreachable".to_string()),
        Ok(Ok(r)) => r,
    };

    let latency_ms = start.elapsed().as_millis() as u64;

    // Parse response body to extract model name
    let Ok(body) = response.text().await else {
        return (
            "N/A".to_string(),
            true,
            latency_ms,
            "parse_error".to_string(),
        );
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
