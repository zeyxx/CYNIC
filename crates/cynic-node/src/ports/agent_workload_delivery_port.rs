use cynic_workload::{
    AgentWorkloadCompletionPublication, AgentWorkloadDelivery, AgentWorkloadDeliveryAcceptance,
};

pub trait AgentWorkloadDeliveryPort: Send + Sync {
    async fn pending_deliveries(&self, node_id: &str)
    -> Result<Vec<AgentWorkloadDelivery>, String>;

    async fn accept_delivery(
        &self,
        node_id: &str,
        workload_id: &str,
        acceptance: &AgentWorkloadDeliveryAcceptance,
    ) -> Result<AgentWorkloadDelivery, String>;

    async fn publish_completion(
        &self,
        publication: &AgentWorkloadCompletionPublication,
    ) -> Result<AgentWorkloadDelivery, String>;
}
