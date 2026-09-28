//! File-backed authority-side enrollment adapter.
//!
//! Canonical home since 2026-09-28. Moved verbatim (behavior-preserving) from
//! `cynic-kernel::bootstrap::node_enrollment`: the kernel performed host
//! filesystem I/O and imported `cynic-host` concretely (Session 5 inversion).
//! Node-local supervision owns enrollment loading; `node -> host` is a legal
//! downward edge.
//!
//! Complements `super::local_node_enrollment` (`LocalNodeEnrollmentReader`,
//! consumer-side read-back): this module is the authority side (loads the
//! authority key, enforces file permissions, seals grants). It does NOT
//! replace `bin/enroll_grant.rs`, whose `read_or_create` key semantics differ
//! (creation vs this loader's load-only gate) — that merge is a later slice.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

use cynic_node_protocol::MachineEnrollmentAuthority;

use crate::ports::node_enrollment_signer_port::NodeEnrollmentSignerPort;
use crate::ports::node_enrollment_store_port::{NodeEnrollmentRuntime, NodeEnrollmentStorePort};

/// Load the authority-side enrollment runtime from the host config root.
///
/// Returns `Ok(None)` (inactive, with a warning) when the authority files or
/// the store are absent — enrollment is opt-in, absence is not an error.
pub fn load_node_enrollment_runtime(
    store: Option<Arc<dyn NodeEnrollmentStorePort>>,
) -> Result<Option<Arc<NodeEnrollmentRuntime>>, String> {
    let authority_path = config_root().join("machine-enrollment-authority.json");
    let signing_key_path = config_root().join("machine-enrollment-signing-key");
    if !authority_path.is_file() || !signing_key_path.is_file() {
        tracing::warn!("machine enrollment authority is absent; automatic enrollment is inactive");
        return Ok(None);
    }
    let authority: MachineEnrollmentAuthority = read_json(&authority_path)?;
    validate_authority(&authority)?;
    let signer = FileEnrollmentSigner::load(&signing_key_path)?;
    let Some(store) = store else {
        tracing::warn!("storage_primary is unavailable; machine enrollment remains inactive");
        return Ok(None);
    };
    Ok(Some(Arc::new(NodeEnrollmentRuntime {
        authority,
        store,
        signer: Arc::new(signer),
    })))
}

/// Ed25519 file signer with a permission gate: the key file must not be
/// group-writable or accessible by others. Load-only — never creates keys
/// (creation lives in `bin/enroll_grant.rs` under different semantics).
struct FileEnrollmentSigner {
    key: SigningKey,
}

impl std::fmt::Debug for FileEnrollmentSigner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FileEnrollmentSigner")
            .field("key_id", &self.key_id())
            .finish()
    }
}

impl FileEnrollmentSigner {
    fn load(path: &Path) -> Result<Self, String> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let mode = std::fs::metadata(path)
                .map_err(|error| format!("inspect enrollment signing key: {error}"))?
                .mode()
                & 0o777;
            if mode & 0o027 != 0 {
                return Err(
                    "enrollment signing key must not be group-writable or accessible by others"
                        .to_string(),
                );
            }
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|error| format!("read enrollment signing key {}: {error}", path.display()))?;
        let bytes: [u8; 32] = hex::decode(raw.trim())
            .map_err(|_| "enrollment signing key must be hex".to_string())?
            .try_into()
            .map_err(|_| "enrollment signing key must contain 32 bytes".to_string())?;
        Ok(Self {
            key: SigningKey::from_bytes(&bytes),
        })
    }
}

impl NodeEnrollmentSignerPort for FileEnrollmentSigner {
    fn key_id(&self) -> String {
        format!(
            "sha256:{}",
            hex::encode(Sha256::digest(self.key.verifying_key().as_bytes()))
        )
    }

    fn sign(&self, payload: &[u8]) -> Result<String, String> {
        Ok(hex::encode(self.key.sign(payload).to_bytes()))
    }

    fn verify(&self, payload: &[u8], signature_hex: &str) -> Result<(), String> {
        let bytes =
            hex::decode(signature_hex).map_err(|_| "enrollment signature is not valid hex")?;
        if bytes.len() != 64 {
            return Err("enrollment signature must be 64 bytes".to_string());
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        let signature = ed25519_dalek::Signature::from_bytes(&arr);
        self.key
            .verifying_key()
            .verify_strict(payload, &signature)
            .map_err(|_| "enrollment signature verification failed".to_string())
    }
}

fn validate_authority(authority: &MachineEnrollmentAuthority) -> Result<(), String> {
    if authority.authority_id.trim().is_empty()
        || authority.seed_id.trim().is_empty()
        || authority.token_digest.trim().is_empty()
        || authority.max_grant_ttl_s == 0
        || authority.renewal_authority_expires_at_unix_s == 0
        || authority.allowed_roles.is_empty()
    {
        return Err("machine enrollment authority is incomplete".to_string());
    }
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let raw = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&raw).map_err(|error| format!("decode {}: {error}", path.display()))
}

fn config_root() -> PathBuf {
    cynic_host::runtime_authority::default_runtime_authority_paths().config_root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_authority() -> MachineEnrollmentAuthority {
        MachineEnrollmentAuthority {
            authority_id: "authority-1".to_string(),
            seed_id: "seed-1".to_string(),
            token_digest: "sha256:token".to_string(),
            expires_at_unix_s: 1_000,
            renewal_authority_expires_at_unix_s: 2_000,
            max_grant_ttl_s: 900,
            allowed_roles: vec!["build_runner".to_string()],
            allowed_capabilities: Vec::new(),
        }
    }

    #[test]
    fn authority_validation_rejects_each_incomplete_field() {
        assert!(validate_authority(&valid_authority()).is_ok());
        for mutate in [
            (|authority: &mut MachineEnrollmentAuthority| authority.authority_id.clear())
                as fn(&mut _),
            (|authority: &mut MachineEnrollmentAuthority| authority.seed_id.clear()),
            (|authority: &mut MachineEnrollmentAuthority| authority.token_digest.clear()),
            (|authority: &mut MachineEnrollmentAuthority| authority.max_grant_ttl_s = 0),
            (|authority: &mut MachineEnrollmentAuthority| {
                authority.renewal_authority_expires_at_unix_s = 0
            }),
            (|authority: &mut MachineEnrollmentAuthority| authority.allowed_roles.clear()),
        ] {
            let mut authority = valid_authority();
            mutate(&mut authority);
            assert_eq!(
                validate_authority(&authority),
                Err("machine enrollment authority is incomplete".to_string())
            );
        }
    }

    fn test_signer() -> FileEnrollmentSigner {
        FileEnrollmentSigner {
            key: SigningKey::from_bytes(&[7u8; 32]),
        }
    }

    #[test]
    fn signer_roundtrip_and_key_id_shape() {
        let signer = test_signer();
        assert!(signer.key_id().starts_with("sha256:"));
        assert_eq!(signer.key_id().len(), "sha256:".len() + 64);
        let signature = signer.sign(b"enroll-me").expect("sign");
        assert_eq!(signature.len(), 128);
        assert!(signer.verify(b"enroll-me", &signature).is_ok());
    }

    #[test]
    fn signer_rejects_malformed_and_forged_signatures() {
        let signer = test_signer();
        assert_eq!(
            signer.verify(b"enroll-me", "zzzz"),
            Err("enrollment signature is not valid hex".to_string())
        );
        assert_eq!(
            signer.verify(b"enroll-me", &"ab".repeat(16)),
            Err("enrollment signature must be 64 bytes".to_string())
        );
        let forged = test_signer_with_byte(9).sign(b"enroll-me").expect("sign");
        assert_eq!(
            signer.verify(b"enroll-me", &forged),
            Err("enrollment signature verification failed".to_string())
        );
        let signature = signer.sign(b"enroll-me").expect("sign");
        assert_eq!(
            signer.verify(b"tampered", &signature),
            Err("enrollment signature verification failed".to_string())
        );
    }

    fn test_signer_with_byte(byte: u8) -> FileEnrollmentSigner {
        FileEnrollmentSigner {
            key: SigningKey::from_bytes(&[byte; 32]),
        }
    }

    #[test]
    fn loader_rejects_non_hex_and_short_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("machine-enrollment-signing-key");
        std::fs::write(&key_path, "not-hex!!").expect("write");
        restrict_to_owner(&key_path);
        assert_eq!(
            FileEnrollmentSigner::load(&key_path),
            Err("enrollment signing key must be hex".to_string())
        );
        std::fs::write(&key_path, "ab".repeat(8)).expect("write");
        assert_eq!(
            FileEnrollmentSigner::load(&key_path),
            Err("enrollment signing key must contain 32 bytes".to_string())
        );
        std::fs::write(&key_path, "07".repeat(32)).expect("write");
        assert!(FileEnrollmentSigner::load(&key_path).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn loader_rejects_group_writable_keys() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("machine-enrollment-signing-key");
        std::fs::write(&key_path, "07".repeat(32)).expect("write");
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        assert_eq!(
            FileEnrollmentSigner::load(&key_path),
            Err(
                "enrollment signing key must not be group-writable or accessible by others"
                    .to_string()
            )
        );
    }

    #[cfg(unix)]
    fn restrict_to_owner(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    }

    #[cfg(not(unix))]
    fn restrict_to_owner(_path: &Path) {}
}
