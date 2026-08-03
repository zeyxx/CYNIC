use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::ports::build_workload_grant_port::{
    BuildWorkloadGrantSignerPort, BuildWorkloadGrantVerifierPort,
};

#[derive(Clone)]
pub struct Ed25519BuildWorkloadGrantSigner {
    signing_key: SigningKey,
    authority_key_id: String,
}

impl std::fmt::Debug for Ed25519BuildWorkloadGrantSigner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Ed25519BuildWorkloadGrantSigner")
            .field("authority_key_id", &self.authority_key_id)
            .finish_non_exhaustive()
    }
}

impl Ed25519BuildWorkloadGrantSigner {
    pub fn load_hex_file(path: &Path) -> Result<Self, String> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("build grant signing key metadata: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("build grant signing key must be a regular non-symlink file".to_string());
        }
        let encoded = fs::read_to_string(path)
            .map_err(|error| format!("build grant signing key read: {error}"))?;
        let bytes = hex::decode(encoded.trim())
            .map_err(|_| "build grant signing key must be hexadecimal".to_string())?;
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "build grant signing key must contain exactly 32 bytes".to_string())?;
        Ok(Self::from_signing_key(SigningKey::from_bytes(&key_bytes)))
    }

    fn from_signing_key(signing_key: SigningKey) -> Self {
        let authority_key_id = key_id(&signing_key.verifying_key());
        Self {
            signing_key,
            authority_key_id,
        }
    }

    pub fn verifier(&self) -> Ed25519BuildWorkloadGrantVerifier {
        Ed25519BuildWorkloadGrantVerifier::new(self.signing_key.verifying_key())
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().as_bytes())
    }

    pub fn persist_public_key(&self, path: &Path) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or_else(|| "build grant public key parent missing".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("build grant public key parent create: {error}"))?;
        let parent_metadata = fs::symlink_metadata(parent)
            .map_err(|error| format!("build grant public key parent metadata: {error}"))?;
        if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
            return Err("build grant public key parent rejected".to_string());
        }
        let desired = format!("{}\n", self.public_key_hex());
        if fs::read_to_string(path).ok().as_deref() == Some(desired.as_str()) {
            return Ok(());
        }
        if fs::symlink_metadata(path)
            .ok()
            .is_some_and(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err("build grant public key path rejected".to_string());
        }
        let temporary = parent.join(format!(".build-authority.{}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o640);
        let mut file = options
            .open(&temporary)
            .map_err(|error| format!("build grant public key temp create: {error}"))?;
        file.write_all(desired.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("build grant public key temp write: {error}"))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("build grant public key publish: {error}"))?;
        let read_back = fs::read_to_string(path)
            .map_err(|error| format!("build grant public key read back: {error}"))?;
        if read_back != desired {
            return Err("build grant public key read back mismatch".to_string());
        }
        Ok(())
    }
}

impl BuildWorkloadGrantSignerPort for Ed25519BuildWorkloadGrantSigner {
    fn authority_key_id(&self) -> &str {
        &self.authority_key_id
    }

    fn sign(&self, payload: &[u8]) -> Result<String, String> {
        Ok(hex::encode(self.signing_key.sign(payload).to_bytes()))
    }
}

#[derive(Debug, Clone)]
pub struct Ed25519BuildWorkloadGrantVerifier {
    verifying_key: VerifyingKey,
    authority_key_id: String,
}

impl Ed25519BuildWorkloadGrantVerifier {
    pub fn from_public_key_file(path: &Path) -> Result<Self, String> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("build grant public key metadata: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("build grant public key must be a regular non-symlink file".to_string());
        }
        let encoded = fs::read_to_string(path)
            .map_err(|error| format!("build grant public key read: {error}"))?;
        Self::from_public_key_hex(&encoded)
    }

    pub fn from_public_key_hex(encoded: &str) -> Result<Self, String> {
        let bytes = hex::decode(encoded.trim())
            .map_err(|_| "build grant public key must be hexadecimal".to_string())?;
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "build grant public key must contain exactly 32 bytes".to_string())?;
        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|_| "build grant public key is invalid".to_string())?;
        Ok(Self::new(verifying_key))
    }

    fn new(verifying_key: VerifyingKey) -> Self {
        let authority_key_id = key_id(&verifying_key);
        Self {
            verifying_key,
            authority_key_id,
        }
    }
}

impl BuildWorkloadGrantVerifierPort for Ed25519BuildWorkloadGrantVerifier {
    fn verify(
        &self,
        authority_key_id: &str,
        payload: &[u8],
        signature_hex: &str,
    ) -> Result<(), String> {
        if authority_key_id != self.authority_key_id {
            return Err("build grant authority key id mismatch".to_string());
        }
        let signature_bytes: [u8; 64] = hex::decode(signature_hex)
            .map_err(|_| "build grant signature must be hexadecimal".to_string())?
            .try_into()
            .map_err(|_| "build grant signature must contain exactly 64 bytes".to_string())?;
        self.verifying_key
            .verify(payload, &Signature::from_bytes(&signature_bytes))
            .map_err(|_| "build grant signature verification failed".to_string())
    }
}

fn key_id(verifying_key: &VerifyingKey) -> String {
    format!(
        "sha256:{}",
        hex::encode(Sha256::digest(verifying_key.as_bytes()))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signer_identity_and_signature_are_read_back_by_the_verifier() {
        let signer =
            Ed25519BuildWorkloadGrantSigner::from_signing_key(SigningKey::from_bytes(&[7_u8; 32]));
        let verifier = signer.verifier();
        let payload = b"content-addressed-build-workload";
        let signature = signer.sign(payload).expect("sign payload");

        assert!(
            verifier
                .verify(signer.authority_key_id(), payload, &signature)
                .is_ok()
        );
        assert!(
            verifier
                .verify(signer.authority_key_id(), b"different", &signature)
                .is_err()
        );
    }
}
