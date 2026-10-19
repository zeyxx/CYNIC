//! Functional Core / DDD Aggregate: Agent session records for epistemic memory.
//!
//! ZERO I/O: pure data structures, validation rules, and schema mapping.

use serde::{Deserialize, Serialize};

/// Epistemic agent session status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionStatus {
    Active,
    Completed,
    Failed,
    Interrupted,
}

/// DDD Aggregate: Agent session persistence record for SurrealDB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionRecord {
    pub session_id: String,
    pub agent_id: String,
    pub status: AgentSessionStatus,
    pub started_at_epoch_ms: u64,
    pub completed_at_epoch_ms: Option<u64>,
    pub turns_count: usize,
    pub goal_summary: String,
}

impl AgentSessionRecord {
    pub const TABLE_NAME: &'static str = "agent_sessions";

    pub fn new(
        session_id: impl Into<String>,
        agent_id: impl Into<String>,
        started_at_epoch_ms: u64,
        goal_summary: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            agent_id: agent_id.into(),
            status: AgentSessionStatus::Active,
            started_at_epoch_ms,
            completed_at_epoch_ms: None,
            turns_count: 0,
            goal_summary: goal_summary.into(),
        }
    }

    /// Pure transition to completed state.
    pub fn mark_completed(&mut self, timestamp_epoch_ms: u64) {
        self.status = AgentSessionStatus::Completed;
        self.completed_at_epoch_ms = Some(timestamp_epoch_ms);
    }

    /// Pure transition to failed state.
    pub fn mark_failed(&mut self, timestamp_epoch_ms: u64) {
        self.status = AgentSessionStatus::Failed;
        self.completed_at_epoch_ms = Some(timestamp_epoch_ms);
    }
}
