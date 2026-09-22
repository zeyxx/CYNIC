//! V1 adapter: resolve secrets from process environment variables.
//!
//! Reads `locator` as an env var name. This is the simplest backend —
//! suitable for CI/CD, containers, ephemeral sessions.
//!
//! Does NOT support store/rotate/delete/list (env vars are injected by
//! the orchestrator, not managed by CYNIC at runtime).

use crate::{SecretProviderKind, SecretProviderPort, SecretReference, SecretStatus};

#[derive(Debug, Clone, Default)]
pub struct ProcessEnvironmentSecretProvider;

impl SecretProviderPort for ProcessEnvironmentSecretProvider {
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        match reference.provider {
            SecretProviderKind::ProcessEnvironment => std::env::var(&reference.locator).ok(),
            _ => None,
        }
    }

    fn status(&self, reference: &SecretReference) -> SecretStatus {
        let present = self.resolve(reference).is_some();
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
