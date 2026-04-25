//! Agent task queue persistence in SurrealDB.

use super::SurrealHttpStorage;
use crate::domain::storage::{AgentTask, StorageError};
use serde_json::Value;

use super::crystals;

fn row_to_agent_task(row: &Value) -> Option<AgentTask> {
    row.as_object().map(|obj| AgentTask {
        id: obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        kind: obj
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        domain: obj
            .get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        content: obj
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        status: obj
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string(),
        result: obj.get("result").and_then(|v| v.as_str()).map(String::from),
        created_at: obj
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        completed_at: obj
            .get("completed_at")
            .and_then(|v| v.as_str())
            .map(String::from),
        agent_id: obj
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        error: obj.get("error").and_then(|v| v.as_str()).map(String::from),
    })
}

/// Store a new agent task. Returns task ID.
pub(super) async fn store_agent_task(
    storage: &SurrealHttpStorage,
    task: &AgentTask,
) -> Result<String, StorageError> {
    let agent_id = task
        .agent_id
        .as_ref()
        .map(|id| format!("'{}'", id))
        .unwrap_or_else(|| "null".to_string());
    let query = format!(
        "INSERT INTO agent_tasks {{ kind: '{}', domain: '{}', content: '{}', status: 'pending', created_at: '{}', agent_id: {} }} RETURN id;",
        task.kind, task.domain, task.content, task.created_at, agent_id
    );
    storage.query_one(&query).await?;
    Ok(task.id.clone())
}

/// List pending tasks of a specific kind.
pub(super) async fn list_pending_agent_tasks(
    storage: &SurrealHttpStorage,
    kind: &str,
    limit: u32,
) -> Result<Vec<AgentTask>, StorageError> {
    let query = format!(
        "SELECT * FROM agent_tasks WHERE kind = '{}' AND status = 'pending' ORDER BY created_at ASC LIMIT {};",
        kind, limit
    );
    let rows = storage.query_one(&query).await?;
    Ok(rows.iter().filter_map(row_to_agent_task).collect())
}

/// Get a single agent task by ID.
pub(super) async fn get_agent_task(
    storage: &SurrealHttpStorage,
    task_id: &str,
) -> Result<Option<AgentTask>, StorageError> {
    let query = format!("SELECT * FROM agent_tasks WHERE id = '{}';", task_id);
    let rows = storage.query_one(&query).await?;
    Ok(rows.first().and_then(row_to_agent_task))
}

/// Mark task as processing.
pub(super) async fn mark_agent_task_processing(
    storage: &SurrealHttpStorage,
    task_id: &str,
) -> Result<(), StorageError> {
    let query = format!(
        "UPDATE agent_tasks SET status = 'processing' WHERE id = '{}';",
        task_id
    );
    storage.query_one(&query).await?;
    Ok(())
}

/// Update task result and status. K15: Also observe the result as a crystal.
pub(super) async fn update_agent_task_result(
    storage: &SurrealHttpStorage,
    task_id: &str,
    result: Option<&str>,
    error: Option<&str>,
) -> Result<(), StorageError> {
    let status = if error.is_some() {
        "failed"
    } else {
        "completed"
    };
    let now = chrono::Utc::now().to_rfc3339();
    let result_str = result
        .map(|r| format!("'{}'", r))
        .unwrap_or_else(|| "null".to_string());
    let error_str = error
        .map(|e| format!("'{}'", e))
        .unwrap_or_else(|| "null".to_string());

    let query = format!(
        "UPDATE agent_tasks SET status = '{}', result = {}, error = {}, completed_at = '{}' WHERE id = '{}';",
        status, result_str, error_str, now, task_id
    );
    storage.query_one(&query).await?;

    // K15: Observe the task result as a crystal to close audit→observation loop.
    // Fire-and-forget: error in observation should not fail the task update.
    if let Ok(Some(task)) = get_agent_task(storage, task_id).await {
        let obs_content = task
            .result
            .as_deref()
            .or(task.error.as_deref())
            .unwrap_or("no result or error");
        let obs_score = if task.error.is_some() { 0.0 } else { 1.0 };
        let _ = crystals::observe_crystal(
            storage,
            &task.id,
            obs_content,
            &task.domain,
            obs_score,
            task.completed_at.as_deref().unwrap_or(&now),
            1,
            &task.id,
            &task.status,
        )
        .await;
    }

    Ok(())
}
