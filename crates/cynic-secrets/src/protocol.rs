//! Pure protocol types for secret management.
//!
//! These types carry metadata about secrets — never the secret values
//! themselves. They are consumed by the ports layer and concrete adapters.
//!
//! Authority: `cynic-protocol::secret_use::{SecretUsePurpose, SecretUseRequest}`.
//! The classes and their permitted purposes are a Rust type, not prose.

use serde::{Deserialize, Serialize};

use crate::{SecretClass, SecretProviderKind, SecretReference};

/// Read-back status of a resolved secret reference.
///
/// Mirrors `SecretReference` fields plus runtime observability data.
/// Never contains the secret value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretStatus {
    pub id: String,
    pub class: SecretClass,
    pub provider: SecretProviderKind,
    pub locator: String,
    pub present: bool,
    pub last_rotated_secs: Option<u64>,
}

/// A single entry in the secret inventory (metadata only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretEntry {
    pub spec: SecretReference,
    pub created_at_secs: Option<u64>,
    pub rotated_at_secs: Option<u64>,
}

/// Errors from secret operations (never leaks the secret value).
#[derive(Debug, Clone, thiserror::Error)]
pub enum SecretError {
    #[error("credential not found for reference: {0}")]
    NotFound(String),

    #[error("credential resolution failed for '{0}': {1}")]
    ResolutionFailed(String, String),

    #[error("secret backend error: {0}")]
    Backend(String),

    #[error("encryption/decryption error: {0}")]
    Crypto(String),

    #[error("not implemented for provider: {0}")]
    Unsupported(String),
}

/// An opaque handle to an authorized secret use.
///
/// The handle names *which* secret was authorized and *when the grant expires*.
/// It is structurally incapable of carrying the secret value: there is no field
/// for it. Only the gatekeeper adapter that minted the handle can resolve the
/// underlying material, and it does so internally, never returning it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretUseHandle {
    /// The secret id the handle authorizes.
    pub secret_id: String,
    /// Opaque, non-guessable token identifying this authorization instance.
    pub token: String,
    /// Wall-clock expiry (unix ms). After this instant the handle is inert.
    pub expires_at_unix_ms: u64,
}

impl SecretUseHandle {
    pub fn is_expired(&self, now_unix_ms: u64) -> bool {
        now_unix_ms >= self.expires_at_unix_ms
    }
}

/// The outcome of a secret-use authorization request.
///
/// `Allowed` carries a handle, never a value. `Denied` carries the machine-readable
/// reason from the pure decision function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum SecretUseOutcome {
    Allowed { handle: SecretUseHandle },
    Denied { reason: SecretUseDenialReason },
}

pub use cynic_protocol::secret_use::SecretUseDenialReason;

/// Result of executing a child process with an authorized secret injected.
///
/// The secret value is absent from every field — `stdout`/`stderr` are the
/// child's output, which the child itself controls and which is scanned for
/// accidental echo by the adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretUseExecutionReport {
    pub schema: &'static str,
    pub secret_id: String,
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// True when the child's output contained the injected value and it was redacted.
    pub output_redacted: bool,
}

pub const SECRET_USE_EXECUTION_SCHEMA: &str = "cynic.secret_use_execution.v1";
