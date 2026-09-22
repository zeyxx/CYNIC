use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
    http::StatusCode,
};
use cynic_agent_bus::*;

pub async fn health() -> &'static str {
    "ok"
}

pub async fn create_task(
    State(ledger): State<Arc<InMemoryLedger>>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let task_id = payload["id"].as_str().unwrap_or("unknown").to_string();
    let task = Task {
        id: task_id.clone(),
        parent_task_id: None,
        created_by: AgentId([0u8; 16]),
        assigned_to: AgentId([0u8; 16]),
        kind: TaskKind::Solo,
        subject: payload["subject"].as_str().unwrap_or("").to_string(),
        body: payload["body"].as_str().unwrap_or("").to_string(),
        status: TaskStatus::Queued,
        result: None,
        error: None,
        created_at_unix_s: now_unix_s(),
        updated_at_unix_s: now_unix_s(),
        expires_at_unix_s: 0,
        budget: Budget {
            actions: payload["budget"]["actions"].as_u64().unwrap_or(10) as u32,
            wall_seconds: payload["budget"]["wall_seconds"].as_u64().unwrap_or(60) as u32,
        },
    };

    match ledger.create_task(task).await {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({ "id": task_id, "status": "created" }))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))),
    }
}

pub async fn get_task(
    State(ledger): State<Arc<InMemoryLedger>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match ledger.get_task(&id).await {
        Some(task) => (StatusCode::OK, Json(serde_json::json!({
            "id": task.id,
            "status": task.status,
            "subject": task.subject,
            "body": task.body,
            "result": task.result,
        }))),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "not found" }))),
    }
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
