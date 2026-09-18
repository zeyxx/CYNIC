//! Driven ports for release gating and relay actuation.

use crate::types::{GateProofRecord, RelayActionKind};

/// Port implemented by build & inspection adapters (e.g. `cynic-build`) to provide gate evidence.
pub trait GateProofProvider {
    /// Retrieve observed gate proofs for a specific commit.
    fn observe_gate_proofs(&self, commit_sha: &str) -> Vec<GateProofRecord>;
}

/// Port implemented by VCS adapters (e.g. `cynic-vcs`) to execute authorized relay actions.
pub trait RelayActuatorPort {
    /// Request execution of an authorized relay push to public projection.
    fn execute_relay(
        &self,
        target_remote: &str,
        target_ref: &str,
        source_commit: &str,
        action: RelayActionKind,
    ) -> Result<String, String>;
}
