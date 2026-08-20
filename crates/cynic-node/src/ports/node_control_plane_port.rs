use cynic_protocol::{
    NodeControlObservation, NodeControlPlan, NodeDesiredState, SignedNodeControlPlan,
};

pub trait NodeDesiredStatePullPort {
    async fn pull_desired_state(&self, node_id: &str) -> Result<Option<NodeDesiredState>, String>;
}

pub trait SignedNodeDesiredStatePullPort {
    async fn pull_signed_desired_state(
        &self,
        node_id: &str,
    ) -> Result<Option<SignedNodeControlPlan>, String>;
}
pub trait NodeControlObservationPort {
    async fn observe_node_control(
        &self,
        desired: &NodeDesiredState,
    ) -> Result<NodeControlObservation, String>;
}

pub trait NodeStatusPublishPort {
    async fn publish_node_status(&self, plan: &NodeControlPlan) -> Result<String, String>;
}
