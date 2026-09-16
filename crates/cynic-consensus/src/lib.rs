//! Fleet Consensus & Temporal Monotonic Synchronization for CYNIC
//! Inspired by Solana Tower BFT & PoH: Pure state machines, 2/3 Quorum verification,
//! and strictly monotonic epoch sequencing without wall-clock trust.

pub mod evaluator;
pub mod types;

pub use evaluator::{compute_epoch_fingerprint, evaluate_fleet_consensus};
pub use types::{
    FLEET_CONSENSUS_STATE_SCHEMA, FLEET_EPOCH_ATTESTATION_SCHEMA, FleetConsensusStatus,
    FleetEpochConsensusState, NodeEpochAttestation,
};
