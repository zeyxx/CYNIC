use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use cynic_protocol::{
    CredentialLeaseMetadata, credential_lease_metadata_signing_bytes,
    validate_credential_lease_metadata,
};

use crate::ports::credential_lease_verifier_port::CredentialLeaseVerifierPort;

#[derive(Debug, Clone)]
pub struct Ed25519CredentialLeaseVerifier {
    authority_public_key_path: PathBuf,
}

impl Ed25519CredentialLeaseVerifier {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            authority_public_key_path: path.into(),
        }
    }
}

impl CredentialLeaseVerifierPort for Ed25519CredentialLeaseVerifier {
    fn verify(
        &self,
        metadata: &CredentialLeaseMetadata,
        expected_provider: &str,
        expected_material_digest: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let blockers = validate_credential_lease_metadata(
            metadata,
            expected_provider,
            expected_material_digest,
        );
        if !blockers.is_empty() {
            return Err(blockers.join(","));
        }
        let key_bytes = read_public_key(&self.authority_public_key_path)?;
        let key_id = format!("sha256:{}", hex::encode(Sha256::digest(key_bytes)));
        if key_id != metadata.authority_key_id {
            return Err("credential_lease_authority_key_id_mismatch".to_string());
        }
        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|_| "credential_lease_authority_public_key_invalid".to_string())?;
        let signature_bytes: [u8; 64] = hex::decode(&metadata.signature_hex)
            .map_err(|_| "credential_lease_signature_not_hex".to_string())?
            .try_into()
            .map_err(|_| "credential_lease_signature_length_invalid".to_string())?;
        verifying_key
            .verify(
                &credential_lease_metadata_signing_bytes(metadata)
                    .map_err(|error| format!("credential_lease_metadata_encode_failed:{error}"))?,
                &Signature::from_bytes(&signature_bytes),
            )
            .map_err(|_| "credential_lease_authority_signature_invalid".to_string())?;
        Ok(vec![metadata.authority_key_id.clone()])
    }
}

fn read_public_key(path: &Path) -> Result<[u8; 32], String> {
    let value = std::fs::read_to_string(path)
        .map_err(|error| format!("credential_lease_authority_key_read_failed:{error}"))?;
    hex::decode(value.trim())
        .map_err(|_| "credential_lease_authority_key_not_hex".to_string())?
        .try_into()
        .map_err(|_| "credential_lease_authority_key_length_invalid".to_string())
}
