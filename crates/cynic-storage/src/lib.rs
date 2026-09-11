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
pub mod wal;

pub use error::StorageError;
pub use ports::{
    DomainProjector, ProjectionCoordinatorPort, StateTransaction, StoragePort, VectorStorePort,
    WalJournalPort,
};
pub use types::{
    AxisToken, CausalWitness, CompactMetadataTensor, DistanceMetric, Hash512, IntegrityReport,
    MetadataTensor, OpaqueEnvelope, PoHTick, SequenceNumber, StorageCheckpoint, VectorEmbedding,
    WalAppendInput, WalEntry, WalReceipt,
};
pub use wal::WalFsAdapter;

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

    #[test]
    fn metadata_tensor_extensibility() {
        let mut tensor = MetadataTensor::new("cynic.test.v1", Hash512::GENESIS);
        tensor
            .continuous_axes
            .insert("metabolic.joules".to_string(), 42.5);
        tensor
            .discrete_axes
            .insert("agent.role".to_string(), "architect".to_string());
        tensor
            .residual_unknowns
            .push("thermal_dissipation".to_string());

        assert_eq!(tensor.continuous_axes.get("metabolic.joules"), Some(&42.5));
        assert_eq!(
            tensor.discrete_axes.get("agent.role").map(String::as_str),
            Some("architect")
        );
        assert_eq!(tensor.residual_unknowns.len(), 1);
    }

    #[test]
    fn compact_metadata_tensor_stack_inlined() {
        let mut tensor = CompactMetadataTensor::new(1, Hash512::GENESIS);
        tensor.insert_continuous(0x0001, 14.2);
        tensor.insert_continuous(0x0002, 0.95);
        tensor.discrete_keys.push(0x0010);
        tensor.discrete_vals.push("crystallization".to_string());

        assert_eq!(tensor.continuous_keys.len(), 2);
        assert_eq!(tensor.continuous_vals[0], 14.2);
        assert_eq!(tensor.discrete_vals[0], "crystallization");
    }
}
