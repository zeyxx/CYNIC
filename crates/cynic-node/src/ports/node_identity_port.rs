use cynic_node_protocol::{NodeIdentityDesiredState, NodeIdentityObservation};

pub trait NodeIdentityPort: Send + Sync {
    fn observe(&self, desired: &NodeIdentityDesiredState) -> NodeIdentityObservation;
    fn generate(&self, desired: &NodeIdentityDesiredState) -> Result<(), String>;
}
