use crate::domain::storage_lifecycle::StorageObservation;

pub trait StorageLifecyclePort {
    async fn observe(&self, service: &str, health_url: &str) -> StorageObservation;
    fn restart(&self, service: &str) -> Result<(), String>;
}
