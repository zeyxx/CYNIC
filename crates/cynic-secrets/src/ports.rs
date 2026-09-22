//! Abstract ports for secret management.
//!
//! These traits define the interface between pure domain logic and concrete
//! secret backends. They must remain I/O-free — all side effects live in the
//! adapter implementations under `src/adapters/`.
//!
//! Authority: AGENTS.md §"Zero-I/O Domain Core".

use crate::{SecretError, SecretReference, SecretStatus};

/// Read-only port: resolve a `SecretReference` to a secret value.
///
/// Implementations must NEVER log the resolved value. Callers should
/// drop the value immediately after use (or wrap in a `Secret<String>`
/// that zeroizes on drop).
pub trait SecretProviderPort {
    /// Resolve a secret reference to its value. Returns `None` if
    /// the backend cannot find the secret (callers may chain providers).
    fn resolve(&self, reference: &SecretReference) -> Option<String>;

    /// Read-back: metadata about the secret's presence and posture.
    /// Default impl derives status from `resolve()` — backends can override
    /// for richer metadata (last_rotated, entropy estimation, etc.).
    fn status(&self, reference: &SecretReference) -> SecretStatus {
        SecretStatus {
            id: reference.id.clone(),
            class: reference.class,
            provider: reference.provider,
            locator: reference.locator.clone(),
            present: self.resolve(reference).is_some(),
            last_rotated_secs: None,
        }
    }
}

/// Write port: create, rotate, delete, and enumerate secrets.
///
/// This is a SUPERSET of `SecretProviderPort` — write-capable providers
/// implement both. Read-only providers (e.g. env) implement only the
/// `SecretProviderPort` read port.
pub trait SecretStorePort: SecretProviderPort {
    /// Store a secret value for the given reference.
    fn store(&self, spec: &SecretReference, value: &str) -> Result<(), SecretError>;

    /// Rotate (overwrite) a secret value.
    fn rotate(&self, spec: &SecretReference, new_value: &str) -> Result<(), SecretError>;

    /// Delete a secret by reference.
    fn delete(&self, spec: &SecretReference) -> Result<(), SecretError>;

    /// Enumerate all secret entries (metadata only, never values).
    fn list(&self) -> Vec<crate::SecretEntry>;
}
