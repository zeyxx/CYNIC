//! Pure credential lease verifier port.
//!
//! The provider adapter must not promote lease metadata based on shape or
//! digest alone: authority is proven by an injected verifier. Lives in
//! `cynic-protocol` (L0) so any L1 provider crate can consume it without
//! inversion.

use crate::CredentialLeaseMetadata;

/// Verifies the authority proof attached to a credential lease.
pub trait CredentialLeaseVerifierPort: Send + Sync {
    fn verify(
        &self,
        metadata: &CredentialLeaseMetadata,
        expected_provider: &str,
        expected_material_digest: Option<&str>,
    ) -> Result<Vec<String>, String>;
}
