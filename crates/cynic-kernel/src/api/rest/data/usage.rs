//! Usage tracking REST handlers.

use axum::{extract::State, response::Json};
use std::sync::Arc;

use crate::api::rest::types::AppState;

pub async fn usage_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let usage = state.usage.lock().await;
    let merged = usage.merged_dogs();
    let active_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut active_dogs: Vec<serde_json::Value> = Vec::new();
    let mut retired_tokens: u64 = 0;
    let mut retired_requests: u64 = 0;
    let mut retired_count: u32 = 0;

    for (id, d) in &merged {
        if active_ids.contains(id) {
            let avg_latency = d.total_latency_ms.checked_div(d.requests).unwrap_or(0);
            active_dogs.push(serde_json::json!({
                "dog_id": id,
                "prompt_tokens": d.prompt_tokens,
                "completion_tokens": d.completion_tokens,
                "total_tokens": d.total_tokens(),
                "requests": d.requests,
                "failures": d.failures,
                "avg_latency_ms": avg_latency,
            }));
        } else {
            retired_tokens += d.total_tokens();
            retired_requests += d.requests;
            retired_count += 1;
        }
    }
    active_dogs.sort_by(|a, b| b["requests"].as_u64().cmp(&a["requests"].as_u64()));
    Json(serde_json::json!({
        "total_tokens": usage.total_tokens(),
        "total_requests": usage.all_time_requests(),
        "estimated_cost_usd": usage.estimated_cost_usd(),
        "uptime_seconds": usage.uptime_seconds(),
        "per_dog": active_dogs,
        "retired": {
            "count": retired_count,
            "total_tokens": retired_tokens,
            "total_requests": retired_requests,
        },
    }))
}
