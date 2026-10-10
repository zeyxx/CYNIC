//! Shared types for the agent bus — re-exported from `cynic-types` (Layer 0).
//!
//! `AgentId`, `Budget`, `Scope`, `ScopeKind`, and `Hash32` now live in
//! `cynic-types` for cross-crate sharing. They are re-exported here so
//! `crate::types::AgentId` continues to work for all internal callers.
//!
//! `Task`, `Claim`, `Proof`, and the status/error enums are agent-bus-specific
//! and remain in this module.

pub use cynic_types::{AgentId, Budget, Hash32, Scope, ScopeKind, TaskId, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskStatus {
    Queued,
    Running,
    WaitingForAgent,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskKind {
    Solo,
    Chain,
    Fanout,
}

/// A unit of agent work.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: String,
    pub parent_task_id: Option<String>,
    pub created_by: AgentId,
    pub assigned_to: AgentId,
    pub kind: TaskKind,
    pub subject: String,
    pub body: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    pub created_at_unix_s: u64,
    pub updated_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub budget: Budget,
}

/// A claim on a task — represents an agent actively working on it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claim {
    pub claim_id: String,
    pub task_id: String,
    pub agent: AgentId,
    pub status: ClaimStatus,
    pub budget: Budget,
    pub consumed_actions: u32,
    pub scope: Scope,
    pub acquired_at_unix_s: u64,
    pub ttl_seconds: u32,
    pub parent_claim_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClaimStatus {
    Active,
    Expired,
    Released,
    Preempted,
    Blocked,
}

#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    #[error("claim {0} already exists")]
    AlreadyExists(String),
    #[error("claim {0} not found")]
    NotFound(String),
    #[error("scope {0} is locked by another task")]
    ScopeLocked(String),
    #[error("budget exhausted for claim {0}")]
    BudgetExhausted(String),
    #[error("invalid proof for claim")]
    InvalidProof,
}

/// A proof that a child task completed within a parent's scope.
pub struct Proof {
    pub parent_task_id: String,
    pub child_task_id: String,
    pub scope: Scope,
    pub consumed_budget: Budget,
    pub output_hash: Hash32,
    pub timestamp_unix_s: u64,
}
