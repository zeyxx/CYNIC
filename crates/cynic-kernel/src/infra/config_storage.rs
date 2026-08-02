//! Storage configuration and runtime machine-identity references.

use cynic_protocol::{SecretClass, SecretProvider, SecretReference};

/// SurrealDB connection config loaded from `[storage]` in the injected runtime
/// TOML. Secret material is never retained in this object.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub url: String,
    pub allow_loopback: bool,
    pub namespace: String,
    pub database: String,
    pub identity: StorageIdentityConfig,
}

#[derive(Debug, Clone)]
pub struct StorageIdentityConfig {
    pub username: String,
    pub secret_sources: Vec<SecretReference>,
}

impl Default for StorageIdentityConfig {
    fn default() -> Self {
        Self {
            username: "cynic-runtime".to_string(),
            secret_sources: vec![
                SecretReference {
                    id: "storage-runtime-env".to_string(),
                    class: SecretClass::RuntimeMachineIdentity,
                    provider: SecretProvider::ProcessEnvironment,
                    locator: "SURREALDB_PASS".to_string(),
                },
                SecretReference {
                    id: "storage-runtime-os-credential".to_string(),
                    class: SecretClass::RuntimeMachineIdentity,
                    provider: SecretProvider::OsCredentialStore,
                    locator: "storage-runtime-password".to_string(),
                },
            ],
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            // Missing desired state is not a local-storage declaration. The
            // runtime adapter rejects this empty endpoint and preserves the
            // configuration gap as an explicit blocked state.
            url: String::new(),
            allow_loopback: false,
            namespace: "cynic".to_string(),
            database: "main".to_string(),
            identity: StorageIdentityConfig::default(),
        }
    }
}
