use cynic_protocol::AgentWorkloadDelivery;

/// Transport boundary for submitting an already-admitted build workload to
/// the kernel-owned durable journal. MCP, CLI, and local Unix clients supply
/// adapters; none owns the delivery contract.
#[async_trait::async_trait]
pub trait BuildWorkloadSubmissionPort: Send + Sync {
    async fn submit(
        &self,
        delivery: &AgentWorkloadDelivery,
    ) -> Result<BuildWorkloadSubmissionReadBack, String>;
}

pub struct BuildWorkloadSubmissionReadBack {
    pub http_status: u16,
    pub body: serde_json::Value,
}
