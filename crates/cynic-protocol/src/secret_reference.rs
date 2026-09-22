//! Pure secret-material governance types.
//!
//! `SecretClass`, `SecretProvider`, and `SecretReference` name *what* secret
//! material exists, *where* it may live, and an inspectable reference to it —
//! never the value itself. They are consumed by identity modules
//! (`machine_identity`), storage identity, and kernel infra config; they do
//! not belong to any one enrollment flow.
//!
//! Authority: AGENTS.md secret classes table (HumanAdmin = bootstrap only).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretClass {
    HumanAdmin,
    MachineEnrollment,
    RuntimeMachineIdentity,
    RecoveryBreakglass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretProvider {
    ProcessEnvironment,
    PrincipalOwnedFile,
    OsCredentialStore,
    ExternalSecretStore,
}

/// An inspectable reference to secret material. `locator` names an environment
/// variable, file, credential id, or external-store key; it never contains the
/// secret value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretReference {
    pub id: String,
    pub class: SecretClass,
    pub provider: SecretProvider,
    pub locator: String,
}

impl SecretReference {
    /// V1 migration helper: construct a SecretReference for a process-environment
    /// variable. Replaces the `std::env::var("X")` anti-pattern with a typed,
    /// injectable port reference.
    ///
    /// Usage: `secrets.resolve(&SecretReference::env("my-secret", "MY_SECRET"))`
    pub fn env(id: &str, locator: &str) -> Self {
        Self {
            id: id.to_string(),
            class: SecretClass::RuntimeMachineIdentity,
            provider: SecretProvider::ProcessEnvironment,
            locator: locator.to_string(),
        }
    }
}
