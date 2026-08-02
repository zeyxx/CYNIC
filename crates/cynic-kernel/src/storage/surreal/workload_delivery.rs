use cynic_protocol::{
    AgentWorkloadDelivery, AgentWorkloadDeliveryStatus, validate_agent_workload_delivery,
};

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
        "SELECT record_json FROM agent_workload_delivery WHERE node_id = '{}' AND status = 'pending' ORDER BY submitted_at_ms ASC;",
        escape_surreal(node_id)
    );
    let rows = storage.query_one(&sql).await?;
    rows.iter().map(parse_row).collect()
}

pub(super) async fn accept(
    storage: &SurrealHttpStorage,
    workload_id: &str,
    node_id: &str,
    record_digest: &str,
    accepted_at_ms: u64,
) -> Result<AgentWorkloadDelivery, StorageError> {
    let mut delivery = read(storage, workload_id)
        .await?
        .ok_or_else(|| StorageError::NotFound("workload delivery".to_string()))?;
    if delivery.target_node_id != node_id {
        return Err(StorageError::InvalidInput(
            "workload delivery target mismatch".to_string(),
        ));
    }
    if delivery.status == AgentWorkloadDeliveryStatus::Accepted {
        return (delivery.accepted_record_digest.as_deref() == Some(record_digest))
            .then_some(delivery)
            .ok_or_else(|| {
                StorageError::InvalidInput("workload delivery acceptance conflict".to_string())
            });
    }
    delivery.status = AgentWorkloadDeliveryStatus::Accepted;
    delivery.accepted_at_ms = Some(accepted_at_ms);
    delivery.accepted_record_digest = Some(record_digest.to_string());
    delivery
        .evidence_refs
        .push(format!("durable-workload-record:{record_digest}"));
    validate(&delivery)?;
    let json = encode(&delivery)?;
    let sql = format!(
        "UPDATE agent_workload_delivery:`{id}` SET status = 'accepted', accepted_at_ms = {accepted_at_ms}, record_json = '{json}' WHERE status = 'pending' RETURN AFTER;",
        id = sanitize_record_id(workload_id),
        accepted_at_ms = accepted_at_ms,
        json = escape_surreal(&json),
    );
    let rows = storage.query_one(&sql).await?;
    if rows.is_empty() {
        let concurrent = read(storage, workload_id).await?.ok_or_else(|| {
            StorageError::QueryFailed("workload delivery acceptance race".to_string())
        })?;
        if concurrent.status == AgentWorkloadDeliveryStatus::Accepted
            && concurrent.accepted_record_digest.as_deref() == Some(record_digest)
        {
            return Ok(concurrent);
        }
        return Err(StorageError::InvalidInput(
            "workload delivery acceptance conflict".to_string(),
        ));
    }
    let read_back = read(storage, workload_id)
        .await?
        .ok_or_else(|| StorageError::QueryFailed("workload acceptance read-back missing".into()))?;
    if read_back != delivery {
        return Err(StorageError::QueryFailed(
            "workload acceptance read-back mismatch".to_string(),
        ));
    }
    Ok(read_back)
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
