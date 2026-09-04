//! Deterministic in-memory state of a CynicPeer.
//!
//! Holds the materialized state of all 7 canonical accounts, active workloads,
//! and sequence tracking.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::accounts::{
    AgentAccount, CitizenAccount, MandateAccount, NodeAccount, OfficeAccount, ResourceAccount,
    TreasuryAccount,
};

pub const PEER_STATE_SCHEMA: &str = "cynic.peer.state.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadExecutionPhase {
    Admitted,
    Started,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmittedWorkload {
    pub workload_id: String,
    pub mandate_id: String,
    pub target_node_id: String,
    pub action: String,
    pub phase: WorkloadExecutionPhase,
    pub admitted_at_unix_s: u64,
    pub started_at_unix_s: Option<u64>,
    pub completed_at_unix_s: Option<u64>,
    pub cpu_cores_reserved: u32,
    pub ram_bytes_reserved: u64,
    pub actions_used: u64,
    pub wall_seconds_used: u64,
    pub success: Option<bool>,
    pub proof_digest: Option<String>,
}

/// In-memory materialized state of a CynicPeer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerState {
    pub schema: String,
    pub epoch: u64,
    pub last_sequence: u64,
    pub last_tx_hash: String,
    pub citizens: BTreeMap<String, CitizenAccount>,
    pub nodes: BTreeMap<String, NodeAccount>,
    pub agents: BTreeMap<String, AgentAccount>,
    pub offices: BTreeMap<String, OfficeAccount>,
    pub mandates: BTreeMap<String, MandateAccount>,
    pub resources: BTreeMap<String, ResourceAccount>,
    pub treasury: BTreeMap<String, TreasuryAccount>,
    pub workloads: BTreeMap<String, AdmittedWorkload>,
}

impl Default for PeerState {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerState {
    pub fn new() -> Self {
        Self {
            schema: PEER_STATE_SCHEMA.to_string(),
            epoch: 0,
            last_sequence: 0,
            last_tx_hash: "0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            citizens: BTreeMap::new(),
            nodes: BTreeMap::new(),
            agents: BTreeMap::new(),
            offices: BTreeMap::new(),
            mandates: BTreeMap::new(),
            resources: BTreeMap::new(),
            treasury: BTreeMap::new(),
            workloads: BTreeMap::new(),
        }
    }

    /// Compute deterministic SHA-256 digest of canonical JSON state.
    pub fn state_root_digest(&self) -> String {
        use sha2::{Digest, Sha256};
        let canonical_json = serde_json::to_string(self).unwrap_or_default();
        let hash = Sha256::digest(canonical_json.as_bytes());
        format!("{:x}", hash)
    }
}
