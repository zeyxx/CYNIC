//! Cryptographic primitives for the CYNIC host.
//!
//! Provides a provider-based abstraction for signing, verification,
//! and key management to ensure the Trust-Chain vertical is substrate-agnostic.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::{RngCore, rngs::OsRng};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("cryptographic operation failed: {0}")]
    OperationFailed(String),
    #[error("invalid key material: {0}")]
    InvalidKey(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
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
}
