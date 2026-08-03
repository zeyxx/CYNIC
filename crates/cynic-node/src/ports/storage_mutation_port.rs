#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageMutationRequest {
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
