use cynic_workload::AgentWorkloadDelivery;

pub trait AgentWorkloadCompletionReceiptPort: Send + Sync {
    fn contains(&self, terminal_record_digest: &str) -> Result<bool, String>;

    fn persist(&self, delivery: &AgentWorkloadDelivery) -> Result<(), String>;
}
