use cynic_protocol::{
    STORAGE_CONTROLLER_RECORD_SCHEMA, StorageControllerRecord, StorageMetadata, StorageObservation,
    StoragePlan, reconcile_storage_status,
};

use crate::adapters::local_storage_controller_store::seal_storage_controller_record;
use crate::ports::storage_controller_store_port::{
    StorageControllerStoreError, StorageControllerStorePort,
};
use crate::ports::storage_mutation_port::{StorageMutationPort, StorageMutationRequest};
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

pub async fn apply_pending_storage_operation(
    store: &dyn StorageControllerStorePort,
    mutation: &dyn StorageMutationPort,
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
