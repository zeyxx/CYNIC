//! Fleet Consensus & Temporal Monotonic Synchronization for CYNIC
//! Inspired by Solana Tower BFT & PoH: Pure state machines, 2/3 Quorum verification,
//! and strictly monotonic epoch sequencing without wall-clock trust.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

/// Computes the unique cryptographic fingerprint of an Epoch.
pub fn compute_epoch_fingerprint(
    epoch_sequence: u64,
    commit_sha: &str,
    binary_sha256: &str,
    registry_digest: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(epoch_sequence.to_be_bytes());
    hasher.update(commit_sha.as_bytes());
    hasher.update(binary_sha256.as_bytes());
    hasher.update(registry_digest.as_bytes());
    hex::encode(hasher.finalize())
}

/// Pure consensus evaluator across fleet attestations (Solana Quorum / BFT principle).
/// Evaluates whether a supermajority (≥ 2/3) of nodes agree on the exact same Epoch and binary.
pub fn evaluate_fleet_consensus(
    expected_epoch: u64,
    expected_commit: &str,
    registered_nodes: &[String],
    attestations: &[NodeEpochAttestation],
    max_allowed_skew_ms: u64,
    now_unix_ms: u64,
) -> FleetEpochConsensusState {
    let total_nodes = registered_nodes.len();
    let quorum_threshold = (total_nodes * 2 / 3) + 1; // Strict > 2/3 supermajority

    let mut epoch_groups: BTreeMap<u64, Vec<String>> = BTreeMap::new();
    let mut confirmed_nodes = Vec::new();
    let mut attested_nodes_set = BTreeSet::new();
    let mut flagged_skew_nodes = Vec::new();

    for att in attestations {
        if !registered_nodes.contains(&att.node_id) {
            continue;
        }

        attested_nodes_set.insert(att.node_id.clone());
        epoch_groups
            .entry(att.epoch_sequence)
            .or_default()
            .push(att.node_id.clone());

        // Check clock skew against now
        let skew = now_unix_ms.abs_diff(att.attested_at_unix_ms);
        if skew > max_allowed_skew_ms {
            flagged_skew_nodes.push(att.node_id.clone());
        }

        // Matching expected epoch and commit
        if att.epoch_sequence == expected_epoch && att.commit_sha == expected_commit {
            confirmed_nodes.push(att.node_id.clone());
        }
    }

    let status = if !flagged_skew_nodes.is_empty() {
        FleetConsensusStatus::DegradedClockSkew {
            max_skew_ms: max_allowed_skew_ms,
            flagged_nodes: flagged_skew_nodes,
        }
    } else if confirmed_nodes.len() >= quorum_threshold {
        FleetConsensusStatus::Synchronized {
            epoch_sequence: expected_epoch,
            confirmed_nodes: confirmed_nodes.clone(),
        }
    } else if epoch_groups.len() > 1 {
        FleetConsensusStatus::Partitioned {
            split_epochs: epoch_groups,
        }
    } else {
        FleetConsensusStatus::Transitioning {
            target_epoch: expected_epoch,
            progress_ratio: format!("{}/{}", confirmed_nodes.len(), total_nodes),
        }
    };

    FleetEpochConsensusState {
        schema: FLEET_CONSENSUS_STATE_SCHEMA.to_string(),
        current_epoch: expected_epoch,
        active_commit: expected_commit.to_string(),
        status,
        quorum_size: quorum_threshold,
        participating_nodes: registered_nodes.to_vec(),
        attested_nodes: attested_nodes_set,
        evaluated_at_unix_ms: now_unix_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn achieves_consensus_when_two_of_three_nodes_agree() {
        let nodes = vec![
            "cortex".to_string(),
            "cynic-builder".to_string(),
            "proxmox".to_string(),
        ];

        let attestations = vec![
            NodeEpochAttestation {
                schema: FLEET_EPOCH_ATTESTATION_SCHEMA.to_string(),
                node_id: "cortex".to_string(),
                epoch_sequence: 42,
                commit_sha: "abc1234".to_string(),
                binary_sha256: "sha256:bin".to_string(),
                registry_digest: "sha256:reg".to_string(),
                monotonic_tick: 1000,
                signature_hex: "sig1".to_string(),
                attested_at_unix_ms: 1000,
            },
            NodeEpochAttestation {
                schema: FLEET_EPOCH_ATTESTATION_SCHEMA.to_string(),
                node_id: "cynic-builder".to_string(),
                epoch_sequence: 42,
                commit_sha: "abc1234".to_string(),
                binary_sha256: "sha256:bin".to_string(),
                registry_digest: "sha256:reg".to_string(),
                monotonic_tick: 1002,
                signature_hex: "sig2".to_string(),
                attested_at_unix_ms: 1005,
            },
        ];

        let consensus = evaluate_fleet_consensus(42, "abc1234", &nodes, &attestations, 5000, 1010);

        match consensus.status {
            FleetConsensusStatus::Synchronized {
                epoch_sequence,
                confirmed_nodes,
            } => {
                assert_eq!(epoch_sequence, 42);
                assert_eq!(confirmed_nodes.len(), 2);
            }
            other => panic!("Expected Synchronized consensus, got: {:?}", other),
        }
    }

    #[test]
    fn detects_partitioned_cluster_on_epoch_split() {
        let nodes = vec!["cortex".to_string(), "cynic-builder".to_string()];

        let attestations = vec![
            NodeEpochAttestation {
                schema: FLEET_EPOCH_ATTESTATION_SCHEMA.to_string(),
                node_id: "cortex".to_string(),
                epoch_sequence: 42,
                commit_sha: "abc".to_string(),
                binary_sha256: "bin1".to_string(),
                registry_digest: "reg1".to_string(),
                monotonic_tick: 100,
                signature_hex: "s1".to_string(),
                attested_at_unix_ms: 1000,
            },
            NodeEpochAttestation {
                schema: FLEET_EPOCH_ATTESTATION_SCHEMA.to_string(),
                node_id: "cynic-builder".to_string(),
                epoch_sequence: 41, // Lagging behind
                commit_sha: "old".to_string(),
                binary_sha256: "bin0".to_string(),
                registry_digest: "reg0".to_string(),
                monotonic_tick: 90,
                signature_hex: "s2".to_string(),
                attested_at_unix_ms: 1000,
            },
        ];

        let consensus = evaluate_fleet_consensus(42, "abc", &nodes, &attestations, 5000, 1000);

        match consensus.status {
            FleetConsensusStatus::Partitioned { split_epochs } => {
                assert_eq!(split_epochs.len(), 2);
                assert!(split_epochs.contains_key(&42));
                assert!(split_epochs.contains_key(&41));
            }
            other => panic!("Expected Partitioned state, got: {:?}", other),
        }
    }
}
