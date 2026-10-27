//! Unified append-only journal port — single SSOT for all WAL implementations.
//!
//! Every append-only log in CYNIC (storage WAL, evidence journal, window
//! journal, peer WAL, settlement WAL) implements this trait. The trait is
//! generic over the entry type E so each domain keeps its own schema.
//!
//! Pure contract only: trait definition. I/O stays in adapters.

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Append-only immutable log port.
///
/// Generic over entry type E. All entries must be serializable and
/// deserializable (for persistence and replay).
pub trait AppendOnlyJournal<E>: Send + Sync
where
    E: Serialize + DeserializeOwned + Clone + Send + Sync,
{
    /// Append a single entry. Returns the sequence number assigned.
    fn append(&mut self, entry: E) -> Result<u64, String>;

    /// Read all entries in order.
    fn read_all(&self) -> Result<Vec<E>, String>;

    /// Verify the integrity of the chain (hash chain, sequence monotonicity).
    /// Returns Ok(()) if valid, Err with reason otherwise.
    fn verify(&self) -> Result<(), String>;

    /// Number of entries in the journal.
    fn len(&self) -> usize;

    /// Whether the journal is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
