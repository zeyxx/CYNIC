use cynic_workload::{
    AgentWorkspaceApplyReport, AgentWorkspaceReconciliation, AgentWorkspaceRequest,
};

use super::agent_workspace_port::{AgentWorkspaceLayout, AgentWorkspacePortError};

pub trait AgentSourceBlobStorePort: Send + Sync {
    fn persist(&self, expected_digest: &str, content: &[u8])
    -> Result<(), AgentWorkspacePortError>;
    fn load(&self, digest: &str) -> Result<Vec<u8>, AgentWorkspacePortError>;
}

pub trait AgentWorkspaceApplyPort: Send + Sync {
    type Receipt;

    fn apply(
        &self,
        layout: &AgentWorkspaceLayout,
        request: &AgentWorkspaceRequest,
        reconciliation: &AgentWorkspaceReconciliation,
    ) -> Result<Self::Receipt, AgentWorkspacePortError>;

    fn report(receipt: &Self::Receipt) -> &AgentWorkspaceApplyReport;

    fn rollback(&self, receipt: Self::Receipt) -> Result<(), AgentWorkspacePortError>;

    fn commit(&self, receipt: Self::Receipt) -> Result<(), AgentWorkspacePortError>;
}
