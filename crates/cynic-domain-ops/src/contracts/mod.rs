//! Machine-readable contracts for domain operations.
//!
//! Phase 2: contract deserialization and validation. This module parses
//! `cynic.domain_ops.desired_state.v1` JSON into typed structs, validates
//! schema/domain/provider/edge-probe constraints, and builds/serializes
//! `cynic.domain_ops.proof.v1` envelopes.
//!
//! Authority: foundation.json §37 (portable_host_contract), §40 (runtime_proof_over_prose).

pub mod desired_state;
pub mod error;
pub mod proof;

pub use desired_state::{desired_state_to_json, parse_desired_state};
pub use error::DomainOpsError;
pub use proof::{
    blocked_proof, build_proof, parse_proof, parse_proof_chain, proof_chain_to_json, proof_to_json,
    violation_proof,
};
