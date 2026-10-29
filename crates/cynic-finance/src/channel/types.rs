use serde::{Deserialize, Serialize};

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
