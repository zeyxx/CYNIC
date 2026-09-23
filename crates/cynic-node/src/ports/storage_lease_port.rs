use cynic_protocol::LeaseGuard;
use cynic_storage::StorageLeaseGrant;

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
