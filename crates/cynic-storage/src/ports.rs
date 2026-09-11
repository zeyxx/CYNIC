//! Storage domain ports — abstract interfaces for the append-only WAL, SMR projectors, and vector store.

use crate::error::StorageError;
use crate::types::{
    IntegrityReport, SequenceNumber, StorageCheckpoint, WalAppendInput, WalEntry, WalReceipt,
};

/// Append-only immutable log port.
pub trait WalJournalPort: Send + Sync {
    /// Atomic batch append with single group-commit durability boundary.
    fn append_batch(&self, events: &[WalAppendInput]) -> Result<Vec<WalReceipt>, StorageError>;

    /// Streaming iterator yielding entries without unbounded heap allocation.
    fn scan_stream<'a>(
        &'a self,
        from_seq: SequenceNumber,
        to_seq: Option<SequenceNumber>,
    ) -> Result<Box<dyn Iterator<Item = Result<WalEntry, StorageError>> + 'a>, StorageError>;

    /// Read the latest sequence number written to the log.
    fn latest_sequence(&self) -> Result<SequenceNumber, StorageError>;

    /// Verify the complete cryptographic hash chain from Genesis.
    fn verify_integrity(&self) -> Result<IntegrityReport, StorageError>;
}

/// Abstract state transaction boundary for projecting views into embedded stores.
pub trait StateTransaction {
    fn execute_statement(&mut self, statement: &str) -> Result<(), StorageError>;
    fn execute_sql(
        &mut self,
        sql: &str,
        params: &[&(dyn rusqlite::ToSql + Sync)],
    ) -> Result<usize, StorageError>;
}

/// Bounded-context projection reducer.
/// Domains implement this trait to project their private views into SQLite or SurrealKV.
pub trait DomainProjector: Send + Sync {
    /// Return the list of schema URIs this projector is registered to reduce.
    fn supported_schemas(&self) -> &[&str];

    /// Apply a single immutable WAL entry into the active projection transaction.
    fn apply(&self, entry: &WalEntry, tx: &mut dyn StateTransaction) -> Result<(), StorageError>;
}

/// SMR Replay and Projection Coordinator port.
pub trait ProjectionCoordinatorPort: Send + Sync {
    /// Register a domain projector for handling specific schemas.
    fn register_projector(
        &mut self,
        projector: Box<dyn DomainProjector>,
    ) -> Result<(), StorageError>;

    /// Project a single WAL entry across all registered domain projectors.
    fn project_entry(&self, entry: &WalEntry) -> Result<(), StorageError>;

    /// Replay all WAL entries from Genesis through all registered projectors.
    fn replay_all(&self) -> Result<u64, StorageError>;

    /// Return the sequence number of the most recently projected event.
    fn last_projected_sequence(&self) -> Result<SequenceNumber, StorageError>;
}

/// Embedded vector search port for cognitive crystals and semantic retrieval.
pub trait VectorStorePort: Send + Sync {
    /// Insert or update a vector embedding by identifier.
    fn insert_vector(&self, id: &str, vector: &[f32]) -> Result<(), StorageError>;

    /// Search for the top-k nearest vectors using cosine similarity.
    fn search_nearest(
        &self,
        query: &[f32],
        k: usize,
        threshold: f32,
    ) -> Result<Vec<(String, f32)>, StorageError>;

    /// Remove a vector embedding by identifier.
    fn delete_vector(&self, id: &str) -> Result<(), StorageError>;
}

/// Master unified storage port exposed to runtime engines.
pub trait StoragePort: Send + Sync {
    fn wal(&self) -> &dyn WalJournalPort;
    fn projections(&self) -> &dyn ProjectionCoordinatorPort;
    fn vectors(&self) -> &dyn VectorStorePort;
    fn checkpoint(&self) -> Result<StorageCheckpoint, StorageError>;
}
