use cynic_protocol::{AgentWorkloadRecord, DurableAgentWorkloadRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWorkloadStoreError {
    pub code: String,
    pub detail: String,
}

impl AgentWorkloadStoreError {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

pub trait AgentWorkloadStorePort: Send + Sync {
    fn read(
        &self,
        workload_id: &str,
    ) -> Result<Option<DurableAgentWorkloadRecord>, AgentWorkloadStoreError>;

    fn active_idempotency_keys(&self) -> Result<Vec<String>, AgentWorkloadStoreError>;

    fn active_records(&self) -> Result<Vec<AgentWorkloadRecord>, AgentWorkloadStoreError>;

    fn durable_records(&self) -> Result<Vec<DurableAgentWorkloadRecord>, AgentWorkloadStoreError>;

    fn compare_and_swap(
        &self,
        workload_id: &str,
        expected_revision: Option<u64>,
        next: &AgentWorkloadRecord,
        committed_at_ms: u64,
    ) -> Result<DurableAgentWorkloadRecord, AgentWorkloadStoreError>;
}
