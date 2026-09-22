//! V2 adapter: resolve secrets from an encrypted principal-owned file.
//!
//! The file stores `SecretEntry` metadata (never raw values in plaintext).
//! Secrets are encrypted at rest using `age` with a master key stored in
//! the OS keyring. Falls back to unencrypted JSON with chmod 600 when keyring
//! is unavailable (dev mode only).
//!
//! TODO: V2 — implement encryption with `age` crate + OS keyring master key.

use std::path::PathBuf;

use crate::{
    SecretEntry, SecretError, SecretProviderPort, SecretReference, SecretStatus, SecretStorePort,
};

#[derive(Debug, Clone)]
pub struct EncryptedFileSecretProvider {
    pub path: PathBuf,
}

impl Default for EncryptedFileSecretProvider {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            path: PathBuf::from(home).join(".cynic").join("secrets.json"),
        }
    }
}

impl SecretProviderPort for EncryptedFileSecretProvider {
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        // TODO: V2 — read from encrypted file
        None
    }

    fn status(&self, reference: &SecretReference) -> SecretStatus {
        SecretStatus {
            id: reference.id.clone(),
            class: reference.class,
            provider: reference.provider,
            locator: reference.locator.clone(),
            present: false,
            last_rotated_secs: None,
        }
    }
}

impl SecretStorePort for EncryptedFileSecretProvider {
    fn store(&self, _spec: &SecretReference, _value: &str) -> Result<(), SecretError> {
        Err(SecretError::Unsupported(
            "V2 file store not yet implemented".into(),
        ))
    }

    fn rotate(&self, _spec: &SecretReference, _new_value: &str) -> Result<(), SecretError> {
        Err(SecretError::Unsupported(
            "V2 file rotate not yet implemented".into(),
        ))
    }

    fn delete(&self, _spec: &SecretReference) -> Result<(), SecretError> {
        Err(SecretError::Unsupported(
            "V2 file delete not yet implemented".into(),
        ))
    }

    fn list(&self) -> Vec<SecretEntry> {
        Vec::new()
    }
}
