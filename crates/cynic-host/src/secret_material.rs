use std::io::Read;
use zeroize::Zeroize;

const MAX_SECRET_BYTES: u64 = 64 * 1024;
const RUNTIME_SECRET_RANDOM_BYTES: usize = 32;

pub(crate) struct SecretMaterial(Vec<u8>);

impl SecretMaterial {
    pub(crate) fn read_file(path: &std::path::Path) -> Result<Self, String> {
        use std::os::unix::fs::OpenOptionsExt as _;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(|error| format!("secret source unavailable: {error}"))?;
        let metadata = file
            .metadata()
            .map_err(|error| format!("secret source metadata unavailable: {error}"))?;
        if !metadata.file_type().is_file() || metadata.len() > MAX_SECRET_BYTES {
            return Err("secret source must be a bounded regular file".to_string());
        }
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err("secret source must not be readable by group or others".to_string());
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_SECRET_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("secret source read failed: {error}"))?;
        if bytes.len() as u64 > MAX_SECRET_BYTES {
            bytes.zeroize();
            return Err("secret source exceeds the bounded size".to_string());
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
        }
        Self::from_bytes(bytes)
    }

    pub(crate) fn read_human_admin_stdin() -> Result<Self, String> {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(MAX_SECRET_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("human admin stdin unavailable: {error}"))?;
        if bytes.len() as u64 > MAX_SECRET_BYTES {
            bytes.zeroize();
            return Err("human admin secret exceeds the bounded size".to_string());
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
        }
        Self::from_bytes(bytes)
    }

    pub(crate) fn generate_runtime_identity() -> Result<Self, String> {
        let mut random = [0u8; RUNTIME_SECRET_RANDOM_BYTES];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut source| source.read_exact(&mut random))
            .map_err(|error| format!("runtime identity entropy unavailable: {error}"))?;
        let encoded = hex::encode(random);
        random.zeroize();
        Self::from_bytes(encoded.into_bytes())
    }

    /// Resolve a credential reference.
    ///
    /// Accepted forms:
    /// - `"ENV:VAR_NAME"` — read from environment variable `VAR_NAME`
    /// - `"file:/path/to/secret"` — read from file
    /// - `"/path/to/secret"` — read from file (implicit, when path starts with `/`)
    ///
    /// The secret is validated as UTF-8 and bounded to `MAX_SECRET_BYTES`.
    pub(crate) fn resolve_credential(reference: &str) -> Result<Self, String> {
        if reference.starts_with("ENV:") {
            let var_name = &reference[4..];
            let value = std::env::var(var_name).map_err(|_| {
                format!("environment variable not found for credential ref: {var_name}")
            })?;
            Self::from_bytes(value.into_bytes())
        } else if reference.starts_with("file:") {
            let path = &reference[5..];
            Self::read_file(std::path::Path::new(path))
        } else if reference.starts_with('/') {
            Self::read_file(std::path::Path::new(reference))
        } else {
            Err(format!(
                "unsupported credential ref scheme: {reference} (expected ENV:, file:, or absolute path)"
            ))
        }
    }

    pub(crate) fn expose_bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn expose_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("secret material is validated as UTF-8")
    }

    #[cfg(test)]
    pub(crate) fn fixture(value: &str) -> Self {
        Self(value.as_bytes().to_vec())
    }

    fn from_bytes(mut bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.is_empty() {
            return Err("secret material is empty".to_string());
        }
        if std::str::from_utf8(&bytes).is_err() {
            bytes.zeroize();
            return Err("secret material is not valid UTF-8".to_string());
        }
        if bytes.iter().any(|byte| byte.is_ascii_control()) {
            bytes.zeroize();
            return Err("secret material contains a control character".to_string());
        }
        Ok(Self(bytes))
    }
}

impl std::fmt::Debug for SecretMaterial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretMaterial([REDACTED])")
    }
}

impl Drop for SecretMaterial {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_projects_material() {
        let secret = SecretMaterial::fixture("machine-enrollment-secret");
        assert!(!format!("{secret:?}").contains("machine-enrollment-secret"));
    }

    #[test]
    fn generated_runtime_identity_has_bounded_high_entropy_encoding() {
        let secret = SecretMaterial::generate_runtime_identity().unwrap();
        assert_eq!(secret.expose_str().len(), RUNTIME_SECRET_RANDOM_BYTES * 2);
        assert!(secret
            .expose_str()
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn file_source_is_bounded_and_trimmed_without_debug_exposure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("provider-token");
        std::fs::write(&path, "provider-secret\n").unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        let secret = SecretMaterial::read_file(&path).unwrap();

        assert_eq!(secret.expose_str(), "provider-secret");
        assert!(!format!("{secret:?}").contains("provider-secret"));
    }

    #[test]
    fn file_source_rejects_material_exposed_to_other_principals() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("provider-token");
        std::fs::write(&path, "provider-secret").unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let error = SecretMaterial::read_file(&path).unwrap_err();

        assert!(error.contains("group or others"));
    }

    // ── resolve_credential tests (file-only, safe for Rust 2024) ─────────────────

    #[test]
    fn resolve_credential_from_absolute_file_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("test-secret-1");
        std::fs::write(&path, "test-secret-value").unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        let result = SecretMaterial::resolve_credential(path.to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().expose_str(), "test-secret-value");
    }

    #[test]
    fn resolve_credential_explicit_file_prefix() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("test-secret-2");
        std::fs::write(&path, "file-prefix-value").unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        let result =
            SecretMaterial::resolve_credential(&format!("file:{}", path.to_str().unwrap()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().expose_str(), "file-prefix-value");
    }

    #[test]
    fn resolve_credential_file_not_found() {
        let result = SecretMaterial::resolve_credential("/nonexistent/path/secret");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("secret source unavailable"));
    }

    #[test]
    fn resolve_credential_unsupported_scheme() {
        let result = SecretMaterial::resolve_credential("http://example.com/secret");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("unsupported credential ref scheme"));
    }

    #[test]
    fn resolve_credential_file_too_permissive() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bad-perms-token");
        std::fs::write(&path, "secret").unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let result = SecretMaterial::resolve_credential(&format!("file:{}", path.to_str().unwrap()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("group or others"));
    }

    #[test]
    fn resolve_credential_empty_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("empty-token");
        std::fs::write(&path, "").unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        let result = SecretMaterial::resolve_credential(path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("secret material is empty"));
    }

    #[test]
    fn resolve_credential_strips_trailing_newline() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("newline-token");
        std::fs::write(&path, "value-with-newline\n").unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        let result = SecretMaterial::resolve_credential(path.to_str().unwrap());
        assert_eq!(result.unwrap().expose_str(), "value-with-newline");
    }

    #[test]
    fn resolve_credential_rejects_symlink() {
        let directory = tempfile::tempdir().unwrap();
        let real_path = directory.path().join("real-secret");
        let link_path = directory.path().join("symlink-secret");
        std::fs::write(&real_path, "symlink-target-value").unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&real_path, std::fs::Permissions::from_mode(0o600)).unwrap();

        std::os::unix::fs::symlink(&real_path, &link_path).unwrap();

        // Symlinks should be rejected by O_NOFOLLOW in read_file
        let result = SecretMaterial::resolve_credential(link_path.to_str().unwrap());
        assert!(result.is_err());
    }
}
