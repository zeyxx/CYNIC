//! Fleet Consensus & Temporal Monotonic Synchronization for CYNIC
//! Inspired by Solana Tower BFT & PoH: Pure state machines, 2/3 Quorum verification,
//! and strictly monotonic epoch sequencing without wall-clock trust.
//!
//! Consensus attestation/evaluation logic is hoisted to `cynic_protocol::fleet::consensus`;
//! this crate re-exports it for backward compatibility and retains the peer domain.

pub mod peer;
pub use peer::*;

pub use cynic_protocol::fleet::consensus::*;