//! V3 adapter: resolve secrets from the OS credential store.
//!
//! Uses platform-native keyring tools via `std::process::Command`:
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
    SecretEntry, SecretError, SecretProviderKind, SecretProviderPort, SecretReference,
    SecretStatus, SecretStorePort,
};

const SERVICE_NAME: &str = "cynic";

#[derive(Debug, Clone)]
pub struct OsCredentialStoreSecretProvider {
    service_name: String,
}

impl Default for OsCredentialStoreSecretProvider {
    fn default() -> Self {
        Self {
            service_name: SERVICE_NAME.to_string(),
        }
    }
}

impl OsCredentialStoreSecretProvider {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_secret_tool_available() -> bool {
        std::process::Command::new("which")
            .arg("secret-tool")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn is_security_available() -> bool {
        std::process::Command::new("which")
            .arg("security")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Linux: secret-tool lookup service cynic id <id>
    fn lookup_linux(&self, id: &str) -> Option<String> {
        if !Self::is_secret_tool_available() {
            return None;
        }
        let output = std::process::Command::new("secret-tool")
            .args(["lookup", "service", &self.service_name, "id", id])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim_end().to_string())
    }

    /// macOS: security find-generic-password -s cynic -S <id> -w
    fn lookup_macos(&self, id: &str) -> Option<String> {
        let output = std::process::Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                &self.service_name,
                "-S",
                id,
                "-w",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim_end().to_string())
    }
}

impl SecretProviderPort for OsCredentialStoreSecretProvider {
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

impl SecretStorePort for OsCredentialStoreSecretProvider {
    fn store(&self, spec: &SecretReference, value: &str) -> Result<(), SecretError> {
        if spec.provider != SecretProviderKind::OsCredentialStore {
            return Err(SecretError::Unsupported(format!(
                "provider {:?} not supported by OsCredentialStoreSecretProvider",
                spec.provider
            )));
        }

        // macOS: security add-generic-password -s cynic -S <id> -w <value> <delete if exists>
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("security")
                .args([
                    "delete-generic-password",
                    "-s",
                    &self.service_name,
                    "-S",
                    &spec.locator,
                ])
                .output();
            let status = std::process::Command::new("security")
                .args([
                    "add-generic-password",
                    "-s",
                    &self.service_name,
                    "-S",
                    &spec.locator,
                    "-w",
                    value,
                ])
                .status()
                .map_err(|e| SecretError::Backend(e.to_string()))?;
            if !status.success() {
                return Err(SecretError::Backend(
                    "security add-generic-password failed".into(),
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
            if !Self::is_secret_tool_available() {
                return Err(SecretError::Backend(
                    "secret-tool not found (install libsecret-tools)".into(),
                ));
            }
            let mut child = std::process::Command::new("secret-tool")
                .args([
                    "store",
                    "--label=cynic",
                    "service",
                    &self.service_name,
                    "id",
                    &spec.locator,
                ])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| SecretError::Backend(e.to_string()))?;
            let write_result = child.stdin.take().map(|mut stdin| {
                use std::io::Write;
                stdin
                    .write_all(value.as_bytes())
                    .and_then(|_| stdin.flush())
            });
            if let Some(Err(e)) = write_result {
                let _ = child.kill();
                return Err(SecretError::Backend(format!(
                    "secret-tool stdin write failed: {e}"
                )));
            }
            let status = child
                .wait()
                .map_err(|e| SecretError::Backend(e.to_string()))?;
            if !status.success() {
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

        Err(SecretError::Unsupported(
            "V3 not supported on this platform".into(),
        ))
    }

    fn rotate(&self, spec: &SecretReference, new_value: &str) -> Result<(), SecretError> {
        // Delete + store = rotate
        self.delete(spec)?;
        self.store(spec, new_value)
    }

    fn delete(&self, spec: &SecretReference) -> Result<(), SecretError> {
        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("security")
                .args([
                    "delete-generic-password",
                    "-s",
                    &self.service_name,
                    "-S",
                    &spec.locator,
                ])
                .status()
                .map_err(|e| SecretError::Backend(e.to_string()))?;
            let _ = status;
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            if !Self::is_secret_tool_available() {
                return Ok(());
            }
            let status = std::process::Command::new("secret-tool")
                .args(["clear", "service", &self.service_name, "id", &spec.locator])
                .status()
                .map_err(|e| SecretError::Backend(e.to_string()))?;
            let _ = status;
            return Ok(());
        }

        Ok(())
    }

    fn list(&self) -> Vec<SecretEntry> {
        Vec::new()
    }
}
