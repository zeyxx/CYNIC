use cynic_protocol::{WorkloadIdentityDesiredState, WorkloadIdentityObservation};

/// Portable workload-identity boundary. Providers such as SPIFFE, a local
/// machine signer, or a cloud workload identity adapter implement this port;
/// callers never inspect provider credential material.
pub trait WorkloadIdentityPort: Send + Sync {
    fn observe(
        &self,
        desired: &WorkloadIdentityDesiredState,
        now_unix_seconds: u64,
    ) -> WorkloadIdentityObservation;

    fn enroll(&self, desired: &WorkloadIdentityDesiredState) -> Result<(), String>;

    fn renew(&self, desired: &WorkloadIdentityDesiredState) -> Result<(), String>;
}
