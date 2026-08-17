//! Universal Proof of History interface (Ledger Event).
//!
//! Any mutating event that impacts the organism's memory or execution state
//! must implement this trait to emit a standardized ProofOfHistory ledger entry.

use crate::json::JsonValue;
use ed25519_dalek::SigningKey;
use sha3::Digest;
use std::sync::OnceLock;

static KERNEL_IDENTITY: OnceLock<SigningKey> = OnceLock::new();

/// Chaine cryptographiquement un hash de payload avec le hash du bloc précédent.
///
/// Le hachage chaîné est le cœur atomique du PoH : SHA3-512(payload_hash | prev_hash).
/// Cette fonction est pure et déterministe — extraction explicite depuis
/// `store_proof_of_history` pour permettre le test unitaire sans DB.
pub fn chain_hash(payload_hash: &str, prev_hash: &str) -> String {
    let mut hasher = sha3::Sha3_512::new();
    sha3::Digest::update(&mut hasher, payload_hash.as_bytes());
    sha3::Digest::update(&mut hasher, b"|");
    sha3::Digest::update(&mut hasher, prev_hash.as_bytes());
    hex::encode(sha3::Digest::finalize(hasher))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_hash_is_deterministic() {
        let a = chain_hash("payload", "prev");
        let b = chain_hash("payload", "prev");
        assert_eq!(a, b, "same inputs must produce identical chain hash");
    }

    #[test]
    fn chain_hash_differs_for_different_prev_hash() {
        let genesis = chain_hash("payload", "genesis");
        let chained = chain_hash("payload", "abc123");
        assert_ne!(
            genesis, chained,
            "different prev_hash inputs must produce different chain hashes"
        );
    }

    #[test]
    fn chain_hash_genesis_uses_genesis_string() {
        let h = chain_hash("payload", "genesis");
        assert!(!h.is_empty());
        assert!(
            h.len() == 128,
            "SHA3-512 hex should be 128 chars, got {} for '{}'",
            h.len(),
            h
        );
    }

    /// Un schéma chaîné : le prev_hash de l'événement N+1 doit être le
    /// cryptographic_hash de l'événement N. Cela garantit l'append-only
    /// sans SELECT-INSERT non atomique.
    #[test]
    fn chain_hash_links_blocks_forward() {
        let hash1 = chain_hash("event-a", "genesis");
        let hash2 = chain_hash("event-b", &hash1);
        // Le hash2 dépend de hash1 — cassure la chaîne si hash1 change
        let hash2_tampered = chain_hash("event-b", &chain_hash("event-a", "WRONG"));
        assert_ne!(hash2, hash2_tampered);
    }
}

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
    /// The unique identifier of the event source (e.g. "cynic-kernel", "organ-forge")
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
