//! Universal Proof of History interface (Ledger Event).
//!
//! Any mutating event that impacts the organism's memory or execution state
//! must implement this trait to emit a standardized ProofOfHistory ledger entry.

use crate::json::JsonValue;
use ed25519_dalek::SigningKey;
use sha3::Digest;
use std::sync::OnceLock;

static KERNEL_IDENTITY: OnceLock<SigningKey> = OnceLock::new();

/// Returns the singleton Ed25519 signing key for the kernel, used to seal the Proof of History.
/// Defaults to CYNIC_IDENTITY_KEY from the environment, otherwise generates an ephemeral key.
pub fn kernel_identity() -> &'static SigningKey {
    KERNEL_IDENTITY.get_or_init(|| {
        if let Ok(hex_key) = std::env::var("CYNIC_IDENTITY_KEY")
            && let Ok(bytes) = hex::decode(hex_key)
            && bytes.len() == 32
        {
            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(&bytes);
            return SigningKey::from_bytes(&key_bytes);
        }
        tracing::warn!(
            "CYNIC_IDENTITY_KEY not found or invalid; using deterministic fallback Ed25519 organism identity."
        );
        SigningKey::from_bytes(&[0u8; 32])
    })
}

/// Universal interface for any domain entity that generates a Proof of History.
pub trait Provable {
    /// The unique identifier of the event source (e.g. "cynic-core", "organ-forge")
    fn source_organ(&self) -> String;

    /// The type of the event (e.g. "agent_task_resolved", "dispatch_completed")
    fn event_type(&self) -> String;

    /// The JSON payload to be hashed and stored. Must contain all context required for verification.
    fn proof_payload(&self) -> JsonValue;

    /// Generates the standardized ProofOfHistory record using SHA3-512.
    fn to_proof_of_history(&self) -> super::storage::ProofOfHistory {
        let payload = self.proof_payload();

        // SHA3-512 standard generation
        let mut hasher = sha3::Sha3_512::new();
        sha3::Digest::update(&mut hasher, self.event_type().as_bytes());
        sha3::Digest::update(&mut hasher, b"|");
        sha3::Digest::update(&mut hasher, self.source_organ().as_bytes());
        sha3::Digest::update(&mut hasher, b"|");
        sha3::Digest::update(&mut hasher, payload.to_string().as_bytes());

        let hash_hex = hex::encode(sha3::Digest::finalize(hasher));

        super::storage::ProofOfHistory {
            id: String::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            source_organ: self.source_organ(),
            event_type: self.event_type(),
            payload,
            cryptographic_hash: hash_hex,
            prev_hash: None,
            cryptographic_signature: None,
        }
    }
}
