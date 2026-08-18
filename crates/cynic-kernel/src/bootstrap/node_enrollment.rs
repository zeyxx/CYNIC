use std::path::{Path, PathBuf};
use std::sync::Arc;

use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

use crate::domain::node_enrollment::{
    NodeEnrollmentRuntime, NodeEnrollmentSignerPort, NodeEnrollmentStorePort,
};
use cynic_protocol::MachineEnrollmentAuthority;

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
