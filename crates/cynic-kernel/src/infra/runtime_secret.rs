use std::fmt;
use std::path::{Path, PathBuf};

use cynic_protocol::{SecretClass, SecretProvider, SecretReference};
use zeroize::Zeroize;

const MAX_SECRET_BYTES: usize = 64 * 1024;

pub trait RuntimeSecretSourcePort {
    fn environment(&self, name: &str) -> Result<Option<Vec<u8>>, String>;
    fn file(&self, path: &Path) -> Result<Option<Vec<u8>>, String>;
    fn credential_directory(&self) -> Option<PathBuf>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalRuntimeSecretSource;

impl RuntimeSecretSourcePort for LocalRuntimeSecretSource {
    fn environment(&self, name: &str) -> Result<Option<Vec<u8>>, String> {
        match std::env::var_os(name) {
            Some(value) => value
                .into_string()
                .map(|value| Some(value.into_bytes()))
                .map_err(|_| format!("environment source `{name}` is not valid UTF-8")),
            None => Ok(None),
        }
    }

    fn file(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("secret file metadata unavailable: {error}")),
        };
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err("secret source is not a regular non-symlink file".to_string());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode() & 0o777;
            if mode & 0o007 != 0 || mode & 0o022 != 0 {
                return Err(format!(
                    "secret file mode {mode:o} permits other access or non-owner writes"
                ));
            }
        }
        if metadata.len() > MAX_SECRET_BYTES as u64 {
            return Err("secret file exceeds the bounded size".to_string());
        }
        std::fs::read(path)
            .map(Some)
            .map_err(|error| format!("secret file unreadable: {error}"))
    }

    fn credential_directory(&self) -> Option<PathBuf> {
        std::env::var_os("CREDENTIALS_DIRECTORY").map(PathBuf::from)
    }
}

pub struct SecretMaterial(Vec<u8>);

impl SecretMaterial {
    pub(crate) fn new(value: String) -> Self {
        Self(value.into_bytes())
    }

    pub fn expose(&self) -> &str {
        std::str::from_utf8(&self.0).expect("runtime secret is validated as UTF-8")
    }
}

impl Drop for SecretMaterial {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for SecretMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretMaterial([REDACTED])")
    }
}

pub struct ResolvedRuntimeSecret {
    pub source: SecretReference,
    pub material: SecretMaterial,
}

impl fmt::Debug for ResolvedRuntimeSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResolvedRuntimeSecret")
            .field("source", &self.source)
            .field("material", &"[REDACTED]")
            .finish()
    }
}

pub fn resolve_runtime_secret(
    sources: &[SecretReference],
    port: &dyn RuntimeSecretSourcePort,
) -> Result<ResolvedRuntimeSecret, String> {
    let mut failures = Vec::new();
    for source in sources {
        if source.class != SecretClass::RuntimeMachineIdentity {
            failures.push(format!("{}:blocked:invalid_secret_class", source.id));
            continue;
        }
        let candidate = match source.provider {
            SecretProvider::ProcessEnvironment => port.environment(&source.locator),
            SecretProvider::PrincipalOwnedFile => port.file(Path::new(&source.locator)),
            SecretProvider::OsCredentialStore => {
                if !valid_credential_id(&source.locator) {
                    Err("invalid OS credential id".to_string())
                } else if let Some(directory) = port.credential_directory() {
                    port.file(&directory.join(&source.locator))
                } else {
                    Ok(None)
                }
            }
            SecretProvider::ExternalSecretStore => {
                Err("external secret store adapter unavailable".to_string())
            }
        };

        match candidate {
            Ok(Some(bytes)) => {
                let value = normalized_secret(bytes)
                    .map_err(|error| format!("secret source `{}`: {error}", source.id))?;
                return Ok(ResolvedRuntimeSecret {
                    source: source.clone(),
                    material: SecretMaterial::new(value),
                });
            }
            Ok(None) => failures.push(format!("{}:absent", source.id)),
            Err(error) => failures.push(format!("{}:blocked:{error}", source.id)),
        }
    }

    Err(format!(
        "runtime machine identity unavailable ({})",
        failures.join(",")
    ))
}

fn normalized_secret(mut bytes: Vec<u8>) -> Result<String, String> {
    if bytes.len() > MAX_SECRET_BYTES {
        bytes.zeroize();
        return Err("secret exceeds the bounded size".to_string());
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
    if bytes.is_empty() {
        return Err("secret is empty".to_string());
    }
    match String::from_utf8(bytes) {
        Ok(value) => Ok(value),
        Err(error) => {
            let mut bytes = error.into_bytes();
            bytes.zeroize();
            Err("secret is not valid UTF-8".to_string())
        }
    }
}

fn valid_credential_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct FakeSource {
        env: BTreeMap<String, Vec<u8>>,
        files: BTreeMap<PathBuf, Vec<u8>>,
        credential_dir: Option<PathBuf>,
    }

    impl RuntimeSecretSourcePort for FakeSource {
        fn environment(&self, name: &str) -> Result<Option<Vec<u8>>, String> {
            Ok(self.env.get(name).cloned())
        }

        fn file(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
            Ok(self.files.get(path).cloned())
        }

        fn credential_directory(&self) -> Option<PathBuf> {
            self.credential_dir.clone()
        }
    }

    fn reference(id: &str, provider: SecretProvider, locator: &str) -> SecretReference {
        SecretReference {
            id: id.to_string(),
            class: SecretClass::RuntimeMachineIdentity,
            provider,
            locator: locator.to_string(),
        }
    }

    #[test]
    fn ordered_sources_choose_process_injection_before_file() {
        let source = FakeSource {
            env: BTreeMap::from([("STORAGE_PASS".to_string(), b"injected".to_vec())]),
            files: BTreeMap::from([(PathBuf::from("/secret"), b"file".to_vec())]),
            credential_dir: None,
        };
        let references = vec![
            reference(
                "process",
                SecretProvider::ProcessEnvironment,
                "STORAGE_PASS",
            ),
            reference("file", SecretProvider::PrincipalOwnedFile, "/secret"),
        ];

        let resolved = resolve_runtime_secret(&references, &source).unwrap();
        assert_eq!(resolved.source.id, "process");
        assert_eq!(resolved.material.expose(), "injected");
        assert!(!format!("{resolved:?}").contains("injected"));
    }

    #[test]
    fn os_credential_id_is_resolved_below_credentials_directory() {
        let source = FakeSource {
            env: BTreeMap::new(),
            files: BTreeMap::from([(
                PathBuf::from("/run/credentials/cynic-kernel/storage-password"),
                b"machine-secret\n".to_vec(),
            )]),
            credential_dir: Some(PathBuf::from("/run/credentials/cynic-kernel")),
        };

        let resolved = resolve_runtime_secret(
            &[reference(
                "systemd",
                SecretProvider::OsCredentialStore,
                "storage-password",
            )],
            &source,
        )
        .unwrap();
        assert_eq!(resolved.material.expose(), "machine-secret");
    }

    #[test]
    fn missing_sources_are_an_explicit_failure() {
        let error = resolve_runtime_secret(
            &[reference(
                "file",
                SecretProvider::PrincipalOwnedFile,
                "/missing",
            )],
            &FakeSource::default(),
        )
        .unwrap_err();
        assert!(error.contains("file:absent"));
    }
}
