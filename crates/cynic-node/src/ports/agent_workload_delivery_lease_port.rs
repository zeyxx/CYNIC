use cynic_protocol::AgentWorkloadDeliveryLease;

pub trait AgentWorkloadDeliveryLeasePort: Send + Sync {
    fn read(&self, workload_id: &str) -> Result<Option<AgentWorkloadDeliveryLease>, String>;

    fn persist(&self, workload_id: &str, lease: &AgentWorkloadDeliveryLease) -> Result<(), String>;
}
