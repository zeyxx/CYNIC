use cynic_agent::{AgentWorkloadRecord, DurableAgentWorkloadRecord};

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

    fn archive_terminal(
        &self,
        workload_id: &str,
        older_than_ms: u64,
        now_ms: u64,
    ) -> Result<Option<String>, AgentWorkloadStoreError>;

    /// Append a read-only JSON evidence sidecar inside the workload directory
    /// (e.g. the settlement proof body). Never mutates revisions; fails closed
    /// on unknown workloads or unsafe file names.
    fn append_evidence(
        &self,
        workload_id: &str,
        file_name: &str,
        body: &serde_json::Value,
    ) -> Result<(), AgentWorkloadStoreError>;
}
