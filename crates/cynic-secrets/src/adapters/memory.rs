//! In-memory secret provider for testing.
//!
//! Stores secret values in a `HashMap<String, String>` keyed by the
//! `SecretReference.id`. NEVER used in production — test only.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::{
    SecretEntry, SecretError, SecretProviderPort, SecretReference, SecretStatus, SecretStorePort,
};

#[derive(Debug, Default)]
pub struct MemorySecretProvider {
    secrets: Mutex<HashMap<String, String>>,
}

impl MemorySecretProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_secret(mut self, id: &str, value: &str) -> Self {
        self.secrets
            .lock()
            .unwrap()
            .insert(id.to_string(), value.to_string());
        self
    }
}

impl SecretProviderPort for MemorySecretProvider {
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        self.secrets.lock().unwrap().get(&reference.id).cloned()
    }

    fn status(&self, reference: &SecretReference) -> SecretStatus {
        let present = self.secrets.lock().unwrap().contains_key(&reference.id);
        SecretStatus {
            id: reference.id.clone(),
            class: reference.class,
            provider: reference.provider,
            locator: reference.locator.clone(),
            present,
            last_rotated_secs: None,
        }
    }
}

impl SecretStorePort for MemorySecretProvider {
    fn store(&self, spec: &SecretReference, value: &str) -> Result<(), SecretError> {
        self.secrets
            .lock()
            .unwrap()
            .insert(spec.id.clone(), value.to_string());
        Ok(())
    }

    fn rotate(&self, spec: &SecretReference, new_value: &str) -> Result<(), SecretError> {
        self.store(spec, new_value)
    }

    fn delete(&self, spec: &SecretReference) -> Result<(), SecretError> {
        self.secrets.lock().unwrap().remove(&spec.id);
        Ok(())
    }

    fn list(&self) -> Vec<SecretEntry> {
        self.secrets
            .lock()
            .unwrap()
            .keys()
            .map(|id| SecretEntry {
                spec: SecretReference {
                    id: id.clone(),
                    class: crate::SecretClass::RuntimeMachineIdentity,
                    provider: crate::SecretProviderKind::ProcessEnvironment,
                    locator: id.clone(),
                },
                created_at_secs: None,
                rotated_at_secs: None,
            })
            .collect()
    }
}
