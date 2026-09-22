use cynic_workload::{
    AgentWorkloadCompletionPublication, AgentWorkloadDelivery, AgentWorkloadDeliveryAcceptance,
    AgentWorkloadDeliveryStatus, accept_agent_workload_delivery, agent_workload_delivery_available,
    complete_agent_workload_delivery, validate_agent_workload_delivery,
};
use std::time::{SystemTime, UNIX_EPOCH};

use super::sanitize_record_id;
use crate::domain::storage::StorageError;
use crate::storage::{SurrealHttpStorage, escape_surreal};

pub(super) async fn submit(
    storage: &SurrealHttpStorage,
    delivery: &AgentWorkloadDelivery,
) -> Result<AgentWorkloadDelivery, StorageError> {
    validate(delivery)?;
    if let Some(existing) = read(storage, &delivery.envelope.workload_id).await? {
        return same_submission(&existing, delivery)
            .then_some(existing)
            .ok_or_else(|| StorageError::InvalidInput("workload delivery conflict".to_string()));
    }
    let json = encode(delivery)?;
    let sql = format!(
        "CREATE ONLY agent_workload_delivery:`{id}` SET workload_id = '{workload_id}', idempotency_key = '{idempotency_key}', node_id = '{node_id}', submitted_by = '{submitted_by}', status = 'pending', submitted_at_ms = {submitted_at_ms}, record_json = '{json}' RETURN AFTER;",
        id = sanitize_record_id(&delivery.envelope.workload_id),
        workload_id = escape_surreal(&delivery.envelope.workload_id),
        idempotency_key = escape_surreal(&delivery.envelope.idempotency_key),
        node_id = escape_surreal(&delivery.target_node_id),
        submitted_by = escape_surreal(&delivery.submitted_by),
        submitted_at_ms = delivery.submitted_at_ms,
        json = escape_surreal(&json),
    );
    match storage.query_one(&sql).await {
        Ok(rows) if !rows.is_empty() => read(storage, &delivery.envelope.workload_id)
            .await?
            .ok_or_else(|| StorageError::QueryFailed("workload delivery read-back missing".into())),
        _ => match read(storage, &delivery.envelope.workload_id).await? {
            Some(existing) if same_submission(&existing, delivery) => Ok(existing),
            _ => Err(StorageError::QueryFailed(
                "workload delivery publish failed".to_string(),
            )),
        },
    }
}

pub(super) async fn read(
    storage: &SurrealHttpStorage,
    workload_id: &str,
) -> Result<Option<AgentWorkloadDelivery>, StorageError> {
    let sql = format!(
        "SELECT record_json FROM agent_workload_delivery:`{}`;",
        sanitize_record_id(workload_id)
    );
    let rows = storage.query_one(&sql).await?;
    rows.first().map(parse_row).transpose()
}

pub(super) async fn pending(
    storage: &SurrealHttpStorage,
    node_id: &str,
) -> Result<Vec<AgentWorkloadDelivery>, StorageError> {
    let sql = format!(
        "SELECT record_json FROM agent_workload_delivery WHERE node_id = '{}' AND (status = 'pending' OR status = 'accepted') ORDER BY submitted_at_ms ASC;",
        escape_surreal(node_id)
    );
    let rows = storage.query_one(&sql).await?;
    let now_ms = now_unix_ms();
    rows.iter()
        .map(parse_row)
        .filter_map(|delivery| match delivery {
            Ok(delivery) if agent_workload_delivery_available(&delivery, now_ms) => {
                Some(Ok(delivery))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

pub(super) async fn accept(
    storage: &SurrealHttpStorage,
    workload_id: &str,
    node_id: &str,
    acceptance: &AgentWorkloadDeliveryAcceptance,
) -> Result<AgentWorkloadDelivery, StorageError> {
    let delivery = read(storage, workload_id)
        .await?
        .ok_or_else(|| StorageError::NotFound("workload delivery".to_string()))?;
    if delivery.target_node_id != node_id {
        return Err(StorageError::InvalidInput(
            "workload delivery target mismatch".to_string(),
        ));
    }
    let accepted = accept_agent_workload_delivery(&delivery, acceptance)
        .map_err(|violations| StorageError::InvalidInput(violations.join(",")))?;
    let old_json = encode(&delivery)?;
    let json = encode(&accepted)?;
    let sql = format!(
        "UPDATE agent_workload_delivery:`{id}` SET status = 'accepted', accepted_at_ms = {accepted_at_ms}, record_json = '{json}' WHERE record_json = '{old_json}' RETURN AFTER;",
        id = sanitize_record_id(workload_id),
        accepted_at_ms = accepted.accepted_at_ms.unwrap_or_default(),
        json = escape_surreal(&json),
        old_json = escape_surreal(&old_json),
    );
    let rows = storage.query_one(&sql).await?;
    if rows.is_empty() {
        let concurrent = read(storage, workload_id).await?.ok_or_else(|| {
            StorageError::QueryFailed("workload delivery acceptance race".to_string())
        })?;
        if concurrent == accepted {
            return Ok(concurrent);
        }
        return Err(StorageError::InvalidInput(
            "workload delivery acceptance conflict".to_string(),
        ));
    }
    let read_back = read(storage, workload_id)
        .await?
        .ok_or_else(|| StorageError::QueryFailed("workload acceptance read-back missing".into()))?;
    if read_back != accepted {
        return Err(StorageError::QueryFailed(
            "workload acceptance read-back mismatch".to_string(),
        ));
    }
    Ok(read_back)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

pub(super) async fn complete(
    storage: &SurrealHttpStorage,
    publication: &AgentWorkloadCompletionPublication,
) -> Result<AgentWorkloadDelivery, StorageError> {
    let observed = read(storage, &publication.workload_id)
        .await?
        .ok_or_else(|| StorageError::NotFound("workload delivery".to_string()))?;
    if observed.status == AgentWorkloadDeliveryStatus::Terminal {
        let accepted = accepted_projection(&observed, &publication.durable_record.record_digest);
        let expected = complete_agent_workload_delivery(&accepted, publication)
            .map_err(|violations| StorageError::InvalidInput(violations.join(",")))?;
        return (expected == observed).then_some(observed).ok_or_else(|| {
            StorageError::InvalidInput("workload completion publication conflict".to_string())
        });
    }
    let terminal = complete_agent_workload_delivery(&observed, publication)
        .map_err(|violations| StorageError::InvalidInput(violations.join(",")))?;
    let observed_json = encode(&observed)?;
    let json = encode(&terminal)?;
    let sql = format!(
        "UPDATE agent_workload_delivery:`{id}` SET status = 'terminal', terminal_at_ms = {terminal_at_ms}, record_json = '{json}' WHERE record_json = '{observed_json}' RETURN AFTER;",
        id = sanitize_record_id(&publication.workload_id),
        terminal_at_ms = publication.published_at_ms,
        json = escape_surreal(&json),
        observed_json = escape_surreal(&observed_json),
    );
    let rows = storage.query_one(&sql).await?;
    if rows.is_empty() {
        let concurrent = read(storage, &publication.workload_id)
            .await?
            .ok_or_else(|| StorageError::QueryFailed("workload completion race".to_string()))?;
        let accepted = accepted_projection(&concurrent, &publication.durable_record.record_digest);
        let expected = complete_agent_workload_delivery(&accepted, publication)
            .map_err(|violations| StorageError::InvalidInput(violations.join(",")))?;
        return (concurrent == expected)
            .then_some(concurrent)
            .ok_or_else(|| {
                StorageError::InvalidInput("workload completion publication conflict".to_string())
            });
    }
    let read_back = read(storage, &publication.workload_id)
        .await?
        .ok_or_else(|| StorageError::QueryFailed("workload completion read-back missing".into()))?;
    if read_back != terminal {
        return Err(StorageError::QueryFailed(
            "workload completion read-back mismatch".to_string(),
        ));
    }
    Ok(read_back)
}

fn accepted_projection(
    terminal: &AgentWorkloadDelivery,
    record_digest: &str,
) -> AgentWorkloadDelivery {
    let mut accepted = terminal.clone();
    accepted.status = AgentWorkloadDeliveryStatus::Accepted;
    accepted.terminal_at_ms = None;
    accepted.terminal_record_digest = None;
    accepted.terminal_record = None;
    accepted.completion = None;
    accepted.completion_evidence.clear();
    accepted
        .evidence_refs
        .retain(|reference| reference != &format!("durable-workload-completion:{record_digest}"));
    accepted
}

fn validate(delivery: &AgentWorkloadDelivery) -> Result<(), StorageError> {
    let violations = validate_agent_workload_delivery(delivery);
    if violations.is_empty() {
        Ok(())
    } else {
        Err(StorageError::InvalidInput(violations.join(",")))
    }
}

fn encode(delivery: &AgentWorkloadDelivery) -> Result<String, StorageError> {
    serde_json::to_string(delivery)
        .map_err(|error| StorageError::InvalidInput(format!("encode workload delivery: {error}")))
}

fn parse_row(row: &serde_json::Value) -> Result<AgentWorkloadDelivery, StorageError> {
    let json = row["record_json"]
        .as_str()
        .ok_or_else(|| StorageError::QueryFailed("workload delivery record missing".to_string()))?;
    let delivery: AgentWorkloadDelivery = serde_json::from_str(json)
        .map_err(|error| StorageError::QueryFailed(format!("decode workload delivery: {error}")))?;
    validate(&delivery)?;
    Ok(delivery)
}

fn same_submission(left: &AgentWorkloadDelivery, right: &AgentWorkloadDelivery) -> bool {
    left.envelope == right.envelope
        && left.target_node_id == right.target_node_id
        && left.submitted_by == right.submitted_by
}
