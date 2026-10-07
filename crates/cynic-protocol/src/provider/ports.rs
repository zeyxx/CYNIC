//! Pure provider substrate ports — zero I/O, types in `super::types`.
//!
//! Portable read-only boundary below SSH and the node runtime.
//! Provider-specific APIs, hypervisor commands, and bare-metal probes belong
//! in adapters implementing this port. Unknown provider state must remain an
//! explicit observation rather than an adapter error promoted to convergence.

use super::types::{NodeSubstrateAction, NodeSubstrateDesiredState, NodeSubstrateObservation};

/// Read-only observation of a provider's substrate state.
pub trait NodeSubstrateObservationPort: Send + Sync {
    fn observe_node_substrate(
        &self,
        desired: &NodeSubstrateDesiredState,
    ) -> Result<NodeSubstrateObservation, String>;
}

/// Explicit provider mutation boundary. Implementations must accept only
/// resource identifiers and actions already admitted by the pure decision.
pub trait NodeSubstrateControlPort: Send + Sync {
    fn apply_node_substrate_action(
        &self,
        desired: &NodeSubstrateDesiredState,
        action: &NodeSubstrateAction,
    ) -> Result<(), String>;
}
