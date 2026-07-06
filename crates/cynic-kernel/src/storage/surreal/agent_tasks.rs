//! Agent task queue persistence in SurrealDB.

use super::SurrealHttpStorage;
use crate::domain::storage::{AgentTask, StorageError};
use serde_json::Value;

use super::crystals;

fn extract_record_id(row: &Value) -> Option<String> {
    row.as_str()
        .map(String::from)
        .or_else(|| row.get("id").and_then(|v| v.as_str()).map(String::from))
}

fn value_as_i64(row: &Value, key: &str) -> i64 {
    row.get(key)
        .and_then(|v| {
            v.as_i64()
                .or_else(|| {
                    v.as_str().and_then(|s| {
                        s.parse::<i64>()
                            .inspect_err(|e| {
                                tracing::warn!("Failed to parse i64 from string '{}': {}", s, e)
                            })
                            .ok()
                    })
                })
                .or_else(|| {
                    v.as_str().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(s)
                            .map(|ts| ts.timestamp_millis())
                            .inspect_err(|e| {
                                tracing::warn!(
                                    "Failed to parse RFC3339 timestamp from string '{}': {}",
                                    s,
                                    e
                                )
                            })
                            .ok()
                    })
                })
        })
        .unwrap_or(0)
}

fn row_to_agent_task(row: &Value) -> Option<AgentTask> {
    row.as_object().map(|obj| {
        AgentTask {
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
            created_at: value_as_i64(row, "created_at"),
            completed_at: obj
                .get("completed_at")
                .and_then(|v| v.as_str())
                .map(String::from),
            agent_id: obj
                .get("agent_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            claimed_by: obj
                .get("claimed_by")
                .and_then(|v| v.as_str())
                .map(String::from),
            error: obj.get("error").and_then(|v| v.as_str()).map(String::from),
            due_at: obj.get("due_at").and_then(|v| v.as_str()).map(String::from),
            priority: obj.get("priority").and_then(|v| v.as_i64()),
            consumer: obj
                .get("consumer")
                .and_then(|v| v.as_str())
                .map(String::from),
            escalation_policy: obj
                .get("escalation_policy")
                .and_then(|v| v.as_str())
                .map(String::from),
            overdue: false,
            age_seconds: None,
        }
        .with_computed_metadata()
    })
}

fn task_record_id(task_id: &str) -> Result<String, StorageError> {
    let raw_id = task_id.strip_prefix("agent_tasks:").unwrap_or(task_id);
    crate::storage::sanitize_id(raw_id)?;
    Ok(format!("agent_tasks:{raw_id}"))
}

/// Store a new agent task. Returns task ID.
pub(super) async fn store_agent_task(
    storage: &SurrealHttpStorage,
    task: &AgentTask,
) -> Result<String, StorageError> {
    let agent_id = task
        .agent_id
        .as_ref()
        .map(|id| format!("'{}'", crate::storage::escape_surreal(id)))
        .unwrap_or_else(|| "null".to_string());
    let due_at = task
        .due_at
        .as_ref()
        .map(|value| format!("'{}'", crate::storage::escape_surreal(value)))
        .unwrap_or_else(|| "null".to_string());
    let priority = task
        .priority
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string());
    let consumer = task
        .consumer
        .as_ref()
        .map(|value| format!("'{}'", crate::storage::escape_surreal(value)))
        .unwrap_or_else(|| "null".to_string());
    let escalation_policy = task
        .escalation_policy
        .as_ref()
        .map(|value| format!("'{}'", crate::storage::escape_surreal(value)))
        .unwrap_or_else(|| "null".to_string());
    let query = format!(
        "INSERT INTO agent_tasks {{ kind: '{}', domain: '{}', content: '{}', status: 'pending', result: null, created_at: {}, completed_at: null, agent_id: {agent_id}, claimed_by: null, error: null, due_at: {due_at}, priority: {priority}, consumer: {consumer}, escalation_policy: {escalation_policy} }} RETURN id;",
        crate::storage::escape_surreal(&task.kind),
        crate::storage::escape_surreal(&task.domain),
        crate::storage::escape_surreal(&task.content),
        task.created_at
    );
    let rows = storage.query_one(&query).await?;
    rows.first()
        .and_then(extract_record_id)
        .ok_or_else(|| StorageError::QueryFailed("agent task insert returned no id".to_string()))
}

/// List pending tasks of a specific kind.
pub(super) async fn list_pending_agent_tasks(
    storage: &SurrealHttpStorage,
    kind: &str,
    domain: Option<&str>,
    limit: u32,
) -> Result<Vec<AgentTask>, StorageError> {
    let escaped_kind = crate::storage::escape_surreal(kind);
    let domain_filter = domain
        .map(crate::storage::escape_surreal)
        .map(|domain| format!(" AND `domain` = '{domain}'"))
        .unwrap_or_default();
    let query = format!(
        "SELECT * FROM agent_tasks WHERE `kind` = '{escaped_kind}'{domain_filter} AND `status` = 'pending' ORDER BY `created_at` ASC LIMIT {limit};"
    );
    let rows = storage.query_one(&query).await?;
    Ok(rows.iter().filter_map(row_to_agent_task).collect())
}

/// List overdue pending tasks across all kinds and domains.
pub(super) async fn list_overdue_agent_tasks(
    storage: &SurrealHttpStorage,
    limit: u32,
) -> Result<Vec<AgentTask>, StorageError> {
    let scan_limit = limit.max(1000);
    let query = format!(
        "SELECT * FROM agent_tasks WHERE `status` = 'pending' ORDER BY `created_at` ASC LIMIT {scan_limit};"
    );
    let rows = storage.query_one(&query).await?;
    Ok(rows
        .iter()
        .filter_map(row_to_agent_task)
        .filter(|task| task.overdue)
        .take(limit as usize)
        .collect())
}

/// List resolved tasks of a specific kind.
pub(super) async fn list_completed_agent_tasks(
    storage: &SurrealHttpStorage,
    kind: &str,
    domain: Option<&str>,
    limit: u32,
) -> Result<Vec<AgentTask>, StorageError> {
    let escaped_kind = crate::storage::escape_surreal(kind);
    let domain_filter = domain
        .map(crate::storage::escape_surreal)
        .map(|domain| format!(" AND `domain` = '{domain}'"))
        .unwrap_or_default();
    let query = format!(
        "SELECT * FROM agent_tasks WHERE `kind` = '{escaped_kind}'{domain_filter} AND (`status` = 'completed' OR `status` = 'failed') ORDER BY `completed_at` DESC LIMIT {limit};"
    );
    let rows = storage.query_one(&query).await?;
    Ok(rows.iter().filter_map(row_to_agent_task).collect())
}

/// Get a single agent task by ID.
pub(super) async fn get_agent_task(
    storage: &SurrealHttpStorage,
    task_id: &str,
) -> Result<Option<AgentTask>, StorageError> {
    let record_id = task_record_id(task_id)?;
    let query = format!("SELECT * FROM agent_tasks WHERE id = {record_id};");
    let rows = storage.query_one(&query).await?;
    Ok(rows.first().and_then(row_to_agent_task))
}

/// Atomically claim task for processing.
pub(super) async fn claim_agent_task(
    storage: &SurrealHttpStorage,
    task_id: &str,
    agent_id: &str,
) -> Result<bool, StorageError> {
    let record_id = task_record_id(task_id)?;
    let escaped_agent_id = crate::storage::escape_surreal(agent_id);
    let query = format!(
        "UPDATE agent_tasks SET status = 'processing', claimed_by = '{escaped_agent_id}' WHERE id = {record_id} AND status = 'pending';"
    );
    let rows = storage.query_one(&query).await?;
    Ok(!rows.is_empty())
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
    let now = chrono::Utc::now().timestamp_millis();
    let result_str = result
        .map(|r| format!("'{}'", crate::storage::escape_surreal(r)))
        .unwrap_or_else(|| "null".to_string());
    let error_str = error
        .map(|e| format!("'{}'", crate::storage::escape_surreal(e)))
        .unwrap_or_else(|| "null".to_string());

    let record_id = task_record_id(task_id)?;
    let query = format!(
        "UPDATE agent_tasks SET status = '{status}', result = {result_str}, error = {error_str}, completed_at = '{now}' WHERE id = {record_id};"
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
        let now_string = now.to_string();
        let completed_at = task.completed_at.as_deref().unwrap_or(&now_string);
        let _ = crystals::observe_crystal(
            storage,
            &task.id,
            obs_content,
            &task.domain,
            obs_score,
            completed_at,
            1,
            &task.id,
            &task.status,
        )
        .await;

        use crate::domain::proof::Provable;
        let poh = task.to_proof_of_history();
        let _ = super::proof_of_history::store_proof_of_history(storage, &poh).await;
    }

    Ok(())
}
