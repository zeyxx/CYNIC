use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const FLEET_EPOCH_ATTESTATION_SCHEMA: &str = "cynic.consensus.fleet_epoch_attestation.v1";
pub const FLEET_CONSENSUS_STATE_SCHEMA: &str = "cynic.consensus.fleet_consensus_state.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEpochAttestation {
    pub schema: String,
    pub node_id: String,
    pub epoch_sequence: u64,
    pub commit_sha: String,
    pub binary_sha256: String,
    pub registry_digest: String,
    pub monotonic_tick: u64,
    pub signature_hex: String,
    pub attested_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FleetConsensusStatus {
    Synchronized {
        epoch_sequence: u64,
        confirmed_nodes: Vec<String>,
    },
    Transitioning {
        target_epoch: u64,
        progress_ratio: String,
    },
    Partitioned {
        split_epochs: BTreeMap<u64, Vec<String>>,
    },
    DegradedClockSkew {
        max_skew_ms: u64,
        flagged_nodes: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetEpochConsensusState {
    pub schema: String,
    pub current_epoch: u64,
    pub active_commit: String,
    pub status: FleetConsensusStatus,
    pub quorum_size: usize,
    pub participating_nodes: Vec<String>,
    pub attested_nodes: BTreeSet<String>,
    pub evaluated_at_unix_ms: u64,
}
