//! Agent identity — 128-bit fingerprint from ed25519 public key via BLAKE3.
//!
//! Pure type: no I/O, no generation. The fingerprint is deterministic —
//! the same public key always yields the same `AgentId`.

use blake3::Hasher;
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

/// 128-bit agent identifier.
///
/// Derived from `BLAKE3(ed25519_verifying_key)[:16]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub [u8; 16]);

impl AgentId {
    /// Derive an `AgentId` from an ed25519 verifying key.
    pub fn from_ed25519(key: &VerifyingKey) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(key.as_bytes());
        let mut id = [0u8; 16];
        id.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
        AgentId(id)
    }

    /// Construct from raw 16 bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        AgentId(bytes)
    }

    /// Borrow the raw byte representation.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Lowercase hex encoding (32 chars).
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from lowercase hex (must decode to exactly 16 bytes).
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let bytes = hex::decode(hex_str).map_err(|e| e.to_string())?;
        if bytes.len() != 16 {
            return Err(format!("expected 16 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        Ok(AgentId(arr))
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u8; 16]> for AgentId {
    fn from(bytes: [u8; 16]) -> Self {
        AgentId(bytes)
    }
}
