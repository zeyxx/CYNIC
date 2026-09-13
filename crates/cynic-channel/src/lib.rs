//! Pure Layer 2 State Channels & Task HTLCs for CYNIC Multi-Agent Systems
//!
//! References:
//! - RFC-002: CYNIC 5-Layer Decentralized Architecture (Layer 2)
//! - Bitcoin Lightning Network (Poon & Dryja 2016)
//! - Multi-Party Virtual State Channels (Dziembowski et al., EUROCRYPT 2019)
//! - BitVM & Optimistic Computation Verification (Robin Linus 2023)
//!
//! 100% Zero-I/O Domain Core: Pure mathematical state machines, sequencing,
//! and cryptographic commitment verification. Zero sockets, zero disk I/O.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const AGENT_CHANNEL_SCHEMA: &str = "cynic.channel.state.v1";
pub const TASK_HTLC_SCHEMA: &str = "cynic.channel.task_htlc.v1";
pub const SETTLEMENT_PROOF_SCHEMA: &str = "cynic.channel.settlement_proof.v1";

/// Current phase of a bilateral agent state channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelPhase {
    Proposed,
    Open,
    Closing,
    Settled,
    Disputed,
}

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

/// Bilateral State Channel maintaining off-chain execution state between two agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentChannel {
    pub schema: String,
    pub channel_id: String,
    pub initiator_agent: String,
    pub worker_agent: String,
    /// Monotonically increasing sequence number preventing replay attacks
    pub sequence: u64,
    pub phase: ChannelPhase,
    /// Available budget balance for initiator
    pub initiator_balance: u64,
    /// Earned budget balance for worker
    pub worker_balance: u64,
    /// Active HTLC contracts in flight
    pub active_htlcs: Vec<TaskHtlc>,
    /// Revocation hash protecting the prior state against double-spending/rollback
    pub current_revocation_hash: String,
}

impl AgentChannel {
    /// Initialize a new open state channel with initial deposit from initiator.
    pub fn open(
        channel_id: impl Into<String>,
        initiator_agent: impl Into<String>,
        worker_agent: impl Into<String>,
        initial_deposit: u64,
        initial_revocation_hash: impl Into<String>,
    ) -> Self {
        Self {
            schema: AGENT_CHANNEL_SCHEMA.to_string(),
            channel_id: channel_id.into(),
            initiator_agent: initiator_agent.into(),
            worker_agent: worker_agent.into(),
            sequence: 1,
            phase: ChannelPhase::Open,
            initiator_balance: initial_deposit,
            worker_balance: 0,
            active_htlcs: Vec::new(),
            current_revocation_hash: initial_revocation_hash.into(),
        }
    }

    /// Add an active task HTLC, locking the corresponding budget from initiator.
    pub fn lock_task_htlc(
        &mut self,
        htlc: TaskHtlc,
        next_revocation_hash: &str,
    ) -> Result<(), String> {
        if self.phase != ChannelPhase::Open {
            return Err("channel_not_open".to_string());
        }
        if self.initiator_balance < htlc.allocated_budget {
            return Err("insufficient_channel_balance".to_string());
        }

        self.initiator_balance -= htlc.allocated_budget;
        self.active_htlcs.push(htlc);
        self.sequence += 1;
        self.current_revocation_hash = next_revocation_hash.to_string();
        Ok(())
    }

    /// Settle a fulfilled HTLC into worker's balance upon receiving valid preimage.
    pub fn settle_task_htlc(
        &mut self,
        htlc_id: &str,
        preimage: &str,
        current_time_ms: u64,
        next_revocation_hash: &str,
    ) -> Result<(), String> {
        if self.phase != ChannelPhase::Open {
            return Err("channel_not_open".to_string());
        }

        let pos = self
            .active_htlcs
            .iter()
            .position(|h| h.htlc_id == htlc_id)
            .ok_or_else(|| "htlc_not_found".to_string())?;

        let mut htlc = self.active_htlcs.remove(pos);
        htlc.fulfill(preimage, current_time_ms)?;

        self.worker_balance += htlc.allocated_budget;
        self.sequence += 1;
        self.current_revocation_hash = next_revocation_hash.to_string();
        Ok(())
    }

    /// Generate an immutable settlement proof to close the channel onto Layer 1.
    pub fn generate_settlement_proof(&self) -> Result<ChannelSettlementProof, String> {
        if !self.active_htlcs.is_empty() {
            return Err("cannot_settle_with_active_htlcs_in_flight".to_string());
        }

        let mut hasher = Sha256::new();
        hasher.update(self.channel_id.as_bytes());
        hasher.update(self.sequence.to_le_bytes());
        hasher.update(self.initiator_balance.to_le_bytes());
        hasher.update(self.worker_balance.to_le_bytes());
        let digest = hex::encode(hasher.finalize());

        Ok(ChannelSettlementProof {
            schema: SETTLEMENT_PROOF_SCHEMA.to_string(),
            channel_id: self.channel_id.clone(),
            final_sequence: self.sequence,
            initiator_balance: self.initiator_balance,
            worker_balance: self.worker_balance,
            settlement_digest: digest,
        })
    }
}

/// Final Layer 1 Settlement Proof submitted to close a Layer 2 channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelSettlementProof {
    pub schema: String,
    pub channel_id: String,
    pub final_sequence: u64,
    pub initiator_balance: u64,
    pub worker_balance: u64,
    pub settlement_digest: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn htlc_lifecycle_success() {
        let preimage = "build_proof_commit_05ecc253457a445d";
        let hash_lock = hex::encode(Sha256::digest(preimage.as_bytes()));

        let mut htlc = TaskHtlc::new("task-01", hash_lock, 100, 5000);
        assert!(!htlc.is_fulfilled());

        // Fulfill with correct preimage before deadline
        assert!(htlc.fulfill(preimage, 2500).is_ok());
        assert!(htlc.is_fulfilled());
        assert_eq!(htlc.revealed_preimage.as_deref(), Some(preimage));
    }

    #[test]
    fn channel_execution_and_settlement() {
        let mut channel = AgentChannel::open(
            "chan-hermes-builder-01",
            "agent:hermes",
            "worker:builder",
            1000,
            "rev_hash_0",
        );

        let preimage = "cargo_check_ok_evidence_sha256";
        let hash_lock = hex::encode(Sha256::digest(preimage.as_bytes()));

        let htlc = TaskHtlc::new("htlc-build-01", hash_lock, 250, 10_000);
        channel.lock_task_htlc(htlc, "rev_hash_1").expect("lock ok");

        assert_eq!(channel.initiator_balance, 750);
        assert_eq!(channel.worker_balance, 0);
        assert_eq!(channel.sequence, 2);

        // Worker provides proof before deadline
        channel
            .settle_task_htlc("htlc-build-01", preimage, 5000, "rev_hash_2")
            .expect("settle ok");

        assert_eq!(channel.initiator_balance, 750);
        assert_eq!(channel.worker_balance, 250);
        assert_eq!(channel.sequence, 3);

        // Generate Layer 1 settlement proof
        let proof = channel
            .generate_settlement_proof()
            .expect("settlement proof generated");
        assert_eq!(proof.final_sequence, 3);
        assert_eq!(proof.worker_balance, 250);
    }
}
