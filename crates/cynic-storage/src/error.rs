//! Storage domain errors — zero-leak error definitions for cynic-storage.

use crate::types::{Hash512, SequenceNumber};

/// Sovereign storage error enumeration.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("WAL sequence mismatch: expected {expected:?}, got {actual:?}")]
    SequenceMismatch {
        expected: SequenceNumber,
        actual: SequenceNumber,
    },

    #[error(
        "Cryptographic hash chain broken at sequence {sequence:?}: expected {expected_prev:?}, got {actual_prev:?}"
    )]
    HashChainCorrupt {
        sequence: SequenceNumber,
        expected_prev: Hash512,
        actual_prev: Hash512,
    },

    #[error("Causal witness verification failed: {0}")]
    UnadmittedCausalWitness(String),

    #[error("Storage I/O failure: {0}")]
    Io(String),

    #[error("Serialization / Codec error: {0}")]
    Codec(String),

    #[error("Entity not found: {schema} id={id}")]
    NotFound { schema: String, id: String },

    #[error("Projection failure at sequence {sequence:?} for schema {schema}: {cause}")]
    ProjectionFailed {
        sequence: SequenceNumber,
        schema: String,
        cause: String,
    },

    #[error("Storage lock contention: {0}")]
    LockContention(String),

    #[error("Substrate disk pressure limit reached (>90% full)")]
    SubstrateDiskPressure,

    #[error("Corrupted ledger at sequence {sequence:?}: {detail}")]
    CorruptedLedgerAtSequence {
        sequence: SequenceNumber,
        detail: String,
    },
}
