//! Cryptographic utilities for Ed25519 signature verification.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use hex::FromHex;
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a secure, high-entropy ID.
/// Returns a 32-byte hex string (similar length to UUID but more entropy).
pub fn generate_secure_id() -> String {
    let mut bytes = [0u8; 16]; // 128 bits of entropy (same as UUID but CSPRNG)
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Generate a high-entropy cryptographic nonce (256 bits).
pub fn generate_nonce() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Error types for cryptographic operations.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid public key format")]
    InvalidPublicKey,
    #[error("Invalid signature format")]
    InvalidSignature,
    #[error("Signature verification failed")]
    VerificationFailed,
    #[error("Timestamp too old (potential replay attack)")]
    TimestampExpired,
    #[error("Timestamp in the future")]
    TimestampFuture,
}

/// Max age for a signed request (in seconds) to prevent replay attacks.
const MAX_REQUEST_AGE_SECS: u64 = 300; // 5 minutes

/// Verify an Ed25519 signature against a message.
///
/// Expected format for `public_key_hex` and `signature_hex` is hexadecimal.
pub fn verify_signature(
    public_key_hex: &str,
    signature_hex: &str,
    timestamp_str: &str,
    message: &[u8],
) -> Result<(), CryptoError> {
    // 1. Verify timestamp to prevent replay attacks
    let request_ts: u64 = timestamp_str
        .parse()
        .map_err(|_e| CryptoError::TimestampExpired)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if now > request_ts && now - request_ts > MAX_REQUEST_AGE_SECS {
        return Err(CryptoError::TimestampExpired);
    }

    // Allow for small clock drift (up to 30s in the future)
    if request_ts > now && request_ts - now > 30 {
        return Err(CryptoError::TimestampFuture);
    }

    // 2. Parse public key
    let public_key_bytes =
        <[u8; 32]>::from_hex(public_key_hex).map_err(|_e| CryptoError::InvalidPublicKey)?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key_bytes).map_err(|_e| CryptoError::InvalidPublicKey)?;

    // 3. Parse signature
    let signature_bytes =
        <[u8; 64]>::from_hex(signature_hex).map_err(|_e| CryptoError::InvalidSignature)?;
    let signature = Signature::from_bytes(&signature_bytes);

    // 4. Verify signature against (timestamp | message)
    // The timestamp is included in the signed payload to ensure it hasn't been tampered with.
    let mut payload = timestamp_str.as_bytes().to_vec();
    payload.extend_from_slice(message);

    verifying_key
        .verify(&payload, &signature)
        .map_err(|_e| CryptoError::VerificationFailed)
}

/// Generate the message to sign for a REST request.
/// Format: `METHOD|PATH|BODY`
pub fn build_rest_payload(method: &str, path: &str, body: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(method.as_bytes());
    payload.push(b'|');
    payload.extend_from_slice(path.as_bytes());
    payload.push(b'|');
    payload.extend_from_slice(body);
    payload
}
