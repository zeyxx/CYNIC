use cynic_protocol::{CredentialLeaseDesiredState, CredentialLeaseObservation};

/// Provider-neutral credential lease boundary. Implementations may use a
/// secret store, systemd credentials, Vault, or a provider-native broker.
/// Material is never returned through this observation port.
pub trait CredentialBrokerPort: Send + Sync {
    fn observe(
        &self,
        desired: &CredentialLeaseDesiredState,
        now_unix_seconds: u64,
    ) -> CredentialLeaseObservation;

    fn issue(&self, desired: &CredentialLeaseDesiredState) -> Result<(), String>;
    fn renew(&self, desired: &CredentialLeaseDesiredState) -> Result<(), String>;
    fn revoke(&self, desired: &CredentialLeaseDesiredState, generation: u64) -> Result<(), String>;
}
