use serde::{Deserialize, Serialize};

use crate::types::{Capability, DISCOVERY_CAPABILITIES_SCHEMA, NodeRole, timestamp_now};

/// Complete capability declaration for a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Stable node identifier (matches registry).
    pub node_id: String,
    /// Observed role.
    pub role: NodeRole,
    /// Network endpoint for this node (IP or hostname).
    pub endpoint: String,
    /// Declared capabilities.
    pub capabilities: Vec<Capability>,
    /// Unix timestamp of last observation.
    pub observed_at_unix_s: u64,
}

impl NodeCapabilities {
    pub fn new(
        node_id: &str,
        role: NodeRole,
        endpoint: &str,
        capabilities: Vec<Capability>,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            role,
            endpoint: endpoint.to_string(),
            capabilities,
            observed_at_unix_s: timestamp_now(),
        }
    }

    /// True if this node declares a specific capability.
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }
}

/// Aggregated capabilities across the entire fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetCapabilities {
    /// Schema version.
    pub schema: String,
    /// Unix timestamp of aggregation.
    pub observed_at_unix_s: u64,
    /// All nodes observed.
    pub nodes: Vec<NodeCapabilities>,
}

impl FleetCapabilities {
    pub fn new(nodes: Vec<NodeCapabilities>) -> Self {
        Self {
            schema: DISCOVERY_CAPABILITIES_SCHEMA.to_string(),
            observed_at_unix_s: timestamp_now(),
            nodes,
        }
    }

    /// Find all nodes with a specific capability.
    pub fn find_by_capability(&self, cap: &Capability) -> Vec<&NodeCapabilities> {
        self.nodes
            .iter()
            .filter(|n| n.has_capability(cap))
            .collect()
    }

    /// Find all nodes with a specific role.
    pub fn find_by_role(&self, role: NodeRole) -> Vec<&NodeCapabilities> {
        self.nodes.iter().filter(|n| n.role == role).collect()
    }

    /// Count of live (responding) nodes.
    pub fn live_count(&self) -> usize {
        self.nodes.len()
    }

    /// True if the fleet has at least one build executor.
    pub fn has_builder(&self) -> bool {
        !self
            .find_by_capability(&Capability::BuildExecute)
            .is_empty()
    }
}
