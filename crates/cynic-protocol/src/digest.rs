//! Canonical digest law (constitutional scope, foundation.json).
//!
//! - Internal integrity hashing is SHA3-512; digest strings carry the
//!   `sha3-512:` prefix.
//! - SHA-256 (`sha256:`) is mandated ONLY where an external contract
//!   requires it (on-chain program payloads expect 32-byte sha256,
//!   third-party APIs).
//! - BLAKE3 (`blake3:`) is permitted for high-throughput non-canonical
//!   caches only.
//! - Unprefixed digests are invalid and rejected by [`validate_digest`].
//!
//! Migration path from the sha256-era: [`digest_json`] now emits prefixed
//! `sha3-512:` digests; legacy stored values remain parseable via
//! [`parse_digest`] so verification code can compare across eras without
//! silently accepting unprefixed strings.

use serde::Serialize;
use sha3::Digest;

pub const SHA3_PREFIX: &str = "sha3-512:";
pub const SHA2_PREFIX: &str = "sha256:";
pub const BLAKE3_PREFIX: &str = "blake3:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestError(pub String);

impl std::fmt::Display for DigestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Canonical internal digest: SHA3-512 over canonical JSON bytes.
///
/// This replaces the legacy sha256-based `digest_json` for all new seals.
/// The output prefix makes the algorithm explicit and future-auditable.
pub fn digest_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = crate::integrity::canonical_json_bytes(value)?;
    let mut hasher = sha3::Sha3_512::new();
    hasher.update(bytes);
    Ok(format!("{}{}", SHA3_PREFIX, hex::encode(hasher.finalize())))
}

/// Raw-bytes internal digest, same law as [`digest_json`] without JSON
/// canonicalization.
pub fn digest_bytes(bytes: &[u8]) -> String {
    let mut hasher = sha3::Sha3_512::new();
    hasher.update(bytes);
    format!("{}{}", SHA3_PREFIX, hex::encode(hasher.finalize()))
}

/// External-contract digest: 32-byte SHA-256 where an on-chain program or
/// third-party API demands it. Never use internally for new surfaces.
pub fn digest_bytes_sha256_external(bytes: &[u8]) -> String {
    use sha2::Digest;
    format!(
        "{}{}",
        SHA2_PREFIX,
        hex::encode(sha2::Sha256::digest(bytes))
    )
}

/// Parse a digest string into `(algorithm, hex)`. Rejects unprefixed values
/// (constitutional rule) and unknown algorithms.
pub fn parse_digest(digest: &str) -> Result<(&'static str, &str), DigestError> {
    for prefix in [SHA3_PREFIX, SHA2_PREFIX, BLAKE3_PREFIX] {
        if let Some(hex_part) = digest.strip_prefix(prefix) {
            if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(DigestError(format!("malformed digest body: {digest}")));
            }
            return Ok((prefix, hex_part));
        }
    }
    Err(DigestError(format!(
        "unprefixed or unknown-algorithm digest rejected by constitutional digest law: {digest}"
    )))
}

/// Validate a digest string against the canonical law. `allow_legacy`
/// permits the sha256/blake3 prefixes (existing stored data); unprefixed
/// strings are always invalid.
pub fn validate_digest(digest: &str, allow_legacy: bool) -> Result<(), DigestError> {
    let (algorithm, _) = parse_digest(digest)?;
    if !allow_legacy && algorithm != SHA3_PREFIX {
        return Err(DigestError(format!(
            "non-canonical digest algorithm {algorithm}; canonical is {SHA3_PREFIX}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn digest_json_is_prefixed_sha3_and_deterministic() {
        let v = json!({"b": 1, "a": [true, null]});
        let d1 = digest_json(&v).unwrap();
        let d2 = digest_json(&v).unwrap();
        assert!(d1.starts_with("sha3-512:"));
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), SHA3_PREFIX.len() + 128); // 512 bits = 128 hex chars
    }

    #[test]
    fn parse_rejects_unprefixed_digests() {
        // The constitutional rule: bare hex is invalid even when it is a
        // well-formed hash body.
        let err = parse_digest(&"a".repeat(64)).unwrap_err();
        assert!(err.0.contains("unprefixed"));
    }

    #[test]
    fn validate_accepts_canonical_and_flagged_legacy() {
        let canonical = digest_bytes(b"x");
        assert!(validate_digest(&canonical, false).is_ok());

        let legacy = format!("{}{}", SHA2_PREFIX, "ab".repeat(32));
        assert!(validate_digest(&legacy, true).is_ok());
        assert!(validate_digest(&legacy, false).is_err());

        let blake = format!("{}{}", BLAKE3_PREFIX, "cd".repeat(32));
        assert!(validate_digest(&blake, true).is_ok());
    }

    #[test]
    fn external_sha256_helper_emits_32byte_hex() {
        let d = digest_bytes_sha256_external(b"on-chain payload");
        assert!(d.starts_with("sha256:"));
        assert_eq!(d[SHA2_PREFIX.len()..].len(), 64);
    }

    #[test]
    fn malformed_bodies_are_rejected() {
        assert!(parse_digest("sha3-512:").is_err());
        assert!(parse_digest("sha3-512:zzzz").is_err());
        assert!(parse_digest("md5:abc").is_err());
    }
}
