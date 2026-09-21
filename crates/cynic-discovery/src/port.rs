use crate::node::{FleetCapabilities, NodeCapabilities};

/// Pure port for observing fleet capabilities. Adapters implement this.
pub trait DiscoveryObservationPort {
    fn observe(&self) -> Result<FleetCapabilities, String>;
}

/// Pure port for observing a single node's capabilities.
pub trait NodeDiscoveryObservationPort {
    fn observe_node(&self, node_id: &str) -> Result<NodeCapabilities, String>;
}
