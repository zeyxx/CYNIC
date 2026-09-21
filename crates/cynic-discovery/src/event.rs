use serde::{Deserialize, Serialize};

use crate::types::{Capability, NodeRole};

/// Events emitted when fleet topology changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiscoveryEvent {
    /// A new node joined and declared its capabilities.
    NodeJoined {
        node_id: String,
        role: NodeRole,
        capabilities: Vec<Capability>,
    },
    /// A node departed the fleet.
    NodeDeparted { node_id: String },
    /// A node's capabilities changed.
    CapabilityChanged {
        node_id: String,
        added: Vec<Capability>,
        removed: Vec<Capability>,
    },
}
