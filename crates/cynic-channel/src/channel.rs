use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::htlc::TaskHtlc;
use crate::types::{
    AGENT_CHANNEL_SCHEMA, ChannelPhase, ChannelSettlementProof, SETTLEMENT_PROOF_SCHEMA,
};

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
