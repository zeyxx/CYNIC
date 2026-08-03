use cynic_protocol::{
    STORAGE_CONTROLLER_RECORD_SCHEMA, StorageControllerRecord, StorageMetadata, StorageObservation,
    StoragePlan, reconcile_storage_status,
};

use crate::adapters::local_storage_controller_store::seal_storage_controller_record;
use crate::ports::storage_controller_store_port::{
    StorageControllerStoreError, StorageControllerStorePort,
};

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
