use cynic_storage::StorageControllerRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageControllerStoreError {
    pub code: String,
}

pub trait StorageControllerStorePort: Send + Sync {
    fn read_latest(
        &self,
        resource_id: &str,
    ) -> Result<Option<StorageControllerRecord>, StorageControllerStoreError>;
    fn append(
        &self,
        expected_generation: Option<u64>,
        record: &StorageControllerRecord,
    ) -> Result<StorageControllerRecord, StorageControllerStoreError>;
}
