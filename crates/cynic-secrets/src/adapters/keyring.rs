//! V3 adapter: resolve secrets from the OS credential store.
//!
//! Uses platform-native keyring tools via the typed argv runner port:
//! - Linux: `secret-tool` (libsecret D-Bus) — fallback to `pass` (password-store)
//! - macOS: `security` command (Keychain)
//! - Windows: unsupported (V3)
//!
//! The OS manages encryption at rest; CYNIC never touches raw secret key
//! material. This avoids the `keyring` crate dep which requires system libs.
//!
//! Authority: `cynic-protocol::secret_reference::{SecretClass, SecretReference}`.
//! The provider kinds (V1/V2/V3) are `SecretProviderKind`; V3 is
//! `OsCredentialStore`. A prose table cannot carry that mapping.

use crate::{
    SecretEntry, SecretError, SecretProviderPort, SecretProviderKind, SecretReference, SecretStatus,
    SecretStorePort,
};

use cynic_process::{ArgvRunner, LocalArgvRunner};

const SERVICE_NAME: &str = "cynic";

#[derive(Debug, Clone)]
pub struct OsCredentialStoreSecretProvider<R: ArgvRunner = LocalArgvRunner> {
    service_name: String,
    runner: R,
}

impl OsCredentialStoreSecretProvider<LocalArgvRunner> {
    pub fn new() -> Self {
        Self {
            service_name: SERVICE_NAME.to_string(),
            runner: LocalArgvRunner,
        }
    }
}

impl Default for OsCredentialStoreSecretProvider<LocalArgvRunner> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: ArgvRunner> OsCredentialStoreSecretProvider<R> {
    /// Wrap an injected argv runner (tests use `MapArgvRunner`).
    pub fn with_runner<R2: ArgvRunner>(runner: R2) -> OsCredentialStoreSecretProvider<R2> {
        OsCredentialStoreSecretProvider {
            service_name: SERVICE_NAME.to_string(),
            runner,
        }
    }

    /// Backwards-compatible alias for [`Self::with_runner`].
    pub fn new_with_runner<R2: ArgvRunner>(runner: R2) -> OsCredentialStoreSecretProvider<R2> {
        Self::with_runner(runner)
    }

    fn is_secret_tool_available(&self) -> bool {
        self.runner.run(&["which", "secret-tool"]).success()
    }

    fn is_security_available(&self) -> bool {
        self.runner.run(&["which", "security"]).success()
    }

    /// Linux: secret-tool lookup service cynic id <id>
    fn lookup_linux(&self, id: &str) -> Option<String> {
        if !self.is_secret_tool_available() {
            return None;
        }
        let service = self.service_name.as_str();
        let output = self
            .runner
            .run(&["secret-tool", "lookup", "service", service, "id", id]);
        if !output.success() {
            return None;
        }
        Some(output.stdout.trim_end().to_string())
    }

    /// macOS: security find-generic-password -s cynic -S <id> -w
    fn lookup_macos(&self, id: &str) -> Option<String> {
        if !self.is_security_available() {
            return None;
        }
        let service = self.service_name.as_str();
        let output = self.runner.run(&[
            "security",
            "find-generic-password",
            "-s",
            service,
            "-S",
            id,
            "-w",
        ]);
        if !output.success() {
            return None;
        }
        Some(output.stdout.trim_end().to_string())
    }
}

impl<R: ArgvRunner> SecretProviderPort for OsCredentialStoreSecretProvider<R> {
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        if reference.provider != SecretProviderKind::OsCredentialStore {
            return None;
        }
        // Cross-platform: try macOS security first, then Linux secret-tool.
        self.lookup_macos(&reference.locator)
            .or_else(|| self.lookup_linux(&reference.locator))
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

impl<R: ArgvRunner> SecretStorePort for OsCredentialStoreSecretProvider<R> {
    fn store(&self, spec: &SecretReference, value: &str) -> Result<(), SecretError> {
        if spec.provider != SecretProviderKind::OsCredentialStore {
            return Err(SecretError::Unsupported(format!(
                "provider {:?} not supported by OsCredentialStoreSecretProvider",
                spec.provider
            )));
        }

        // macOS: security add-generic-password -s cynic -S <id> -w <value> (delete if exists)
        #[cfg(target_os = "macos")]
        {
            let service = self.service_name.as_str();
            let locator = spec.locator.as_str();
            let _ = self.runner.run(&[
                "security",
                "delete-generic-password",
                "-s",
                service,
                "-S",
                locator,
            ]);
            let output = self.runner.run(&[
                "security",
                "add-generic-password",
                "-s",
                service,
                "-S",
                locator,
                "-w",
                value,
            ]);
            if !output.success() {
                return Err(SecretError::Backend(
                    "security add-generic-password failed".into(),
                ));
            }
            // Read-back: the store is only trusted when a lookup finds it.
            if self.lookup_macos(&spec.locator).is_none() {
                return Err(SecretError::Backend(
                    "security store unverified: lookup after store found nothing".into(),
                ));
            }
            return Ok(());
        }

        // Linux: secret-tool store --label=cynic service cynic id <id>
        // The secret value travels on the child's stdin (never argv, never
        // env). The previous implementation piped stdin but never wrote to
        // it, so every store silently persisted an EMPTY value (exit 0).
        #[cfg(target_os = "linux")]
        {
            if !self.is_secret_tool_available() {
                return Err(SecretError::Backend(
                    "secret-tool not found (install libsecret-tools)".into(),
                ));
            }
            let service = self.service_name.as_str();
            let locator = spec.locator.as_str();
            let argv: [&str; 7] = [
                "secret-tool",
                "store",
                "--label=cynic",
                "service",
                service,
                "id",
                locator,
            ];
            let refs: Vec<&str> = argv.to_vec();
            let output = self.runner.run_with_stdin(&refs, value.as_bytes());
            if !output.success() {
                return Err(SecretError::Backend("secret-tool store failed".into()));
            }
            // Read-back: the store is only trusted when a lookup finds it.
            if self.lookup_linux(&spec.locator).is_none() {
                return Err(SecretError::Backend(
                    "secret-tool store unverified: lookup after store found nothing".into(),
                ));
            }
            return Ok(());
        }

        Err(SecretError::Unsupported("V3 not supported on this platform".into()))
    }

    fn rotate(&self, spec: &SecretReference, new_value: &str) -> Result<(), SecretError> {
        // Delete + store = rotate
        self.delete(spec)?;
        self.store(spec, new_value)
    }

    fn delete(&self, spec: &SecretReference) -> Result<(), SecretError> {
        #[cfg(target_os = "macos")]
        {
            let service = self.service_name.as_str();
            let locator = spec.locator.as_str();
            let output = self.runner.run(&[
                "security",
                "delete-generic-password",
                "-s",
                service,
                "-S",
                locator,
            ]);
            let _ = output;
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            if !self.is_secret_tool_available() {
                return Ok(());
            }
            let service = self.service_name.as_str();
            let locator = spec.locator.as_str();
            let output = self.runner.run(&[
                "secret-tool",
                "clear",
                "service",
                service,
                "id",
                locator,
            ]);
            let _ = output;
            return Ok(());
        }

        Ok(())
    }

    fn list(&self) -> Vec<SecretEntry> {
        Vec::new()
    }
}