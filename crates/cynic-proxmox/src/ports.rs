//! Port re-exports for provider adapters.
//!
//! The pure ports live in `cynic-protocol` (L0). This module re-exports the
//! ones the Proxmox adapters implement, so consumers can name them through
//! the provider crate without reaching into `cynic-protocol` internals.

pub use cynic_protocol::provider::credential_lease_verifier::CredentialLeaseVerifierPort;
pub use cynic_protocol::provider::{NodeSubstrateControlPort, NodeSubstrateObservationPort};
