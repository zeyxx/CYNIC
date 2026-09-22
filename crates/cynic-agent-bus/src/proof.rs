use sha2::{Digest, Sha256};

use crate::types::Hash32;

/// Compute SHA-256 hash of content, returning the raw 32 bytes.
pub fn sha256_bytes(content: &[u8]) -> Hash32 {
    let mut hasher = Sha256::new();
    hasher.update(content);
    Hash32(hasher.finalize().into())
}

/// Compute SHA-256 hash of a string, returning hex string with `sha256:` prefix.
pub fn sha256_hex(content: &str) -> String {
    let hash = sha256_bytes(content.as_bytes());
    format!("sha256:{}", hex::encode(hash.0))
}

/// Compute SHA-256 of a git commit diff (for proof linking).
pub fn diff_hash(diff: &str) -> Hash32 {
    sha256_bytes(diff.as_bytes())
}

/// Verify that a proof's output hash matches the expected content.
pub fn verify_proof(proof: &super::types::Proof, expected_output: &str) -> bool {
    let actual = sha256_bytes(expected_output.as_bytes());
    proof.output_hash == actual
}

/// Compute a handoff proof hash linking parent and child commits.
pub fn handoff_hash(parent_commit: &Hash32, child_commit: &Hash32) -> Hash32 {
    let mut hasher = Sha256::new();
    hasher.update(b"cynic.handoff.v1");
    hasher.update(parent_commit.0);
    hasher.update(child_commit.0);
    Hash32(hasher.finalize().into())
}
