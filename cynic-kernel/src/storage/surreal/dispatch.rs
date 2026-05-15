//! Multi-cortex dispatch tracking in SurrealDB.
//! Coordinates work across Claude Code, Gemini CLI, and Hermes agents.

use super::SurrealHttpStorage;
use crate::domain::storage::{AgentDispatch, StorageError};
use sha2::{Digest, Sha256};

fn row_to_agent_dispatch(row: &serde_json::Value) -> Option<AgentDispatch> {
    row.as_object().map(|obj| AgentDispatch {
        id: obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        scope: obj
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        zone: obj
            .get("zone")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        claimed_by: obj
            .get("claimed_by")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        branch: obj
            .get("branch")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        status: obj
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("CLAIMED")
            .to_string(),
        created_at: obj
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        completed_at: obj
            .get("completed_at")
            .and_then(|v| v.as_str())
            .map(String::from),
        pr_number: obj
            .get("pr_number")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        hash: obj
            .get("hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        prev_hash: obj
            .get("prev_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Compute SHA256 hash of dispatch record (for chain integrity).
/// Hashes: scope, zone, claimed_by, branch, status (NOT id, created_at, hash fields).
fn compute_dispatch_hash(dispatch: &AgentDispatch) -> String {
    let data = format!(
        "{}|{}|{}|{}|{}",
        dispatch.scope, dispatch.zone, dispatch.claimed_by, dispatch.branch, dispatch.status
    );
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    // Convert bytes to hex string
    result.iter().map(|b| format!("{b:02x}")).collect()
}

/// Store a new dispatch record. Returns dispatch ID.
/// Automatically computes hash and looks up prev_hash from last dispatch for scope.
pub(super) async fn store_agent_dispatch(
    storage: &SurrealHttpStorage,
    dispatch: &AgentDispatch,
) -> Result<String, StorageError> {
    let hash = compute_dispatch_hash(dispatch);

    // Look up previous dispatch for same scope to chain hashes
    let prev_hash = get_last_completed_dispatch_for_scope(storage, &dispatch.scope)
        .await
        .ok()
        .flatten()
        .map(|d| d.hash)
        .unwrap_or_default();

    let query = format!(
        "INSERT INTO agent_dispatch {{ scope: '{}', zone: '{}', claimed_by: '{}', branch: '{}', status: 'CLAIMED', \
         created_at: '{}', hash: '{}', prev_hash: '{}' }} RETURN id;",
        crate::storage::escape_surreal(&dispatch.scope),
        crate::storage::escape_surreal(&dispatch.zone),
        crate::storage::escape_surreal(&dispatch.claimed_by),
        crate::storage::escape_surreal(&dispatch.branch),
        dispatch.created_at,
        hash,
        prev_hash,
    );

    storage.query_one(&query).await?;
    Ok(dispatch.id.clone())
}

/// Get active dispatch for a specific scope (where status != "COMPLETED").
pub(super) async fn get_active_dispatch_for_scope(
    storage: &SurrealHttpStorage,
    scope: &str,
) -> Result<Option<AgentDispatch>, StorageError> {
    let escaped_scope = crate::storage::escape_surreal(scope);
    let query = format!(
        "SELECT * FROM agent_dispatch WHERE scope = '{escaped_scope}' AND status != 'COMPLETED' \
         ORDER BY created_at DESC LIMIT 1;"
    );
    let rows = storage.query_one(&query).await?;
    Ok(rows.first().and_then(row_to_agent_dispatch))
}

/// Get dispatch by ID.
pub(super) async fn get_dispatch(
    storage: &SurrealHttpStorage,
    dispatch_id: &str,
) -> Result<Option<AgentDispatch>, StorageError> {
    let query = format!("SELECT * FROM agent_dispatch WHERE id = {dispatch_id};");
    let rows = storage.query_one(&query).await?;
    Ok(rows.first().and_then(row_to_agent_dispatch))
}

/// Get all active dispatches for a specific agent (claimed_by).
pub(super) async fn get_active_dispatches_for_agent(
    storage: &SurrealHttpStorage,
    agent_id: &str,
) -> Result<Vec<AgentDispatch>, StorageError> {
    let escaped_agent = crate::storage::escape_surreal(agent_id);
    let query = format!(
        "SELECT * FROM agent_dispatch WHERE claimed_by = '{escaped_agent}' AND status != 'COMPLETED' \
         ORDER BY created_at DESC;"
    );
    let rows = storage.query_one(&query).await?;
    Ok(rows.iter().filter_map(row_to_agent_dispatch).collect())
}

/// Get last completed dispatch for a scope (for chain prev_hash).
async fn get_last_completed_dispatch_for_scope(
    storage: &SurrealHttpStorage,
    scope: &str,
) -> Result<Option<AgentDispatch>, StorageError> {
    let escaped_scope = crate::storage::escape_surreal(scope);
    let query = format!(
        "SELECT * FROM agent_dispatch WHERE scope = '{escaped_scope}' AND status = 'COMPLETED' \
         ORDER BY completed_at DESC LIMIT 1;"
    );
    let rows = storage.query_one(&query).await?;
    Ok(rows.first().and_then(row_to_agent_dispatch))
}

/// Update dispatch status. Recomputes hash after status change.
pub(super) async fn update_dispatch_status(
    storage: &SurrealHttpStorage,
    dispatch_id: &str,
    new_status: &str,
) -> Result<(), StorageError> {
    let now = chrono::Utc::now().to_rfc3339();

    let completed_at = if new_status == "COMPLETED" {
        format!("completed_at: '{now}',")
    } else {
        String::new()
    };

    // Fetch current dispatch to recompute hash
    if let Ok(Some(mut dispatch)) = get_dispatch(storage, dispatch_id).await {
        dispatch.status = new_status.to_string();
        let new_hash = compute_dispatch_hash(&dispatch);

        let query = format!(
            "UPDATE agent_dispatch SET status = '{}', hash = '{new_hash}', {completed_at} updated_at = '{now}' WHERE id = {dispatch_id};",
            crate::storage::escape_surreal(new_status),
        );
        storage.query_one(&query).await?;
    }

    Ok(())
}

/// Update dispatch with PR number and transition to PROPOSED.
pub(super) async fn update_dispatch_pr(
    storage: &SurrealHttpStorage,
    dispatch_id: &str,
    pr_number: u32,
) -> Result<(), StorageError> {
    let query = format!(
        "UPDATE agent_dispatch SET status = 'PROPOSED', pr_number = {pr_number} WHERE id = {dispatch_id};"
    );
    storage.query_one(&query).await?;
    Ok(())
}
