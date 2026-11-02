use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use super::delivery::AgentWorkloadDelivery;

use crate::{
    AgentSchedulerJournalRecord,
    AgentWorkloadCompletionPublication,
    AgentWorkloadDelivery,
    AgentWorkloadDeliveryAcceptance,
    AgentWorkloadDeliveryLease,
    AgentWorkloadRecord,
    DurableAgentWorkloadRecord,
};
use crate::workspace::ports::AgentWorkspacePortError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildWorkloadSubmissionReadBack {
    pub http_status: u16,
    pub body: String,
}

#[async_trait]
pub trait BuildWorkloadSubmissionPort: Send + Sync {
    async fn submit(
        &self,
        delivery: &AgentWorkloadDelivery,
    ) -> Result<BuildWorkloadSubmissionReadBack, String>;
}

#[async_trait]
pub trait BuildWorkloadObservationPort: Send + Sync {
    async fn observe_workload(
        &self,
        workload_id: &str,
    ) -> Result<BuildWorkloadObservationReadBack, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildWorkloadObservationReadBack {
    pub http_status: u16,
    pub body: serde_json::Value,
}

pub trait AgentSchedulerJournalPort: Send + Sync {
    fn records(
        &self,
        node_id: &str,
        scheduler_epoch: u64,
        step_id: &str,
    ) -> Result<Vec<AgentSchedulerJournalRecord>, AgentWorkspacePortError>;

    fn persist(
        &self,
        record: &AgentSchedulerJournalRecord,
    ) -> Result<AgentSchedulerJournalRecord, AgentWorkspacePortError>;
}

pub trait AgentWorkloadCompletionReceiptPort: Send + Sync {
    fn contains(&self, terminal_record_digest: &str) -> Result<bool, String>;

    fn persist(&self, delivery: &AgentWorkloadDelivery) -> Result<(), String>;
}

pub trait AgentWorkloadDeliveryLeasePort: Send + Sync {
    fn read(&self, workload_id: &str) -> Result<Option<AgentWorkloadDeliveryLease>, String>;

    fn persist(&self, workload_id: &str, lease: &AgentWorkloadDeliveryLease) -> Result<(), String>;
}

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
