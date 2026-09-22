//! V2 adapter: resolve secrets from an encrypted principal-owned file.
//!
//! Secrets are stored as AES-256-GCM encrypted files at
//! `${CYNIC_PRINCIPAL_HOME}/.config/cynic/secrets/<id>`.
//!
//! Key derivation: `blake3(id || CYNIC_PRINCIPAL_SEED)` — a per-secret
//! key derived from the reference id and a principal-level seed.
//! The seed is resolved from environment or a seed file (never inline).
//!
//! File format: `[12-byte nonce || AES-GCM ciphertext || 16-byte tag]`.
//! File permissions set to 0600 on Unix.

use std::path::PathBuf;

use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};

use crate::{
    SecretEntry, SecretError, SecretProviderPort, SecretProviderKind, SecretReference, SecretStatus,
    SecretStorePort,
};

/// Derive the principal seed from environment or a file.
/// V1: CYNIC_PRINCIPAL_SEED env var.
/// V2: ${HOME}/.config/cynic/principal-seed (chmod 600).
/// Never logs or returns the seed value directly — used only for key derivation.
fn principal_seed() -> Vec<u8> {
    if let Ok(seed) = std::env::var("CYNIC_PRINCIPAL_SEED") {
        return seed.into_bytes();
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let seed_path = PathBuf::from(home).join(".config/cynic/principal-seed");
    std::fs::read_to_string(&seed_path).map(|s| s.trim().as_bytes().to_vec()).unwrap_or_default()
}

/// Generate a cryptographically secure random nonce via getrandom.
fn random_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    getrandom::getrandom(&mut nonce).expect("getrandom failed for nonce generation");
    nonce
}

#[derive(Debug, Clone)]
pub struct EncryptedFileSecretProvider {
    pub base_dir: PathBuf,
}

impl Default for EncryptedFileSecretProvider {
    fn default() -> Self {
        let home = std::env::var("CYNIC_PRINCIPAL_HOME")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| "/tmp".to_string());
        Self {
            base_dir: PathBuf::from(home).join(".config/cynic/secrets"),
        }
    }
}

impl EncryptedFileSecretProvider {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self { base_dir: base_dir.into() }
    }

    /// Derive a 256-bit key from the secret id + principal seed.
    fn derive_key(&self, id: &str) -> Key<Aes256Gcm> {
        let seed = principal_seed();
        let mut hasher = blake3::Hasher::new();
        hasher.update(id.as_bytes());
        hasher.update(&seed);
        let mut key = [0u8; 32];
        key.copy_from_slice(hasher.finalize().as_bytes());
        Key::<Aes256Gcm>::from(key)
    }

    fn secret_path(&self, reference: &SecretReference) -> PathBuf {
        // The locator can be a path (if starting with / or ~) or a simple id.
        let locator = &reference.locator;
        if locator.starts_with('/') || locator.starts_with("~") {
            PathBuf::from(locator)
        } else {
            self.base_dir.join(&reference.id)
        }
    }

    fn file_mtime_secs(&self, path: &std::path::Path) -> Option<u64> {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }
}

impl SecretProviderPort for EncryptedFileSecretProvider {
    fn resolve(&self, reference: &SecretReference) -> Option<String> {
        if reference.provider != SecretProviderKind::PrincipalOwnedFile {
            return None;
        }
        let path = self.secret_path(reference);
        let data = std::fs::read(&path).ok()?;
        if data.len() < 12 {
            return None;
        }
        let key = self.derive_key(&reference.id);
        let cipher = Aes256Gcm::new(&key);
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
        String::from_utf8(plaintext).ok()
    }

    fn status(&self, reference: &SecretReference) -> SecretStatus {
        let path = self.secret_path(reference);
        let exists = path.exists();
        SecretStatus {
            id: reference.id.clone(),
            class: reference.class,
            provider: reference.provider,
            locator: reference.locator.clone(),
            present: exists,
            last_rotated_secs: self.file_mtime_secs(&path),
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

        // V2 — TODO: integrate with cynic-askesis for append-only backup before write.
        // For now: backup old file by renaming to .bak before overwrite.
        let path = self.secret_path(spec);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SecretError::Backend(format!("mkdir: {e}")))?;
        }

        // Backup: if file exists, copy to .bak
        if path.exists() {
            let bak = path.with_extension("bak");
            let _ = std::fs::copy(&path, &bak);
        }

        let key = self.derive_key(&spec.id);
        let cipher = Aes256Gcm::new(&key);
        let nonce_bytes = random_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, value.as_bytes())
            .map_err(|e| SecretError::Crypto(e.to_string()))?;

        // Write: nonce (12 bytes) || ciphertext (includes 16-byte tag)
        let mut file_data = nonce.to_vec();
        file_data.extend_from_slice(&ciphertext);
        std::fs::write(&path, &file_data).map_err(|e| SecretError::Backend(e.to_string()))?;

        // Set restrictive permissions on Unix (0600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }

        Ok(())
    }

    fn rotate(&self, spec: &SecretReference, new_value: &str) -> Result<(), SecretError> {
        // Rotate = re-encrypt with a new key derived from the same id + seed.
        // For true key rotation, the seed changes — but that would invalidate
        // all derived keys. TODO: per-secret random keys stored in the file.
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
                    let id = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
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