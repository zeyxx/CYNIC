//! Pure protocol types for secret management.
//!
//! These types carry metadata about secrets — never the secret values
//! themselves. They are consumed by the ports layer and concrete adapters.
//!
//! Authority: AGENTS.md secret classes table.

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
