//! Settlement WAL — wraps the cynic-storage `WalFsAdapter` for scheduler settlements.
//!
//! Every settlement appends one hash-chained entry to the global node WAL.
//! The WAL is the audit source; sidecars stay as read models.
//!
//! Ordering: append to WAL first, then settle the workload store. Append
//! failure returns `Err` — the workload stays Claimed (retryable), never
//! half-settled.

use std::path::Path;

use cynic_protocol::AgentCompletionStatus;
use cynic_storage::{CausalWitness, StorageError, WalAppendInput, WalFsAdapter, WalJournalPort};
use tokio_util::bytes::Bytes;

/// Schema for scheduler settlement entries in the global WAL.
pub const SCHEDULER_SETTLEMENT_SCHEMA: &str = "cynic.scheduler.settlement.v1";

/// Default witness for settlement WAL entries.
///
/// In production, this would be the node's signing key. For now, a
/// deterministic witness is sufficient — the WAL chain integrity is
/// verified by hash continuity, not signature validity.
fn settlement_witness() -> CausalWitness {
    CausalWitness {
        principal_pubkey: [7u8; 32],
        signature: [14u8; 64],
        mandate_id: Some("build-scheduler".to_string()),
        proof_anchor: None,
    }
}

/// Settlement WAL — thin wrapper over `WalFsAdapter` for audit entries.
#[derive(Clone)]
pub struct SettlementWal {
    inner: WalFsAdapter,
}

impl SettlementWal {
    /// Open (or create) the settlement WAL at the given directory.
    ///
    /// Crash-recovery is handled by `WalFsAdapter::open()`: torn tails at
    /// EOF are truncated, mid-log corruption fails closed.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, StorageError> {
        let inner = WalFsAdapter::open(dir)?;
        Ok(Self { inner })
    }

    /// Append one settlement entry to the global WAL.
    ///
    /// The entry captures the settlement transition: workload_id, status,
    /// workload_digest, evidence refs, and timing. The payload is JSON;
    /// the schema is `cynic.scheduler.settlement.v1`.
    ///
    /// Returns the WAL receipt on success. Failure returns `Err` — the
    /// caller must leave the workload Claimed (retryable).
    pub fn append_settlement(
        &self,
        node_id: &str,
        workload_id: &str,
        status: &AgentCompletionStatus,
        workload_digest: &str,
        evidence_refs: &[String],
        settled_at_ms: u64,
    ) -> Result<cynic_storage::WalReceipt, StorageError> {
        let status_str = format!("{status:?}").to_lowercase();
        let payload = serde_json::json!({
            "node_id": node_id,
            "workload_id": workload_id,
            "status": status_str,
            "workload_digest": workload_digest,
            "evidence_refs": evidence_refs,
            "settled_at_ms": settled_at_ms,
        });
        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let input = WalAppendInput {
            schema: SCHEDULER_SETTLEMENT_SCHEMA.to_string(),
            payload: Bytes::from(payload_bytes),
            witness: settlement_witness(),
        };
        let receipts = self.inner.append_batch(&[input])?;
        Ok(receipts.into_iter().next().expect("one receipt"))
    }

    /// Verify the full WAL chain from Genesis.
    ///
    /// Returns the integrity report. `is_valid` is true iff every entry
    /// hashes correctly and the chain is unbroken.
    pub fn verify_integrity(&self) -> Result<cynic_storage::IntegrityReport, StorageError> {
        self.inner.verify_integrity()
    }

    /// Read the latest sequence number in the WAL.
    pub fn latest_sequence(&self) -> Result<cynic_storage::SequenceNumber, StorageError> {
        self.inner.latest_sequence()
    }
}
