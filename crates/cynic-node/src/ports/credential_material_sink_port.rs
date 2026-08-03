/// Provider-neutral sink for newly issued credential material.
///
/// Rotation authorities receive a sink instead of returning secret bytes. The
/// sink owns atomic persistence, permissions, and read-back; only the
/// resulting material digest may flow into lease metadata.
pub trait CredentialMaterialSinkPort: Send + Sync {
    fn store(&self, secret_ref: &str, material: &[u8]) -> Result<String, String>;
}
