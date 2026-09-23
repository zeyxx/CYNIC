use cynic_storage::StorageObservation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageProviderTarget {
    pub node: String,
    pub vmid: String,
    pub volume_id: String,
    pub disk_key: String,
}

pub trait StorageProviderPort {
    async fn observe_volume(
        &self,
        target: &StorageProviderTarget,
    ) -> Result<StorageObservation, String>;
}
