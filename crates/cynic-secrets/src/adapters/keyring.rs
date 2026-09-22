//! V3 adapter: resolve secrets from the OS credential store (keyring).
//!
//! Uses `secret-service` (Linux D-Bus), `keychain` (macOS), or
//! Credential Manager (Windows). The OS manages encryption at rest —
//! CYNIC never touches raw secret key material.
//!
//! TODO: V3 — integrate with the `keyring` crate for cross-platform access.

use crate::{
    SecretEntry, SecretError, SecretProviderPort, SecretReference, SecretStatus, SecretStorePort,
};

#[derive(Debug, Clone)]
pub struct OsCredentialStoreSecretProvider {
    service_name: String,
}

impl Default for OsCredentialStoreSecretProvider {
    fn default() -> Self {
        Self {
            service_name: "CYNIC".to_string(),
        }
    }
}

impl OsCredentialStoreSecretProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretProviderPort for OsCredentialStoreSecretProvider {
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        // TODO: V3 — use `keyring` crate to lookup `locator` under `service_name`
        let _ = (self.service_name.as_str(), &reference.locator);
        None
    }

    fn status(&self, reference: &SecretReference) -> SecretStatus {
        let _ = &self.service_name;
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

impl SecretStorePort for OsCredentialStoreSecretProvider {
    fn store(&self, _spec: &SecretReference, _value: &str) -> Result<(), SecretError> {
        Err(SecretError::Unsupported(
            "V3 keyring store not yet implemented".into(),
        ))
    }

    fn rotate(&self, _spec: &SecretReference, _new_value: &str) -> Result<(), SecretError> {
        Err(SecretError::Unsupported(
            "V3 keyring rotate not yet implemented".into(),
        ))
    }

    fn delete(&self, _spec: &SecretReference) -> Result<(), SecretError> {
        Err(SecretError::Unsupported(
            "V3 keyring delete not yet implemented".into(),
        ))
    }

    fn list(&self) -> Vec<SecretEntry> {
        Vec::new()
    }
}
