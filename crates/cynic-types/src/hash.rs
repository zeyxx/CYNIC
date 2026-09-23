//! 256-bit content hash (SHA-256 or BLAKE3).

use serde::{Deserialize, Serialize};

/// 32-byte cryptographic hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash32(pub [u8; 32]);

impl Hash32 {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex encoding with `sha256:` prefix (32-byte content address).
    pub fn to_hex(&self) -> String {
        format!("sha256:{}", hex::encode(self.0))
    }
}

impl From<[u8; 32]> for Hash32 {
    fn from(bytes: [u8; 32]) -> Self {
        Hash32(bytes)
    }
}

impl AsRef<[u8]> for Hash32 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Display for Hash32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Serialize for Hash32 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Hash32 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s: String = serde::Deserialize::deserialize(deserializer)?;
        let s = s.strip_prefix("sha256:").ok_or_else(|| {
            serde::de::Error::custom("expected sha256: prefix")
        })?;
        let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
        let mut arr = [0u8; 32];
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("expected 32 bytes"));
        }
        arr.copy_from_slice(&bytes[..32]);
        Ok(Hash32(arr))
    }
}
