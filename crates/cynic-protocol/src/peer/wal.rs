//! Pure WAL (Write-Ahead Log) models and hash-chain verification for CynicPeer.
//!
//! Enforces the immutable Single Source of Truth (SSOT).
//! Each entry is cryptographically linked:
//!   entry_hash = SHA256(canonical_json(tx) + prev_hash + sequence + timestamp)
//!
//! Pure, zero I/O, deterministic.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::engine::{TransactionReceipt, TransitionError, apply_transaction};
use super::state::PeerState;
use super::transactions::LedgerTransaction;

pub const WAL_ENTRY_SCHEMA: &str = "cynic.peer.wal_entry.v1";
pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// A single immutable entry sealed into the WAL hash-chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalEntry {
    pub schema: String,
    pub sequence: u64,
    pub timestamp_unix_s: u64,
    pub prev_hash: String,
    pub entry_hash: String,
    pub tx: LedgerTransaction,
}

impl WalEntry {
    /// Seal a new transaction into a WalEntry given the preceding hash.
    pub fn seal(prev_hash: &str, tx: LedgerTransaction) -> Self {
        let sequence = tx.sequence;
        let timestamp_unix_s = tx.timestamp_unix_s;
        let entry_hash = calculate_entry_hash(prev_hash, sequence, timestamp_unix_s, &tx);
        Self {
            schema: WAL_ENTRY_SCHEMA.to_string(),
            sequence,
            timestamp_unix_s,
            prev_hash: prev_hash.to_string(),
            entry_hash,
            tx,
        }
    }

    /// Verify whether this entry's hash matches its contents and previous link.
    pub fn verify_integrity(&self, expected_prev_hash: &str) -> bool {
        if self.prev_hash != expected_prev_hash {
            return false;
        }
        let expected_hash = calculate_entry_hash(
            &self.prev_hash,
            self.sequence,
            self.timestamp_unix_s,
            &self.tx,
        );
        self.entry_hash == expected_hash
    }
}

/// Calculate the deterministic SHA-256 hash of a WAL entry.
pub fn calculate_entry_hash(
    prev_hash: &str,
    sequence: u64,
    timestamp_unix_s: u64,
    tx: &LedgerTransaction,
) -> String {
    let tx_canonical = serde_json::to_string(tx).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(sequence.to_le_bytes());
    hasher.update(timestamp_unix_s.to_le_bytes());
    hasher.update(tx_canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalChainError {
    BrokenHashChain {
        sequence: u64,
        expected_prev_hash: String,
        got_prev_hash: String,
    },
    CorruptedEntryHash {
        sequence: u64,
        expected_hash: String,
        got_hash: String,
    },
    SequenceGap {
        expected: u64,
        got: u64,
    },
    TransitionFailed {
        sequence: u64,
        error: TransitionError,
    },
}

impl std::fmt::Display for WalChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BrokenHashChain {
                sequence,
                expected_prev_hash,
                got_prev_hash,
            } => {
                write!(
                    f,
                    "broken hash chain at seq {sequence}: expected prev {expected_prev_hash}, got {got_prev_hash}"
                )
            }
            Self::CorruptedEntryHash {
                sequence,
                expected_hash,
                got_hash,
            } => {
                write!(
                    f,
                    "corrupted entry hash at seq {sequence}: expected {expected_hash}, got {got_hash}"
                )
            }
            Self::SequenceGap { expected, got } => {
                write!(f, "sequence gap: expected {expected}, got {got}")
            }
            Self::TransitionFailed { sequence, error } => {
                write!(f, "transition failed at seq {sequence}: {error}")
            }
        }
    }
}

impl std::error::Error for WalChainError {}

/// Pure verification of an entire slice of WAL entries from genesis or checkpoint.
pub fn verify_wal_chain(entries: &[WalEntry], start_prev_hash: &str) -> Result<(), WalChainError> {
    let mut current_prev_hash = start_prev_hash.to_string();
    let mut expected_seq = if let Some(first) = entries.first() {
        first.sequence
    } else {
        return Ok(());
    };

    for entry in entries {
        if entry.sequence != expected_seq {
            return Err(WalChainError::SequenceGap {
                expected: expected_seq,
                got: entry.sequence,
            });
        }
        if entry.prev_hash != current_prev_hash {
            return Err(WalChainError::BrokenHashChain {
                sequence: entry.sequence,
                expected_prev_hash: current_prev_hash,
                got_prev_hash: entry.prev_hash.clone(),
            });
        }
        let computed_hash = calculate_entry_hash(
            &entry.prev_hash,
            entry.sequence,
            entry.timestamp_unix_s,
            &entry.tx,
        );
        if entry.entry_hash != computed_hash {
            return Err(WalChainError::CorruptedEntryHash {
                sequence: entry.sequence,
                expected_hash: computed_hash,
                got_hash: entry.entry_hash.clone(),
            });
        }
        current_prev_hash = entry.entry_hash.clone();
        expected_seq += 1;
    }

    Ok(())
}

/// Pure replay of WAL entries to deterministically reconstruct the PeerState.
pub fn replay_wal(
    initial_state: &PeerState,
    entries: &[WalEntry],
    start_prev_hash: &str,
) -> Result<(PeerState, Vec<TransactionReceipt>), WalChainError> {
    // 1. Verify chain integrity first
    verify_wal_chain(entries, start_prev_hash)?;

    // 2. Apply transitions sequentially
    let mut current_state = initial_state.clone();
    let mut receipts = Vec::with_capacity(entries.len());

    for entry in entries {
        let (next_state, receipt) =
            apply_transaction(&current_state, &entry.tx).map_err(|error| {
                WalChainError::TransitionFailed {
                    sequence: entry.sequence,
                    error,
                }
            })?;
        current_state = next_state;
        receipts.push(receipt);
    }

    Ok((current_state, receipts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::accounts::*;
    use crate::peer::transactions::*;

    #[test]
    fn seals_and_verifies_hash_chain() {
        let now = 1000;
        let admin = CitizenAccount::new_admin("cit_admin", "Operator", "pub", now);
        let tx1 = LedgerTransaction::new(
            "tx_01",
            1,
            now,
            "cit_admin",
            0,
            LedgerTransactionPayload::CitizenRegistered { citizen: admin },
        );

        let entry1 = WalEntry::seal(GENESIS_PREV_HASH, tx1);
        assert!(entry1.verify_integrity(GENESIS_PREV_HASH));

        let node = NodeAccount {
            schema: NODE_ACCOUNT_SCHEMA.to_string(),
            node_id: "node_01".to_string(),
            owner_citizen_id: "cit_admin".to_string(),
            hostname: "n1".to_string(),
            host_family: "linux".to_string(),
            capabilities: vec![],
            status: NodeMembershipStatus::Enrolled,
            last_liveness_proof_unix_s: None,
        };
        let tx2 = LedgerTransaction::new(
            "tx_02",
            2,
            now + 1,
            "cit_admin",
            1,
            LedgerTransactionPayload::NodeBound { node },
        );
        let entry2 = WalEntry::seal(&entry1.entry_hash, tx2);
        assert!(entry2.verify_integrity(&entry1.entry_hash));

        let entries = vec![entry1, entry2];
        assert!(verify_wal_chain(&entries, GENESIS_PREV_HASH).is_ok());

        // Replay reconstructs state
        let (reconstructed, receipts) =
            replay_wal(&PeerState::new(), &entries, GENESIS_PREV_HASH).expect("replay");
        assert_eq!(receipts.len(), 2);
        assert_eq!(reconstructed.last_sequence, 2);
        assert_eq!(reconstructed.citizens.len(), 1);
        assert_eq!(reconstructed.nodes.len(), 1);
    }

    #[test]
    fn detects_tampered_entry_in_chain() {
        let now = 1000;
        let admin = CitizenAccount::new_admin("cit_admin", "Operator", "pub", now);
        let tx1 = LedgerTransaction::new(
            "tx_01",
            1,
            now,
            "cit_admin",
            0,
            LedgerTransactionPayload::CitizenRegistered { citizen: admin },
        );
        let mut entry1 = WalEntry::seal(GENESIS_PREV_HASH, tx1);

        // Tamper with payload
        entry1.prev_hash =
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();

        let entries = vec![entry1];
        match verify_wal_chain(&entries, GENESIS_PREV_HASH) {
            Err(WalChainError::BrokenHashChain { .. }) => {}
            other => panic!("expected BrokenHashChain, got {other:?}"),
        }
    }
}
