//! CYNIC Storage — sovereign SMR event-sourced WAL and embedded state projections.
//!
//! # Architecture
//!
//! Storage strictly adheres to Foundation §481 and §489:
//! 1. **WAL is Source of Truth** (`INV-01`): The append-only, SHA3-512 chained binary WAL is authoritative.
//! 2. **Projections are Disposable**: Embedded state views (SurrealKV / SQLite) are derived caches.
//! 3. **Zero External Daemon SPOF** (`INV-03`): Storage operates strictly in-process.

pub mod error;
pub mod ports;
pub mod types;

pub use error::StorageError;
pub use ports::{
    DomainProjector, ProjectionCoordinatorPort, StateTransaction, StoragePort, VectorStorePort,
    WalJournalPort,
};
pub use types::{
    CausalWitness, DistanceMetric, Hash512, IntegrityReport, OpaqueEnvelope, PoHTick,
    SequenceNumber, StorageCheckpoint, VectorEmbedding, WalAppendInput, WalEntry, WalReceipt,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_number_increments_monotonically() {
        let seq = SequenceNumber::GENESIS;
        assert_eq!(seq.0, 0);
        assert_eq!(seq.next().0, 1);
    }

    #[test]
    fn hash512_canonical_roundtrip() {
        let raw = [42u8; 64];
        let hash = Hash512(raw);
        let hex_prefixed = hash.to_prefixed_hex();
        assert!(hex_prefixed.starts_with("sha3-512:"));

        let parsed = Hash512::from_prefixed_hex(&hex_prefixed).expect("roundtrip should succeed");
        assert_eq!(parsed, hash);
    }

    #[test]
    fn hash512_rejects_missing_prefix() {
        let invalid = "00".repeat(64);
        assert!(Hash512::from_prefixed_hex(&invalid).is_err());
    }

    #[test]
    fn vector_embedding_supports_dynamic_dimension() {
        let v384 = VectorEmbedding::new("minilm", DistanceMetric::Cosine, vec![0.1; 384]);
        assert_eq!(v384.dimension, 384);

        let v1536 = VectorEmbedding::new("voyage", DistanceMetric::Cosine, vec![0.1; 1536]);
        assert_eq!(v1536.dimension, 1536);
    }
}
