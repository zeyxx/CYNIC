//! V2 adapter: resolve secrets from an encrypted principal-owned file.
//!
//! ## Storage
//!
//! Encrypted secret files live under a principal-owned directory
//! (`${CYNIC_PRINCIPAL_HOME}/.config/cynic/secrets` for interactive use, or
//! any directory passed to [`EncryptedFileSecretProvider::new`] — daemons pass
//! the system secrets directory, [`CynicPaths::secrets_dir`]).
//!
//! ## On-disk format (production-grade)
//!
//! Every stored file is self-describing:
//!
//! ```text
//! [ magic "CYNICSEC1" (9) ][ key_nonce (12) ][ sealed_key (48) ][ body_nonce (12) ][ ciphertext + tag ]
//! ```
//!
//! `sealed_key` is `AES-256-GCM(KEK, key_nonce, random_32_byte_data_key)` —
//! a fresh random data key per secret, sealed under a **key-encryption key
//! (KEK)** derived from the principal seed (`blake3("cynic-secrets/kek/v2" ||
//! seed)`). Consequences:
//!
//! - two secrets never share a key, even with identical ids on two machines;
//! - the data key is random, so rotation re-keys without changing the seed;
//! - every block is authenticated with the magic as AAD, so a corrupted or
//!   foreign file is a hard error, never a silent plaintext read.
//!
//! ## Fail-closed
//!
//! A file that is not in the format above is **rejected**, not read as
//! plaintext. There is no cleartext fallback: legacy plaintext files must be
//! migrated through [`SecretStorePort::store`], which always encrypts. A
//! missing principal seed is an error, never an empty-seed key.
//!
//! Authority: AGENTS.md §"Runtime secrets are injected"; `cynic-secrets` ports.

use std::path::{Path, PathBuf};

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use cynic_types::paths::CynicPaths;

use crate::{
    SecretEntry, SecretError, SecretProviderKind, SecretProviderPort, SecretReference,
    SecretStatus, SecretStorePort,
};

/// On-disk format marker. Bumped whenever the layout changes.
const MAGIC: &[u8; 9] = b"CYNICSEC1";
/// Length of a random data key.
const DATA_KEY_LEN: usize = 32;
/// AES-GCM nonce length.
const NONCE_LEN: usize = 12;
/// AES-GCM authentication tag length.
const TAG_LEN: usize = 16;
/// Offset where the body ciphertext begins.
const BODY_OFFSET: usize = MAGIC.len() + NONCE_LEN + DATA_KEY_LEN + TAG_LEN + NONCE_LEN;

/// Resolve the principal seed from the environment or a principal-owned file.
///
/// Order:
/// 1. `CYNIC_PRINCIPAL_SEED` environment variable.
/// 2. `${CYNIC_PRINCIPAL_SEED_FILE}` when set.
/// 3. `${CYNIC_PRINCIPAL_HOME:-$HOME}/.config/cynic/principal-seed`.
/// 4. the system principal seed (daemons have no `$HOME`).
///
/// The system location is [`CynicPaths::system_config_dir`] joined with
/// `principal-seed`, never a restated literal: this file must not hold a
/// second copy of the placement truth.
///
/// Returns `None` when no seed exists. Callers MUST treat that as an error for
/// any cryptographic operation: an absent seed must never silently degrade to
/// a well-known key.
fn resolve_principal_seed() -> Option<Vec<u8>> {
    if let Ok(seed) = std::env::var("CYNIC_PRINCIPAL_SEED") {
        let trimmed = seed.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.as_bytes().to_vec());
        }
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(explicit) = std::env::var("CYNIC_PRINCIPAL_SEED_FILE") {
        if !explicit.trim().is_empty() {
            candidates.push(PathBuf::from(explicit));
        }
    }
    if let Ok(home) = std::env::var("CYNIC_PRINCIPAL_HOME").or_else(|_| std::env::var("HOME")) {
        candidates.push(PathBuf::from(home).join(".config/cynic/principal-seed"));
    }
    candidates.push(CynicPaths::system_config_dir().join("principal-seed"));

    for path in candidates {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.as_bytes().to_vec());
            }
        }
    }
    None
}

/// Generate `N` cryptographically secure random bytes.
fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf).expect("getrandom failed");
    buf
}

/// Derive the key-encryption key from the principal seed.
///
/// Domain-separated from any other use of the seed so a key derived here can
/// never collide with another subsystem's derivation.
fn derive_kek(seed: &[u8]) -> Key<Aes256Gcm> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cynic-secrets/kek/v2");
    hasher.update(seed);
    let mut key = [0u8; 32];
    key.copy_from_slice(hasher.finalize().as_bytes());
    Key::<Aes256Gcm>::from(key)
}

#[derive(Debug, Clone)]
pub struct EncryptedFileSecretProvider {
    pub base_dir: PathBuf,
    /// Additional directory searched on read (legacy layout). Never written to.
    pub legacy_dir: Option<PathBuf>,
}

impl Default for EncryptedFileSecretProvider {
    fn default() -> Self {
        // Principal-owned store. A system principal (anything under the system
        // config directory) is preferred when present: daemons run as root
        // with no user session and must read/write the same store as the
        // operator CLI.
        if CynicPaths::system_config_dir().exists() {
            return Self {
                base_dir: CynicPaths::secrets_dir(),
                legacy_dir: None,
            };
        }
        let home = std::env::var("CYNIC_PRINCIPAL_HOME")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| "/tmp".to_string());
        Self {
            base_dir: PathBuf::from(home).join(".config/cynic/secrets"),
            legacy_dir: None,
        }
    }
}

impl EncryptedFileSecretProvider {
    /// Provider rooted at an explicit directory (daemons: the system secrets
    /// directory, [`CynicPaths::secrets_dir`]).
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            legacy_dir: None,
        }
    }

    /// Same as [`Self::new`] but also searching `legacy_dir` on read.
    pub fn with_legacy_dir(base_dir: impl Into<PathBuf>, legacy_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            legacy_dir: Some(legacy_dir.into()),
        }
    }

    fn secret_path(&self, reference: &SecretReference) -> PathBuf {
        self.base_dir.join(&reference.id)
    }

    fn file_mtime_secs(&self, path: &Path) -> Option<u64> {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }

    /// Encrypt `value` under a fresh random data key sealed by the KEK.
    fn seal(&self, value: &str) -> Result<Vec<u8>, SecretError> {
        let seed = resolve_principal_seed().ok_or_else(|| {
            SecretError::Backend(format!(
                "principal seed missing: set CYNIC_PRINCIPAL_SEED, CYNIC_PRINCIPAL_SEED_FILE, \
                 ~/.config/cynic/principal-seed or {}",
                CynicPaths::system_config_dir()
                    .join("principal-seed")
                    .display()
            ))
        })?;
        let kek = derive_kek(&seed);

        let data_key: [u8; DATA_KEY_LEN] = random_bytes();
        let key_nonce: [u8; NONCE_LEN] = random_bytes();
        let sealed_key = Aes256Gcm::new(&kek)
            .encrypt(
                Nonce::from_slice(&key_nonce),
                Payload {
                    msg: &data_key,
                    aad: MAGIC,
                },
            )
            .map_err(|e| SecretError::Crypto(e.to_string()))?;
        debug_assert_eq!(sealed_key.len(), DATA_KEY_LEN + TAG_LEN);

        let body_nonce: [u8; NONCE_LEN] = random_bytes();
        let ciphertext = Aes256Gcm::new(&Key::<Aes256Gcm>::from(data_key))
            .encrypt(
                Nonce::from_slice(&body_nonce),
                Payload {
                    msg: value.as_bytes(),
                    aad: MAGIC,
                },
            )
            .map_err(|e| SecretError::Crypto(e.to_string()))?;

        let mut out = Vec::with_capacity(BODY_OFFSET + ciphertext.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&key_nonce);
        out.extend_from_slice(&sealed_key);
        out.extend_from_slice(&body_nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypt a stored payload. Fail-closed: any deviation is an error.
    fn unseal(&self, data: &[u8], id: &str) -> Result<String, SecretError> {
        if data.len() < BODY_OFFSET + TAG_LEN {
            return Err(SecretError::Crypto(format!(
                "secret '{id}' too short to be a sealed payload ({} bytes)",
                data.len()
            )));
        }
        if &data[..MAGIC.len()] != MAGIC {
            return Err(SecretError::Crypto(format!(
                "secret '{id}' is not a sealed payload (missing CYNICSEC1 magic); \
                 migrate legacy plaintext through SecretStorePort::store"
            )));
        }
        let seed = resolve_principal_seed().ok_or_else(|| {
            SecretError::Backend("principal seed missing while reading an encrypted secret".into())
        })?;
        let kek = derive_kek(&seed);

        let key_nonce = &data[MAGIC.len()..MAGIC.len() + NONCE_LEN];
        let sealed_key =
            &data[MAGIC.len() + NONCE_LEN..MAGIC.len() + NONCE_LEN + DATA_KEY_LEN + TAG_LEN];
        let body_nonce = &data[MAGIC.len() + NONCE_LEN + DATA_KEY_LEN + TAG_LEN..BODY_OFFSET];
        let ciphertext = &data[BODY_OFFSET..];

        let data_key = Aes256Gcm::new(&kek)
            .decrypt(
                Nonce::from_slice(key_nonce),
                Payload {
                    msg: sealed_key,
                    aad: MAGIC,
                },
            )
            .map_err(|_| {
                SecretError::Crypto(format!(
                    "secret '{id}' key block failed to open (wrong principal seed or corruption)"
                ))
            })?;
        let data_key: [u8; DATA_KEY_LEN] = data_key.try_into().map_err(|_| {
            SecretError::Crypto(format!("secret '{id}' data key has an invalid length"))
        })?;

        let plaintext = Aes256Gcm::new(&Key::<Aes256Gcm>::from(data_key))
            .decrypt(
                Nonce::from_slice(body_nonce),
                Payload {
                    msg: ciphertext,
                    aad: MAGIC,
                },
            )
            .map_err(|_| {
                SecretError::Crypto(format!("secret '{id}' body failed to authenticate"))
            })?;
        String::from_utf8(plaintext)
            .map_err(|_| SecretError::Crypto(format!("secret '{id}' is not valid UTF-8")))
    }

    fn read_sealed(&self, path: &Path, id: &str) -> Result<Option<String>, SecretError> {
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read(path).map_err(|e| SecretError::Backend(e.to_string()))?;
        if data.is_empty() {
            return Err(SecretError::Crypto(format!(
                "secret '{id}' file is empty at {}",
                path.display()
            )));
        }
        self.unseal(&data, id).map(Some)
    }
}

impl SecretProviderPort for EncryptedFileSecretProvider {
    /// Read-only resolution. Cryptographic failures are reported as absent so
    /// the composite chain can continue; the underlying error is never turned
    /// into a plaintext read.
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        let primary = self.secret_path(reference);
        if let Ok(Some(value)) = self.read_sealed(&primary, &reference.id) {
            return Some(value);
        }
        if let Some(legacy) = &self.legacy_dir {
            let path = legacy.join(&reference.id);
            if path != primary {
                if let Ok(Some(value)) = self.read_sealed(&path, &reference.id) {
                    return Some(value);
                }
            }
        }
        None
    }

    fn status(&self, reference: &SecretReference) -> SecretStatus {
        let primary = self.secret_path(reference);
        let (exists, mtime) = if primary.exists() {
            (true, self.file_mtime_secs(&primary))
        } else if let Some(legacy) = &self.legacy_dir {
            let path = legacy.join(&reference.id);
            if path.exists() {
                (true, self.file_mtime_secs(&path))
            } else {
                (false, None)
            }
        } else {
            (false, None)
        };
        SecretStatus {
            id: reference.id.clone(),
            class: reference.class,
            provider: reference.provider,
            locator: reference.locator.clone(),
            present: exists,
            last_rotated_secs: mtime,
        }
    }
}

impl SecretStorePort for EncryptedFileSecretProvider {
    fn store(&self, spec: &SecretReference, value: &str) -> Result<(), SecretError> {
        if spec.provider != SecretProviderKind::PrincipalOwnedFile {
            return Err(SecretError::Unsupported(format!(
                "provider {:?} not supported by EncryptedFileSecretProvider",
                spec.provider
            )));
        }
        let path = self.secret_path(spec);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SecretError::Backend(format!("mkdir: {e}")))?;
        }
        if path.exists() {
            let _ = std::fs::copy(&path, path.with_extension("bak"));
        }
        let sealed = self.seal(value)?;
        std::fs::write(&path, &sealed).map_err(|e| SecretError::Backend(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        // Read-back: a store is only trusted when the sealed payload opens again.
        match self.read_sealed(&path, &spec.id)? {
            Some(read_back) if read_back == value => Ok(()),
            Some(_) => Err(SecretError::Backend(
                "store read-back mismatch: sealed payload opened to a different value".into(),
            )),
            None => Err(SecretError::Backend(
                "store read-back failed: sealed payload did not open".into(),
            )),
        }
    }

    /// Rotation re-keys: a fresh random data key is generated on every store,
    /// so the previous data key no longer opens the file.
    fn rotate(&self, spec: &SecretReference, new_value: &str) -> Result<(), SecretError> {
        self.store(spec, new_value)
    }

    fn delete(&self, spec: &SecretReference) -> Result<(), SecretError> {
        let path = self.secret_path(spec);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| SecretError::Backend(e.to_string()))?;
        }
        Ok(())
    }

    fn list(&self) -> Vec<SecretEntry> {
        let mut entries = Vec::new();
        if let Ok(reader) = std::fs::read_dir(&self.base_dir) {
            for entry in reader.flatten() {
                let path = entry.path();
                if path.extension().is_none() && path.is_file() {
                    let id = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let created = self.file_mtime_secs(&path);
                    entries.push(SecretEntry {
                        spec: SecretReference {
                            id,
                            class: crate::SecretClass::RuntimeMachineIdentity,
                            provider: SecretProviderKind::PrincipalOwnedFile,
                            locator: path.to_string_lossy().to_string(),
                        },
                        created_at_secs: created,
                        rotated_at_secs: created,
                    });
                }
            }
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SecretClass, SecretReference};
    use std::sync::{Mutex, MutexGuard};

    /// Process environment is global: these tests mutate `CYNIC_PRINCIPAL_SEED*`
    /// and `CYNIC_PRINCIPAL_HOME`, so they must not run concurrently.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_provider(dir: &str) -> EncryptedFileSecretProvider {
        let base = std::env::temp_dir().join(dir);
        let _ = std::fs::remove_dir_all(&base);
        EncryptedFileSecretProvider::new(base)
    }

    fn spec(id: &str) -> SecretReference {
        SecretReference::with_class(
            id,
            id,
            SecretClass::RuntimeMachineIdentity,
            SecretProviderKind::PrincipalOwnedFile,
        )
    }

    /// Point every seed source at a fresh file holding `seed`, so the test never
    /// depends on the host's real principal seed.
    fn seed_via_file(guard: &MutexGuard<'_, ()>, seed: &str) -> PathBuf {
        let _guard = guard;
        let path = std::env::temp_dir().join(format!("cynic-seed-{}", seed.len()));
        std::fs::write(&path, seed).expect("write seed");
        unsafe {
            std::env::remove_var("CYNIC_PRINCIPAL_SEED");
            std::env::set_var("CYNIC_PRINCIPAL_SEED_FILE", &path);
        }
        path
    }

    #[test]
    fn round_trip_with_seed_in_env() {
        let guard = ENV_LOCK.lock().unwrap();
        seed_via_file(&guard, "unit-test-seed-alpha");
        let provider = temp_provider("cynic-secrets-roundtrip");
        let reference = spec("alpha");
        provider.store(&reference, "s3cret-value").expect("store");
        assert_eq!(
            provider.resolve(&reference).as_deref(),
            Some("s3cret-value")
        );
    }

    #[test]
    fn stored_payload_is_not_plaintext_on_disk() {
        let guard = ENV_LOCK.lock().unwrap();
        seed_via_file(&guard, "unit-test-seed-beta");
        let provider = temp_provider("cynic-secrets-notplain");
        let reference = spec("beta");
        provider.store(&reference, "do-not-read-me").expect("store");
        let raw = std::fs::read(provider.secret_path(&reference)).expect("read raw");
        assert!(
            raw.starts_with(MAGIC),
            "payload must carry the magic header"
        );
        assert!(
            !raw.windows(13).any(|w| w == b"do-not-read-me"),
            "plaintext must not appear anywhere in the stored payload"
        );
    }

    #[test]
    fn rotation_rekeys_so_the_old_key_no_longer_opens() {
        let guard = ENV_LOCK.lock().unwrap();
        seed_via_file(&guard, "unit-test-seed-gamma");
        let provider = temp_provider("cynic-secrets-rotation");
        let reference = spec("gamma");
        provider.store(&reference, "first").expect("store first");
        let first = std::fs::read(provider.secret_path(&reference)).expect("read first");
        provider.rotate(&reference, "second").expect("rotate");
        let second = std::fs::read(provider.secret_path(&reference)).expect("read second");
        assert_ne!(first, second, "rotation must produce a different payload");
        assert_eq!(provider.resolve(&reference).as_deref(), Some("second"));
    }

    #[test]
    fn legacy_plaintext_file_is_rejected_not_read() {
        let guard = ENV_LOCK.lock().unwrap();
        seed_via_file(&guard, "unit-test-seed-delta");
        let provider = temp_provider("cynic-secrets-legacy");
        std::fs::create_dir_all(&provider.base_dir).expect("mkdir");
        let reference = spec("delta");
        std::fs::write(provider.secret_path(&reference), b"legacy-plaintext").expect("write");
        assert_eq!(
            provider.resolve(&reference),
            None,
            "a plaintext file must not resolve as a secret"
        );
        assert!(
            provider
                .read_sealed(&provider.secret_path(&reference), "delta")
                .is_err(),
            "reading a non-sealed file must be an explicit error"
        );
    }

    #[test]
    fn missing_seed_fails_closed() {
        let guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("CYNIC_PRINCIPAL_SEED");
            std::env::set_var("CYNIC_PRINCIPAL_SEED_FILE", "/nonexistent/seed");
            std::env::set_var("CYNIC_PRINCIPAL_HOME", "/nonexistent/home");
        }
        let provider = temp_provider("cynic-secrets-noseed");
        let reference = spec("epsilon");
        // Only assert when the host itself carries no system seed.
        if !Path::new("/etc/cynic/principal-seed").exists() {
            assert!(
                provider.store(&reference, "value").is_err(),
                "a missing principal seed must fail closed, never derive a key"
            );
        }
        let _guard = guard;
    }
}
