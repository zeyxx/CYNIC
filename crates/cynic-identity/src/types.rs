#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};

use ed25519_dalek::Signer;
use serde::Serialize;

/// NodeId — 128-bit fingerprint derived from ed25519 public key via BLAKE3.
///
/// Pure type: no I/O, no generation. Available in `no_std` contexts (Solana, WASM, sandboxes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct NodeId(pub [u8; 16]);

impl NodeId {
    pub fn from_ed25519(pubkey: &ed25519_dalek::VerifyingKey) -> Self {
        let hash = blake3::hash(pubkey.as_bytes());
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&hash.as_bytes()[..16]);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self, crate::error::IdentityError> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| crate::error::IdentityError::HexDecode(e.to_string()))?;
        if bytes.len() != 16 {
            return Err(crate::error::IdentityError::InvalidLength(bytes.len()));
        }
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl core::fmt::Display for NodeId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// NodeIdentity — full cryptographic identity of a CYNIC node.
///
/// Storage and signing are available in `no_std`.
/// Key generation (`generate()`) requires the `std` feature (OS entropy via OsRng).
/// Same split as `solana-program`: Pubkey (no_std) vs Keypair::generate (std).
#[derive(Debug, Clone, Serialize)]
pub struct NodeIdentity {
    #[serde(skip)]
    pub signing_key: ed25519_dalek::SigningKey,
    pub node_id: NodeId,
}

impl NodeIdentity {
    /// Generate a new random identity using OS entropy.
    /// Requires the `std` feature (not available in Solana/WASM/sandbox targets).
    #[cfg(feature = "std")]
    pub fn generate() -> Self {
        let signing_key =
            ed25519_dalek::SigningKey::generate(&mut rand_core::OsRng);
        let node_id = NodeId::from_ed25519(&signing_key.verifying_key());
        Self { signing_key, node_id }
    }

    /// Construct from an existing signing key — available in all targets.
    pub fn from_signing_key(signing_key: ed25519_dalek::SigningKey) -> Self {
        let node_id = NodeId::from_ed25519(&signing_key.verifying_key());
        Self { signing_key, node_id }
    }

    pub fn sign(&self, message: &[u8]) -> ed25519_dalek::Signature {
        self.signing_key.sign(message)
    }

    pub fn verifying_key(&self) -> ed25519_dalek::VerifyingKey {
        self.signing_key.verifying_key()
    }
}

/// SigningMaterial — opaque wrapper for a signing key.
/// Available in `no_std` (no generation, only signing from an existing key).
pub struct SigningMaterial(ed25519_dalek::SigningKey);

impl SigningMaterial {
    pub fn from_key(key: ed25519_dalek::SigningKey) -> Self {
        Self(key)
    }

    pub fn sign(&self, message: &[u8]) -> ed25519_dalek::Signature {
        self.0.sign(message)
    }

    pub fn node_id(&self) -> NodeId {
        NodeId::from_ed25519(&self.0.verifying_key())
    }

    pub fn verifying_key(&self) -> ed25519_dalek::VerifyingKey {
        self.0.verifying_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;

    #[test]
    fn node_id_deterministic() {
        let id1 = NodeIdentity::generate();
        let id2 = NodeIdentity::from_signing_key(id1.signing_key.clone());
        assert_eq!(id1.node_id, id2.node_id);
    }

    #[test]
    fn node_id_hex_roundtrip() {
        let id = NodeIdentity::generate();
        let hex = id.node_id.to_hex();
        let parsed = NodeId::from_hex(&hex).unwrap();
        assert_eq!(id.node_id, parsed);
    }

    #[test]
    fn sign_and_verify() {
        let id = NodeIdentity::generate();
        let message = b"test message";
        let sig = id.sign(message);
        assert!(id.verifying_key().verify(message, &sig).is_ok());
    }
}
