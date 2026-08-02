use cynic_protocol::AgentWorkloadDelivery;

pub trait AgentWorkloadDeliveryPort: Send + Sync {
    async fn pending_deliveries(&self, node_id: &str)
    -> Result<Vec<AgentWorkloadDelivery>, String>;

    async fn accept_delivery(
        &self,
        node_id: &str,
        workload_id: &str,
        record_digest: &str,
        accepted_at_ms: u64,
    ) -> Result<AgentWorkloadDelivery, String>;
}
