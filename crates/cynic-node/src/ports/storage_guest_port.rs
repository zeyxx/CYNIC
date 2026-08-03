use cynic_protocol::StorageObservation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageGuestTarget {
    pub volume_id: String,
    pub service: String,
    pub mount_path: String,
    pub health_endpoint: String,
}

/// Read-only guest substrate boundary.  A transport (local, LXC, or another
/// provider) supplies argv execution; this port owns neither SSH nor a shell.
pub trait StorageGuestObservationPort {
    async fn observe_guest(
        &self,
        target: &StorageGuestTarget,
    ) -> Result<StorageObservation, String>;
}
