//! Unified hash-chain primitive — single SSOT for all WAL hash chains.
//!
//! Every hash chain in CYNIC follows the same pattern:
//! - Each entry has content that gets hashed
//! - Each entry includes the previous entry's hash
//! - The hash uses the canonical SHA-256 digest law (Solana-aligned,
//!   32-byte digests)
//!
//! This module provides the common seal/verify logic; each domain
//! provides its own [`ChainLink`] implementation.
//!
//! Migrated implementations:
//! - `cynic-storage` binary WAL frames
//! - `cynic-consensus` peer WAL entries
//! - `cynic-protocol` evidence journal
//! - `cynic-evidence` append-only log
//! - `cynic-lifecycle` kernel release proof
//!
//! **Intentional clean break (T13)**: all chains now use SHA-256 with
//! `genesis` as the genesis prev-hash. Old SHA3-512 entries do NOT
//! verify under the new code — no migration path exists. Pre-T10
//! SHA-256 entries do NOT verify either (genesis changed from 64-zero
//! hex / empty string to `genesis`, and hashes are now `sha256:`-prefixed).

use crate::digest::digest_bytes;

/// A link in a hash chain.
///
/// Each domain implements this trait to define its own canonical
/// serialization and provide the hash/prev_hash fields.
pub trait ChainLink {
    /// Canonical bytes hashed into `entry_hash`.
    fn chain_input(&self) -> Vec<u8>;
    /// This entry's hash (prefixed, canonical law).
    fn entry_hash(&self) -> &str;
    /// Previous entry's hash.
    fn prev_hash(&self) -> &str;
}

/// Unified hash-chain operations.
///
/// All hash chains in CYNIC use SHA-256 as the digest law (Solana-aligned,
/// 32-byte digests). The `seal` function computes the canonical hash for
/// a new link; `verify` checks an entire chain from a genesis hash.
pub struct HashChain;

impl HashChain {
    /// Compute the canonical entry hash for a link.
    ///
    /// The hash covers the link's full [`ChainLink::chain_input`],
    /// which must include the predecessor's hash.
    pub fn seal<L: ChainLink>(link: &L) -> String {
        digest_bytes(&link.chain_input())
    }

    /// Verify a chain of links from a given genesis hash.
    ///
    /// Checks that each link's `prev_hash` matches the previous link's
    /// `entry_hash`, and that each `entry_hash` matches the recomputed
    /// hash of its `chain_input`.
    pub fn verify<L: ChainLink>(genesis: &str, links: &[L]) -> Result<(), ChainError> {
        let mut expected_prev = genesis.to_string();
        for link in links {
            if link.prev_hash() != expected_prev {
                return Err(ChainError::BrokenChain {
                    expected_prev: expected_prev.clone(),
                    got_prev: link.prev_hash().to_string(),
                });
            }
            let computed = Self::seal(link);
            if link.entry_hash() != computed {
                return Err(ChainError::CorruptedEntry {
                    expected_hash: computed,
                    got_hash: link.entry_hash().to_string(),
                });
            }
            expected_prev = link.entry_hash().to_string();
        }
        Ok(())
    }
}

/// Errors produced by hash-chain verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// The previous hash does not match the expected value.
    BrokenChain {
        expected_prev: String,
        got_prev: String,
    },
    /// The entry hash does not match the recomputed hash.
    CorruptedEntry {
        expected_hash: String,
        got_hash: String,
    },
}

impl std::fmt::Display for ChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BrokenChain {
                expected_prev,
                got_prev,
            } => write!(
                f,
                "broken chain: expected prev {expected_prev}, got {got_prev}"
            ),
            Self::CorruptedEntry {
                expected_hash,
                got_hash,
            } => write!(
                f,
                "corrupted entry: expected hash {expected_hash}, got {got_hash}"
            ),
        }
    }
}

impl std::error::Error for ChainError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::digest::SHA2_PREFIX;

    /// A minimal test link for unit tests.
    struct TestLink {
        chain_input: Vec<u8>,
        entry_hash: String,
        prev_hash: String,
    }

    impl ChainLink for TestLink {
        fn chain_input(&self) -> Vec<u8> {
            self.chain_input.clone()
        }
        fn entry_hash(&self) -> &str {
            &self.entry_hash
        }
        fn prev_hash(&self) -> &str {
            &self.prev_hash
        }
    }

    fn make_link(seq: u64, prev: &str) -> TestLink {
        let input = format!("{prev}:{seq}");
        let hash = digest_bytes(input.as_bytes());
        TestLink {
            chain_input: input.into_bytes(),
            entry_hash: hash,
            prev_hash: prev.to_string(),
        }
    }

    #[test]
    fn seal_produces_prefixed_sha256() {
        let link = make_link(1, "genesis");
        let sealed = HashChain::seal(&link);
        assert!(sealed.starts_with(SHA2_PREFIX));
        assert_eq!(sealed.len(), SHA2_PREFIX.len() + 64);
        assert_eq!(sealed, link.entry_hash);
    }

    #[test]
    fn verify_accepts_valid_chain() {
        let genesis = "genesis";
        let l1 = make_link(1, genesis);
        let l2 = make_link(2, &l1.entry_hash);
        let l3 = make_link(3, &l2.entry_hash);
        assert!(HashChain::verify(genesis, &[l1, l2, l3]).is_ok());
    }

    #[test]
    fn verify_detects_broken_prev_hash() {
        let genesis = "genesis";
        let l1 = make_link(1, genesis);
        // l2 points to wrong prev
        let l2 = make_link(2, "wrong_prev");
        match HashChain::verify(genesis, &[l1, l2]) {
            Err(ChainError::BrokenChain { .. }) => {}
            other => panic!("expected BrokenChain, got {other:?}"),
        }
    }

    #[test]
    fn verify_detects_corrupted_entry_hash() {
        let genesis = "genesis";
        let mut l1 = make_link(1, genesis);
        // Tamper: change the stored hash but keep the chain input
        l1.entry_hash = format!("{}{}", SHA2_PREFIX, "ab".repeat(32));
        match HashChain::verify(genesis, &[l1]) {
            Err(ChainError::CorruptedEntry { .. }) => {}
            other => panic!("expected CorruptedEntry, got {other:?}"),
        }
    }

    #[test]
    fn verify_accepts_empty_chain() {
        let links: Vec<TestLink> = vec![];
        assert!(HashChain::verify("genesis", &links).is_ok());
    }

    #[test]
    fn verify_detects_tampered_chain_input() {
        let genesis = "genesis";
        let mut l1 = make_link(1, genesis);
        // Tamper: change the chain input after sealing
        l1.chain_input = b"tampered".to_vec();
        match HashChain::verify(genesis, &[l1]) {
            Err(ChainError::CorruptedEntry { .. }) => {}
            other => panic!("expected CorruptedEntry, got {other:?}"),
        }
    }
}
