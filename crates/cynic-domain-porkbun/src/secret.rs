//! Secret resolution from process environment variables.
//!
//! Implements the `SecretProvider` port trait by reading env vars.
//! This is the V1 credential mechanism — the V2 `os_keyring` backend
//! would replace this adapter without changing any domain logic.

use cynic_domain_ops::SecretProvider as SecretProviderPort;
use cynic_protocol::secret_reference::{SecretProvider as SecretProviderEnum, SecretReference};

/// Resolve `SecretReference` values from process environment variables.
///
/// Supports `SecretProvider::ProcessEnvironment` (lookup env var by `locator`).
/// Returns `None` for other provider classes — callers fall back to other
/// resolution strategies (principal-owned file, OS keyring).
#[derive(Debug, Clone, Default)]
pub struct EnvSecretProvider;

impl SecretProviderPort for EnvSecretProvider {
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        match reference.provider {
            SecretProviderEnum::ProcessEnvironment => std::env::var(&reference.locator).ok(),
            _ => None,
        }
    }
}
