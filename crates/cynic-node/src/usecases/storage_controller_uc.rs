use cynic_protocol::{
    STORAGE_CONTROLLER_RECORD_SCHEMA, StorageControllerRecord, StorageMetadata, StorageObservation,
    StoragePlan, reconcile_storage_status,
};

use crate::adapters::local_storage_controller_store::seal_storage_controller_record;
use crate::ports::storage_controller_store_port::{
    StorageControllerStoreError, StorageControllerStorePort,
};
use crate::ports::storage_mutation_port::{
    StorageMutationPort, StorageMutationRequest, StorageOperationReadBackPort,
};
use cynic_protocol::{StorageOperationEvent, advance_storage_operation};

pub fn reconcile_storage_controller(
    store: &dyn StorageControllerStorePort,
    metadata: StorageMetadata,
    plan: &StoragePlan,
    observed: &StorageObservation,
    now_ms: u64,
) -> Result<StorageControllerRecord, StorageControllerStoreError> {
    if metadata
        .lease_expires_at_ms
        .is_none_or(|expires| expires <= now_ms)
        || metadata.lease_fencing_token.is_none_or(|token| token == 0)
    {
        return Err(StorageControllerStoreError {
            code: "storage_controller_lease_missing_or_expired".to_string(),
        });
    }
    let (status, operation) = reconcile_storage_status(&metadata, plan, observed, now_ms);
    let previous = store.read_latest(&metadata.id)?;
    let mut record = StorageControllerRecord {
        schema: STORAGE_CONTROLLER_RECORD_SCHEMA.to_string(),
        metadata,
        status,
        operation,
        record_digest: String::new(),
        previous_record_digest: previous.as_ref().map(|item| item.record_digest.clone()),
        committed_at_ms: now_ms,
    };
    seal_storage_controller_record(&mut record)
        .map_err(|code| StorageControllerStoreError { code })?;
    store.append(
        previous.as_ref().map(|item| item.metadata.generation),
        &record,
    )
}

pub async fn apply_pending_storage_operation<M: StorageMutationPort + ?Sized>(
    store: &dyn StorageControllerStorePort,
    mutation: &M,
    request: &StorageMutationRequest,
    now_ms: u64,
    accepted_at_ms: u64,
) -> Result<StorageControllerRecord, StorageControllerStoreError> {
    let current =
        store
            .read_latest(&request.resource_id)?
            .ok_or_else(|| StorageControllerStoreError {
                code: "storage_controller_record_missing".to_string(),
            })?;
    let operation = current
        .operation
        .as_ref()
        .filter(|operation| operation.phase == cynic_protocol::StorageOperationPhase::Pending)
        .ok_or_else(|| StorageControllerStoreError {
            code: "storage_operation_not_pending".to_string(),
        })?;
    let mut applying = current.clone();
    applying.metadata.generation = current.metadata.generation.saturating_add(1);
    applying.metadata.observed_generation = None;
    applying.operation = Some(advance_storage_operation(
        operation,
        StorageOperationEvent::StartApply,
        &[],
        now_ms,
    ));
    if let Some(operation) = applying.operation.as_mut() {
        operation.generation = applying.metadata.generation;
    }
    applying.previous_record_digest = Some(current.record_digest.clone());
    applying.committed_at_ms = now_ms;
    seal_storage_controller_record(&mut applying)
        .map_err(|code| StorageControllerStoreError { code })?;
    store.append(Some(current.metadata.generation), &applying)?;

    let accepted = mutation
        .resize_volume(request)
        .await
        .map_err(|code| StorageControllerStoreError { code })?;
    let current =
        store
            .read_latest(&request.resource_id)?
            .ok_or_else(|| StorageControllerStoreError {
                code: "storage_controller_record_missing_after_apply".to_string(),
            })?;
    let operation = current
        .operation
        .as_ref()
        .ok_or_else(|| StorageControllerStoreError {
            code: "storage_operation_missing_after_apply".to_string(),
        })?;
    let mut verifying = current.clone();
    verifying.metadata.generation = current.metadata.generation.saturating_add(1);
    verifying.operation = Some(advance_storage_operation(
        operation,
        StorageOperationEvent::ApplyAccepted,
        &accepted.evidence_refs,
        accepted_at_ms,
    ));
    if let Some(operation) = verifying.operation.as_mut() {
        operation.generation = verifying.metadata.generation;
        operation.id = accepted.operation_ref;
    }
    verifying.previous_record_digest = Some(current.record_digest.clone());
    verifying.committed_at_ms = accepted_at_ms;
    seal_storage_controller_record(&mut verifying)
        .map_err(|code| StorageControllerStoreError { code })?;
    store.append(Some(current.metadata.generation), &verifying)
}

pub async fn verify_storage_operation<R: StorageOperationReadBackPort + ?Sized>(
    store: &dyn StorageControllerStorePort,
    read_back: &R,
    node: &str,
    resource_id: &str,
    guest_observation: &cynic_protocol::StorageObservation,
    now_ms: u64,
) -> Result<StorageControllerRecord, StorageControllerStoreError> {
    let current = store
        .read_latest(resource_id)?
        .ok_or_else(|| StorageControllerStoreError {
            code: "storage_controller_record_missing".to_string(),
        })?;
    let operation = current
        .operation
        .as_ref()
        .filter(|operation| operation.phase == cynic_protocol::StorageOperationPhase::Verifying)
        .ok_or_else(|| StorageControllerStoreError {
            code: "storage_operation_not_verifying".to_string(),
        })?;
    let task = read_back
        .observe_operation(node, &operation.id)
        .await
        .map_err(|code| StorageControllerStoreError { code })?;
    if !task.terminal {
        return Err(StorageControllerStoreError {
            code: "storage_operation_not_terminal".to_string(),
        });
    }
    let mut evidence = task.evidence_refs;
    evidence.extend(guest_observation.evidence_refs.iter().cloned());
    let mut next = current.clone();
    next.metadata.generation = current.metadata.generation.saturating_add(1);
    next.metadata.observed_generation = (task.succeeded
        && guest_observation.service_ready == Some(true)
        && guest_observation.unknowns.is_empty())
    .then_some(next.metadata.generation);
    next.status.provider_bytes = guest_observation
        .provider_bytes
        .or(next.status.provider_bytes);
    next.status.filesystem_bytes = guest_observation.filesystem_bytes;
    next.status.free_bytes = guest_observation.free_bytes;
    next.status.service_ready = guest_observation.service_ready;
    next.status.evidence_refs.extend(evidence.iter().cloned());
    next.status.unknowns = guest_observation.unknowns.clone();
    next.operation = Some(advance_storage_operation(
        operation,
        if task.succeeded
            && guest_observation.service_ready == Some(true)
            && guest_observation.unknowns.is_empty()
        {
            StorageOperationEvent::ReadBack
        } else {
            StorageOperationEvent::Fail
        },
        &evidence,
        now_ms,
    ));
    if let Some(operation) = next.operation.as_mut() {
        operation.generation = next.metadata.generation;
        if !task.succeeded {
            operation
                .blockers
                .push("storage_provider_task_failed".to_string());
        }
        if guest_observation.service_ready != Some(true) || !guest_observation.unknowns.is_empty() {
            operation
                .blockers
                .push("storage_guest_read_back_incomplete".to_string());
        }
    }
    next.previous_record_digest = Some(current.record_digest.clone());
    next.committed_at_ms = now_ms;
    seal_storage_controller_record(&mut next)
        .map_err(|code| StorageControllerStoreError { code })?;
    store.append(Some(current.metadata.generation), &next)
}
