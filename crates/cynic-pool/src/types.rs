//! Core types for the CYNIC contribution pool.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Agent identity registered in the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub agent_id: String,
    pub role: Role,
    pub capabilities: Vec<String>,
    pub node_id: String,
    pub registered_at_unix_ms: u64,
}

/// Role of an agent (distinct from node identity).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Builder,
    Auditor,
    Scheduler,
    Observer,
    Relay,
    Network,
}

/// Contribution kinds (Proof of Useful Work).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContributionKind {
    BuildProof,
    AuditProof,
    ObservationProof,
    RelayProof,
    NetworkProof,
}

/// A verified contribution from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    pub schema: &'static str,
    pub contribution_id: String,
    pub agent_id: String,
    pub kind: ContributionKind,
    pub proof: ContributionProof,
    pub submitted_at_unix_ms: u64,
    pub nonce: u64,
}

/// Proof payload attached to a contribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionProof {
    pub digest: String,
    pub evidence: Vec<String>,
    pub schema_version: String,
}

/// A validated share (partial contribution in the current epoch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    pub share_id: String,
    pub contribution_id: String,
    pub agent_id: String,
    pub value: u64,
    pub validated_at_unix_ms: u64,
}

/// Proof that a share is included in a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareProof {
    pub share_id: String,
    pub block_hash: String,
    pub merkle_root: String,
    pub merkle_path: Vec<String>,
}

/// Block header (aggregation of shares for an epoch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub block_id: String,
    pub epoch_number: u64,
    pub merkle_root: String,
    pub share_count: usize,
    pub timestamp_unix_ms: u64,
    pub previous_block_hash: Option<String>,
}

/// A block of validated shares.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub shares: Vec<Share>,
    pub rewards: Vec<Reward>,
}

/// Reward entry for a contributor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    pub agent_id: String,
    pub amount: f64,
    pub currency: String,
    pub share_ids: Vec<String>,
}

/// Epoch: a window of N shares before difficulty adjustment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Epoch {
    pub epoch_number: u64,
    pub started_at_unix_ms: u64,
    pub ended_at_unix_ms: Option<u64>,
    pub target_share_count: usize,
    pub difficulty: u64,
    pub share_count: usize,
}

impl Epoch {
    /// Duration since epoch start.
    pub fn elapsed_ms(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now.saturating_sub(self.started_at_unix_ms)
    }

    /// Whether the epoch has reached its target share count.
    pub fn is_complete(&self) -> bool {
        self.share_count >= self.target_share_count
    }
}

/// A contributor's state in the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contributor {
    pub agent_id: String,
    pub role: Role,
    pub total_shares: u64,
    pub total_rewards: f64,
    pub joined_at_unix_ms: u64,
}
