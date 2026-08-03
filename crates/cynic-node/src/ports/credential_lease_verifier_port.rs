use cynic_protocol::CredentialLeaseMetadata;

/// Verifies the authority proof attached to a credential lease. The provider
/// adapter must not promote metadata based on shape or digest alone.
pub trait CredentialLeaseVerifierPort: Send + Sync {
    fn verify(
        &self,
        metadata: &CredentialLeaseMetadata,
        expected_provider: &str,
        expected_material_digest: Option<&str>,
    ) -> Result<Vec<String>, String>;
}
