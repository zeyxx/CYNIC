use cynic_protocol::LeaseGuard;

use super::controller::{StorageControllerRecord, StorageLeaseGrant};
use super::lifecycle::StorageObservation;

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

pub trait StorageLeasePort: Send + Sync {
    fn acquire(
        &self,
        resource_id: &str,
        agent_id: &str,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<StorageLeaseGrant, String>;
    fn renew(
        &self,
        guard: &LeaseGuard,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<StorageLeaseGrant, String>;
    fn release(&self, guard: &LeaseGuard, now_ms: u64) -> Result<(), String>;
}

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

/// Compatibility alias for the canonical read-back port.
pub trait StorageOperationStatus: StorageOperationReadBackPort {}

impl<T: StorageOperationReadBackPort> StorageOperationStatus for T {}

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
