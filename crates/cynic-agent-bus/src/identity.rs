use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::Rng;
use rand::RngCore;

use crate::types::AgentId;

/// Generate a new ed25519 keypair for an agent identity.
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut rng = rand::thread_rng();
    let mut seed = [0u8; 32];
    rng.fill_bytes(&mut seed);
    let signing = SigningKey::from_bytes(&seed);
    let verifying = VerifyingKey::from(&signing);
    (signing, verifying)
}

/// Derive an AgentId from a VerifyingKey.
pub fn agent_id(key: &VerifyingKey) -> AgentId {
    let hash = blake3::hash(key.as_bytes());
    let mut id = [0u8; 16];
    id.copy_from_slice(&hash.as_bytes()[..16]);
    AgentId(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_generates_unique_ids() {
        let (s1, v1) = generate_keypair();
        let (s2, v2) = generate_keypair();
        assert_ne!(agent_id(&v1), agent_id(&v2));
    }

    #[test]
    fn agent_id_is_deterministic() {
        let (_, v) = generate_keypair();
        assert_eq!(agent_id(&v), agent_id(&v));
    }
}
