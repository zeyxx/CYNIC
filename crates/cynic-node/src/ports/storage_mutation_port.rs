#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageMutationRequest {
    pub resource_id: String,
    pub node: String,
    pub vmid: String,
    pub disk_key: String,
    pub target_bytes: u64,
    pub observed_provider_bytes: u64,
    pub approved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageMutationResult {
    pub accepted: bool,
    pub operation_ref: String,
    pub evidence_refs: Vec<String>,
}

pub trait StorageMutationPort {
    async fn resize_volume(
        &self,
        request: &StorageMutationRequest,
    ) -> Result<StorageMutationResult, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageOperationReadBack {
    pub terminal: bool,
    pub succeeded: bool,
    pub evidence_refs: Vec<String>,
}

pub trait StorageOperationReadBackPort {
    async fn observe_operation(
        &self,
        node: &str,
        operation_ref: &str,
    ) -> Result<StorageOperationReadBack, String>;
}
