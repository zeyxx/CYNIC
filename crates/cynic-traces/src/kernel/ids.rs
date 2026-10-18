//! Content addressing and stable identifiers.
//!
//! Strict port of `apps/cynic-data/src/cynic_data/kernel/ids.py`.
//! Content addressing is sha256 over canonical bytes. Identifiers are
//! deterministic so that a re-run of the pipeline is idempotent.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Return the canonical sha256 digest of a payload, prefixed with its algorithm.
pub fn content_hash(payload: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(payload);
    format!("sha256:{}", hex_bytes(&h.finalize()))
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap_or('0'));
    }
    s
}

fn short_hash(text: &str, length: usize) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    let full = hex_bytes(&h.finalize());
    full[..length.min(64)].to_owned()
}

/// A harness-qualified session identifier: `<harness>:<raw_id>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUid {
    pub harness: String,
    pub raw_id: String,
}

impl std::fmt::Display for SessionUid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.harness, self.raw_id)
    }
}

pub fn turn_uid(session_uid: &str, index: i64) -> String {
    format!("{session_uid}:t{index:04}")
}

pub fn tool_uid(session_uid: &str, index: i64) -> String {
    format!("{session_uid}:x{index:04}")
}

pub fn reasoning_uid(session_uid: &str, index: i64) -> String {
    format!("{session_uid}:r{index:04}")
}

pub fn edge_uid(session_uid: &str, kind: &str, ordinal: i64) -> String {
    format!("{session_uid}:e{ordinal:04}:{kind}")
}

pub fn state_uid(session_uid: &str, index: i64) -> String {
    format!("{session_uid}:s{index:04}")
}

pub fn human_uid(session_uid: &str, turn: &str) -> String {
    format!("{session_uid}:h:{}", short_hash(turn, 10))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_prefixed_sha256() {
        assert_eq!(
            content_hash(b""),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn uid_shapes_match_python() {
        assert_eq!(turn_uid("h:s", 3), "h:s:t0003");
        assert_eq!(tool_uid("h:s", 3), "h:s:x0003");
        assert_eq!(reasoning_uid("h:s", 3), "h:s:r0003");
        assert_eq!(edge_uid("h:s", "reply", 3), "h:s:e0003:reply");
        assert_eq!(state_uid("h:s", 3), "h:s:s0003");
        assert_eq!(human_uid("h:s", "t"), format!("h:s:h:{}", short_hash("t", 10)));
        assert_eq!(SessionUid { harness: "h".into(), raw_id: "s".into() }.to_string(), "h:s");
    }
}
