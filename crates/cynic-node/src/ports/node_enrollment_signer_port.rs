//! Node enrollment signer port — capability to sign and verify enrollment grants.
//!
//! Canonical home since 2026-09-28. Extracted verbatim from
//! `cynic-kernel::domain::node_enrollment` (pure, zero I/O): the kernel must
//! not own node-domain ports (Session 5 inversion fix). The file-backed
//! implementation lives in `crate::adapters::local_node_enrollment_signer`.

/// Boundary that signs enrollment payloads and verifies enrollment signatures.
pub trait NodeEnrollmentSignerPort: Send + Sync + std::fmt::Debug {
    /// Key identifier of the form `sha256:<hex>` (SHA-256 of the verifying key).
    fn key_id(&self) -> String;
    /// Sign `payload`, returning the lowercase-hex 64-byte signature.
    fn sign(&self, payload: &[u8]) -> Result<String, String>;
    /// Verify a lowercase-hex signature over `payload`.
    fn verify(&self, payload: &[u8], signature_hex: &str) -> Result<(), String>;
}
