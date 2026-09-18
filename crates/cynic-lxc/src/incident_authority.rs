//! Injected incident-authority boundary for one-shot LXC recovery actions.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use cynic_protocol::{BreakglassAdmission, CynicErrorCode};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const CONSUMPTION_SCHEMA: &str = "cynic.breakglass.consumption.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreakglassConsumptionProof {
    pub schema: String,
    pub receipt_id: String,
    pub target_scope: String,
    pub action: String,
    pub consumed_at_unix_s: u64,
    pub authority_key_id: String,
    pub admission_digest: String,
}

pub trait IncidentAuthorityPort: Send + Sync {
    fn verify_and_consume(
        &self,
        admission: &BreakglassAdmission,
        target_scope: &str,
        action: &str,
        now_unix_s: u64,
    ) -> Result<BreakglassConsumptionProof, CynicErrorCode>;
}

#[derive(Debug, Default)]
pub struct UnavailableIncidentAuthority;

impl IncidentAuthorityPort for UnavailableIncidentAuthority {
    fn verify_and_consume(
        &self,
        _: &BreakglassAdmission,
        _: &str,
        _: &str,
        _: u64,
    ) -> Result<BreakglassConsumptionProof, CynicErrorCode> {
        Err(CynicErrorCode::BreakglassAuthorityVerifierUnavailable)
    }
}

#[derive(Debug, Clone)]
pub struct LocalIncidentAuthority {
    public_key_path: PathBuf,
    receipt_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct LocalIncidentAuthorityConfig {
    public_key_path: PathBuf,
    receipt_root: PathBuf,
}

impl LocalIncidentAuthority {
    pub fn new(public_key_path: impl Into<PathBuf>, receipt_root: impl Into<PathBuf>) -> Self {
        Self {
            public_key_path: public_key_path.into(),
            receipt_root: receipt_root.into(),
        }
    }

    /// Loads the root-owned configuration selected by host bootstrap. The
    /// config names portable authority locations; it never contains signing
    /// material itself.
    pub fn from_config_file(path: &Path) -> Result<Self, CynicErrorCode> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|_| CynicErrorCode::BreakglassAuthorityVerifierUnavailable)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(CynicErrorCode::BreakglassAuthorityVerifierUnavailable);
        }
        let config: LocalIncidentAuthorityConfig = toml::from_str(
            &fs::read_to_string(path)
                .map_err(|_| CynicErrorCode::BreakglassAuthorityVerifierUnavailable)?,
        )
        .map_err(|_| CynicErrorCode::BreakglassAuthorityVerifierUnavailable)?;
        Ok(Self::new(config.public_key_path, config.receipt_root))
    }

    fn verifying_key(&self, expected_key_id: &str) -> Result<VerifyingKey, CynicErrorCode> {
        let metadata = fs::symlink_metadata(&self.public_key_path)
            .map_err(|_| CynicErrorCode::BreakglassAuthorityVerifierUnavailable)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(CynicErrorCode::BreakglassAuthorityVerifierUnavailable);
        }
        let encoded = fs::read_to_string(&self.public_key_path)
            .map_err(|_| CynicErrorCode::BreakglassAuthorityVerifierUnavailable)?;
        let bytes: [u8; 32] = hex::decode(encoded.trim())
            .map_err(|_| CynicErrorCode::BreakglassAuthorityVerifierUnavailable)?
            .try_into()
            .map_err(|_| CynicErrorCode::BreakglassAuthorityVerifierUnavailable)?;
        let key = VerifyingKey::from_bytes(&bytes)
            .map_err(|_| CynicErrorCode::BreakglassAuthorityVerifierUnavailable)?;
        let actual_key_id = format!("sha256:{}", hex::encode(Sha256::digest(key.as_bytes())));
        if actual_key_id != expected_key_id {
            return Err(CynicErrorCode::BreakglassSignatureInvalid);
        }
        Ok(key)
    }

    fn persist_and_read_back(
        &self,
        proof: &BreakglassConsumptionProof,
    ) -> Result<(), CynicErrorCode> {
        if !safe_receipt_id(&proof.receipt_id) {
            return Err(CynicErrorCode::BreakglassConsumptionPersistFailed);
        }
        let root = fs::symlink_metadata(&self.receipt_root)
            .map_err(|_| CynicErrorCode::BreakglassConsumptionPersistFailed)?;
        if root.file_type().is_symlink() || !root.is_dir() {
            return Err(CynicErrorCode::BreakglassConsumptionPersistFailed);
        }
        let path = self.receipt_root.join(format!("{}.json", proof.receipt_id));
        let bytes = serde_json::to_vec_pretty(proof)
            .map_err(|_| CynicErrorCode::BreakglassConsumptionPersistFailed)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o640);
        let mut file = match options.open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(CynicErrorCode::BreakglassReceiptConsumed);
            }
            Err(_) => return Err(CynicErrorCode::BreakglassConsumptionPersistFailed),
        };
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| CynicErrorCode::BreakglassConsumptionPersistFailed)?;
        let read_back: BreakglassConsumptionProof = serde_json::from_slice(
            &fs::read(&path).map_err(|_| CynicErrorCode::BreakglassConsumptionPersistFailed)?,
        )
        .map_err(|_| CynicErrorCode::BreakglassConsumptionPersistFailed)?;
        if &read_back != proof {
            return Err(CynicErrorCode::BreakglassConsumptionPersistFailed);
        }
        Ok(())
    }
}

impl IncidentAuthorityPort for LocalIncidentAuthority {
    fn verify_and_consume(
        &self,
        admission: &BreakglassAdmission,
        target_scope: &str,
        action: &str,
        now_unix_s: u64,
    ) -> Result<BreakglassConsumptionProof, CynicErrorCode> {
        if !admission
            .validate_for(target_scope, action, now_unix_s)
            .is_empty()
        {
            return Err(CynicErrorCode::BreakglassAdmissionInvalid);
        }
        let payload = admission
            .signing_payload()
            .map_err(|_| CynicErrorCode::BreakglassSignatureInvalid)?;
        let signature: [u8; 64] = hex::decode(&admission.signature_hex)
            .map_err(|_| CynicErrorCode::BreakglassSignatureInvalid)?
            .try_into()
            .map_err(|_| CynicErrorCode::BreakglassSignatureInvalid)?;
        self.verifying_key(&admission.authority_key_id)?
            .verify(&payload, &Signature::from_bytes(&signature))
            .map_err(|_| CynicErrorCode::BreakglassSignatureInvalid)?;
        let proof = BreakglassConsumptionProof {
            schema: CONSUMPTION_SCHEMA.to_string(),
            receipt_id: admission.receipt_id.clone(),
            target_scope: target_scope.to_string(),
            action: action.to_string(),
            consumed_at_unix_s: now_unix_s,
            authority_key_id: admission.authority_key_id.clone(),
            admission_digest: format!("sha256:{}", hex::encode(Sha256::digest(payload))),
        };
        self.persist_and_read_back(&proof)?;
        Ok(proof)
    }
}

fn safe_receipt_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed_admission(key: &SigningKey) -> BreakglassAdmission {
        let mut admission = BreakglassAdmission {
            schema: cynic_protocol::BREAKGLASS_ADMISSION_SCHEMA.to_string(),
            status: "admitted".to_string(),
            authority_class: "RecoveryBreakglass".to_string(),
            receipt_id: "INC-1".to_string(),
            target_scope: "lxc:105".to_string(),
            expires_at_unix_s: 100,
            allowed_actions: vec!["systemctl:restart:cynic-node.service".to_string()],
            consumption_proof: None,
            evidence_refs: vec!["incident:INC-1".to_string()],
            authority_key_id: format!(
                "sha256:{}",
                hex::encode(Sha256::digest(key.verifying_key().as_bytes()))
            ),
            signature_hex: String::new(),
        };
        admission.signature_hex =
            hex::encode(key.sign(&admission.signing_payload().unwrap()).to_bytes());
        admission
    }

    #[test]
    fn signed_receipt_is_consumed_once_with_read_back_proof() {
        let directory = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[9; 32]);
        let key_path = directory.path().join("authority.pub");
        fs::write(&key_path, hex::encode(key.verifying_key().as_bytes())).unwrap();
        let receipts = directory.path().join("receipts");
        fs::create_dir(&receipts).unwrap();
        let authority = LocalIncidentAuthority::new(key_path, receipts);
        let admission = signed_admission(&key);
        let proof = authority
            .verify_and_consume(
                &admission,
                "lxc:105",
                "systemctl:restart:cynic-node.service",
                1,
            )
            .unwrap();
        assert_eq!(proof.receipt_id, "INC-1");
        assert_eq!(
            authority.verify_and_consume(
                &admission,
                "lxc:105",
                "systemctl:restart:cynic-node.service",
                1
            ),
            Err(CynicErrorCode::BreakglassReceiptConsumed)
        );
    }
}
