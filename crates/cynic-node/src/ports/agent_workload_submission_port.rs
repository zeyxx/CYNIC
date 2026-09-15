use cynic_workload::AgentWorkloadDelivery;

/// Transport-neutral boundary for delivery of any already-admitted agent
/// workload.  The transport owns no admission, scheduler, or execution policy.
#[async_trait::async_trait]
pub trait AgentWorkloadSubmissionPort: Send + Sync {
    async fn submit(
        &self,
        delivery: &AgentWorkloadDelivery,
    ) -> Result<AgentWorkloadSubmissionReadBack, String>;
}

pub struct AgentWorkloadSubmissionReadBack {
    pub http_status: u16,
    pub body: serde_json::Value,
}
