//! Composite secret provider — tries multiple backends in priority order.
//!
//! Resolution order: `ProcessEnvironment` → `PrincipalOwnedFile` → `OsCredentialStore`.
//! This lets CYNIC operate in dev (env vars), staging (encrypted file), and
//! production (OS keyring) without changing domain logic.
//!
//! Pure orchestration — delegates I/O to the injected adapters.

use crate::{SecretEntry, SecretError, SecretProviderPort, SecretReference, SecretStatus};

pub struct CompositeSecretProvider {
    pub env: super::adapters::ProcessEnvironmentSecretProvider,
    pub file: super::adapters::EncryptedFileSecretProvider,
    pub keyring: super::adapters::OsCredentialStoreSecretProvider,
}

impl Default for CompositeSecretProvider {
    fn default() -> Self {
        Self {
            env: super::adapters::ProcessEnvironmentSecretProvider,
            file: super::adapters::EncryptedFileSecretProvider::default(),
            keyring: super::adapters::OsCredentialStoreSecretProvider::new(),
        }
    }
}

impl SecretProviderPort for CompositeSecretProvider {
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        // Try env first, then file, then OS keyring.
        self.env
            .resolve(reference)
            .or_else(|| self.file.resolve(reference))
            .or_else(|| self.keyring.resolve(reference))
    }

    fn status(&self, reference: &SecretReference) -> SecretStatus {
        if self.env.resolve(reference).is_some() {
            self.env.status(reference)
        } else if self.file.resolve(reference).is_some() {
            self.file.status(reference)
        } else {
            self.keyring.status(reference)
        }
    }
}

impl super::SecretStorePort for CompositeSecretProvider {
    fn store(&self, spec: &SecretReference, value: &str) -> Result<(), SecretError> {
        match spec.provider {
            crate::SecretProviderKind::ProcessEnvironment => Err(SecretError::Unsupported(
                "env var storage not supported".into(),
            )),
            crate::SecretProviderKind::PrincipalOwnedFile => self.file.store(spec, value),
            crate::SecretProviderKind::OsCredentialStore => self.keyring.store(spec, value),
            crate::SecretProviderKind::ExternalSecretStore => Err(SecretError::Unsupported(
                "external store not configured".into(),
            )),
        }
    }

    fn rotate(&self, spec: &SecretReference, new_value: &str) -> Result<(), SecretError> {
        self.delete(spec)?;
        self.store(spec, new_value)
    }

    fn delete(&self, spec: &SecretReference) -> Result<(), SecretError> {
        match spec.provider {
            crate::SecretProviderKind::PrincipalOwnedFile => self.file.delete(spec),
            crate::SecretProviderKind::OsCredentialStore => self.keyring.delete(spec),
            _ => Err(SecretError::Unsupported(format!(
                "delete not supported for {:?}",
                spec.provider
            ))),
        }
    }

    fn list(&self) -> Vec<SecretEntry> {
        let mut entries = self.file.list();
        entries.extend(self.keyring.list());
        entries
    }
}
