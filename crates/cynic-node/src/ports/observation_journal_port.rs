use cynic_protocol::HealthSignal;
use std::error::Error;

/// Port: Node-local append-only Write-Ahead Log (WAL) for observations and signals.
/// Provides immediate, partition-tolerant local persistence before remote synchronization.
pub trait ObservationJournalPort: Send + Sync {
    /// Append an observation/signal to the local write-ahead log (append-only JSONL).
    fn append_signal(&self, signal: &HealthSignal) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Read uncommitted signals from the local write-ahead log.
    fn read_signals(&self, limit: usize)
    -> Result<Vec<HealthSignal>, Box<dyn Error + Send + Sync>>;

    /// Rotate the journal if its size exceeds `max_bytes`.
    fn rotate_journal(&self, max_bytes: u64) -> Result<bool, Box<dyn Error + Send + Sync>>;
}
