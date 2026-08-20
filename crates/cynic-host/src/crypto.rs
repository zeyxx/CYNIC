//! Cryptographic primitives for the CYNIC host.
//!
//! Provides a provider-based abstraction for signing, verification,
//! and key management to ensure the Trust-Chain vertical is substrate-agnostic.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::{RngCore, rngs::OsRng};
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Debug)]
pub enum CryptoError {
    OperationFailed(String),
    InvalidKey(String),
    Io(io::Error),
    Serialization(serde_json::Error),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationFailed(s) => write!(f, "cryptographic operation failed: {s}"),
            Self::InvalidKey(s) => write!(f, "invalid key material: {s}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Serialization(e) => write!(f, "serialization error: {e}"),
        }
    }
}

impl std::error::Error for CryptoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Serialization(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for CryptoError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for CryptoError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}
/// Provider for cryptographic signing and verification.
pub trait CryptoProvider: Send + Sync {
    /// Signs a message using the provided private key.
    fn sign(&self, message: &[u8], private_key: &[u8]) -> Result<Vec<u8>, CryptoError>;

    /// Verifies a signature against a message and public key.
    fn verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, CryptoError>;

    /// Generates a new Ed25519 keypair.
    /// Returns (SecretKey, PublicKey).
    fn generate_keypair(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError>;
}

/// Ed25519 implementation of the CryptoProvider.
pub struct Ed25519Provider;

impl CryptoProvider for Ed25519Provider {
    fn sign(&self, message: &[u8], private_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let bytes: [u8; 32] = private_key
            .try_into()
            .map_err(|_| CryptoError::InvalidKey("Invalid private key length".into()))?;
        let signing_key = SigningKey::from_bytes(&bytes);
        let signature = signing_key.sign(message);
        Ok(signature.to_bytes().to_vec())
    }

    fn verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, CryptoError> {
        let bytes: [u8; 32] = public_key
            .try_into()
            .map_err(|_| CryptoError::InvalidKey("Invalid public key length".into()))?;
        let verifying_key = VerifyingKey::from_bytes(&bytes)
            .map_err(|e| CryptoError::OperationFailed(e.to_string()))?;

        let sig = Signature::from_slice(signature)
            .map_err(|e| CryptoError::OperationFailed(e.to_string()))?;

        Ok(verifying_key.verify(message, &sig).is_ok())
    }

    fn generate_keypair(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let mut secret_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut secret_bytes);

        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = VerifyingKey::from(&signing_key);

        Ok((secret_bytes.to_vec(), verifying_key.to_bytes().to_vec()))
    }
}

/// Helper for secure storage of cryptographic material with strict file permissions.
pub struct SecretStore;

impl SecretStore {
    /// Saves a secret to a file with strict 0600 permissions.
    pub fn save_secret(path: &Path, secret: &[u8]) -> Result<(), CryptoError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, secret)?;
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)?;
        }
        Ok(())
    }

    /// Loads a secret from a file.
    pub fn load_secret(path: &Path) -> Result<Vec<u8>, CryptoError> {
        let data = fs::read(path)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keypair_signs_and_verifies() {
        let provider = Ed25519Provider;
        let (secret, public) = provider.generate_keypair().expect("keypair");
        assert_eq!(secret.len(), 32);
        assert_eq!(public.len(), 32);

        let message = b"cynic.machine_enrollment.v1";
        let signature = provider.sign(message, &secret).expect("sign");
        assert_eq!(signature.len(), 64);
        assert!(
            provider
                .verify(message, &signature, &public)
                .expect("verify")
        );
    }

    #[test]
    fn verification_refuses_a_tampered_message_or_signature() {
        // A wrong signature must return Ok(false), never an error that a caller
        // could mistake for a transport problem and retry into acceptance.
        let provider = Ed25519Provider;
        let (secret, public) = provider.generate_keypair().expect("keypair");
        let signature = provider.sign(b"original", &secret).expect("sign");

        assert!(
            !provider
                .verify(b"tampered", &signature, &public)
                .expect("verify runs")
        );

        let mut flipped = signature.clone();
        flipped[0] ^= 0x01;
        assert!(
            !provider
                .verify(b"original", &flipped, &public)
                .expect("verify runs")
        );
    }

    #[test]
    fn a_foreign_public_key_does_not_verify() {
        let provider = Ed25519Provider;
        let (secret, _) = provider.generate_keypair().expect("keypair a");
        let (_, other_public) = provider.generate_keypair().expect("keypair b");
        let signature = provider.sign(b"payload", &secret).expect("sign");

        assert!(
            !provider
                .verify(b"payload", &signature, &other_public)
                .expect("verify runs")
        );
    }

    #[test]
    fn malformed_key_material_is_rejected_as_invalid_key() {
        let provider = Ed25519Provider;
        assert!(matches!(
            provider.sign(b"m", &[0u8; 31]),
            Err(CryptoError::InvalidKey(_))
        ));
        assert!(matches!(
            provider.verify(b"m", &[0u8; 64], &[0u8; 31]),
            Err(CryptoError::InvalidKey(_))
        ));
    }

    #[test]
    fn two_generated_keypairs_differ() {
        let provider = Ed25519Provider;
        let (secret_a, public_a) = provider.generate_keypair().expect("keypair a");
        let (secret_b, public_b) = provider.generate_keypair().expect("keypair b");
        assert_ne!(secret_a, secret_b, "OsRng must not repeat key material");
        assert_ne!(public_a, public_b);
    }

    #[test]
    fn secret_store_saves_and_loads_with_restricted_permissions() {
        let dir = tempfile::tempdir().expect("tempdir");
        let secret_path = dir.path().join("sub/node-identity.key");
        let secret_data = b"super-secret-ed25519-seed-data";

        SecretStore::save_secret(&secret_path, secret_data).expect("save secret");
        assert!(secret_path.exists());

        #[cfg(unix)]
        {
            let meta = fs::metadata(&secret_path).expect("metadata");
            let mode = meta.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "Secret file must have 0600 permissions");
        }

        let loaded = SecretStore::load_secret(&secret_path).expect("load secret");
        assert_eq!(loaded, secret_data);
    }
}
