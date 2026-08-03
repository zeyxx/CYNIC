use cynic_protocol::{CredentialLeaseDesiredState, CredentialLeaseMetadata};

use super::credential_material_sink_port::CredentialMaterialSinkPort;

/// Separate provisioning authority for provider credentials. A runtime
/// credential must never mint or revoke its own successor.
pub trait CredentialRotationAuthorityPort: Send + Sync {
    fn issue(
        &self,
        desired: &CredentialLeaseDesiredState,
        next_generation: u64,
        sink: &dyn CredentialMaterialSinkPort,
    ) -> Result<CredentialLeaseMetadata, String>;

    fn revoke(
        &self,
        desired: &CredentialLeaseDesiredState,
        generation: u64,
    ) -> Result<Vec<String>, String>;
}
