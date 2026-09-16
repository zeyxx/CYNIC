use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A Hash Time-Locked Contract (HTLC) for task execution delegation.
/// The worker receives reward/budget settlement if and only if they provide
/// the preimage revealing the deterministic execution proof before the timelock expires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskHtlc {
    pub htlc_id: String,
    /// SHA-256 hash lock of the expected execution proof/preimage
    pub proof_hash_lock: String,
    /// Allocated execution budget (in abstract tokens/energy units)
    pub allocated_budget: u64,
    /// Monotonic deadline in milliseconds after which the task times out and refunds
    pub expires_at_unix_ms: u64,
    /// Settled preimage once revealed by the executing worker
    pub revealed_preimage: Option<String>,
}

impl TaskHtlc {
    /// Create a new pending task HTLC bound by a proof hash lock.
    pub fn new(
        htlc_id: impl Into<String>,
        proof_hash_lock: impl Into<String>,
        allocated_budget: u64,
        expires_at_unix_ms: u64,
    ) -> Self {
        Self {
            htlc_id: htlc_id.into(),
            proof_hash_lock: proof_hash_lock.into(),
            allocated_budget,
            expires_at_unix_ms,
            revealed_preimage: None,
        }
    }

    /// Pure verification and settlement of an HTLC via preimage revelation.
    pub fn fulfill(&mut self, preimage: &str, current_time_ms: u64) -> Result<(), String> {
        if current_time_ms > self.expires_at_unix_ms {
            return Err("htlc_expired_before_fulfillment".to_string());
        }

        let computed_hash = hex::encode(Sha256::digest(preimage.as_bytes()));
        if computed_hash != self.proof_hash_lock {
            return Err("invalid_preimage_hash_mismatch".to_string());
        }

        self.revealed_preimage = Some(preimage.to_string());
        Ok(())
    }

    /// Checks whether this HTLC is already resolved.
    pub fn is_fulfilled(&self) -> bool {
        self.revealed_preimage.is_some()
    }
}
