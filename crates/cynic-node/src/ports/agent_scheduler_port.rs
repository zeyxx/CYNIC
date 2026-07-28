use cynic_protocol::AgentSchedulerJournalRecord;

use crate::ports::agent_workspace_port::AgentWorkspacePortError;

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
