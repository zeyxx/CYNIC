//! Abstract ports for secret management.
//!
//! These traits define the interface between pure domain logic and concrete
//! secret backends. They must remain I/O-free — all side effects live in the
//! adapter implementations under `src/adapters/`.
//!
//! Authority: `cynic-secrets::ports` is a pure port declaration. The purity
//! rule is executed by `crates/cynic-secrets/tests/architecture_boundary.rs`,
//! which asserts the absence of process spawning and HTTP clients here —
//! AGENTS.md only restates it for humans.

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

/// Gatekeeper port: authorize secret use and execute a child process with the
/// secret injected, without ever returning the value to the caller.
///
/// This is the ONLY path by which an agent-facing surface may cause a secret to
/// be used. Implementations must:
/// 1. Evaluate the request against a declared policy via the pure
///    `decide_secret_use` function — never their own ad-hoc logic.
/// 2. Append one `SecretUseAuditEntry` to the audit sink for EVERY request,
///    allowed or denied.
/// 3. On `Allowed`, resolve the material internally and inject it into a
///    short-lived child process; the value must never cross this boundary.
pub trait SecretUseGatePort {
    /// Authorize a secret-use request. Returns a handle on success, a reason
    /// on refusal. Never returns secret material.
    fn authorize(
        &self,
        request: &crate::secret_use::SecretUseRequest,
    ) -> crate::protocol::SecretUseOutcome;

    /// Execute `argv` in a child process with the authorized secret injected as
    /// the environment variable named by `env_var`. The value is resolved
    /// internally and never returned.
    ///
    /// `handle` must be one previously returned by `authorize`; an expired or
    /// unknown handle fails closed.
    fn execute_with_secret(
        &self,
        handle: &crate::protocol::SecretUseHandle,
        env_var: &str,
        argv: &[String],
    ) -> Result<crate::protocol::SecretUseExecutionReport, SecretError>;
}
