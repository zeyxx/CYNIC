//! Pool protocol messages (Stratum-like).

use crate::types::{ContributionKind, Worker};
use serde::{Deserialize, Serialize};

/// Messages exchanged between pool and workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum PoolMessage {
    /// Worker registers with the pool.
    WorkerHandshake {
        worker: Worker,
    },
    /// Pool assigns difficulty target.
    PoolDifficulty {
        epoch: u64,
        target: u64,
        contribution_kind: ContributionKind,
    },
    /// Worker requests a task.
    TaskRequest {
        agent_id: String,
        capabilities: Vec<String>,
    },
    /// Pool dispatches a task.
    TaskDispatch {
        task: Task,
    },
    /// Worker submits task result.
    TaskSubmit {
        result: TaskResult,
    },
    /// Pool acknowledges submission.
    TaskAck {
        task_id: String,
        status: AckStatus,
        share_value: Option<u64>,
        reason: Option<String>,
    },
    /// Worker claims reward.
    RewardClaim {
        agent_id: String,
        up_to_epoch: u64,
    },
    /// Pool responds with reward summary.
    RewardResponse {
        agent_id: String,
        total_rewards: f64,
        currency: String,
        epochs: Vec<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AckStatus {
    Accepted,
    Rejected,
}

/// A task dispatched by the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub contribution_kind: ContributionKind,
    pub inputs: TaskInputs,
    pub target: u64,
}

/// Inputs for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInputs {
    pub source_commit: Option<String>,
    pub action: Option<String>,
    pub registry_path: Option<String>,
    pub extra: serde_json::Value,
}

/// Result of executing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub contribution_kind: ContributionKind,
    pub success: bool,
    pub proof_digest: String,
    pub evidence: Vec<String>,
    pub nonce: u64,
    pub execution_time_ms: u64,
}

/// Helper to create a handshake message.
pub fn worker_handshake(agent_id: &str, role: crate::types::Role, node_id: &str, capabilities: Vec<String>) -> PoolMessage {
    PoolMessage::WorkerHandshake {
        worker: Worker {
            agent_id: agent_id.to_string(),
            role,
            capabilities,
            node_id: node_id.to_string(),
            registered_at_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        },
    }
}

/// Helper to create a task dispatch.
pub fn task_dispatch(task_id: &str, kind: ContributionKind, inputs: TaskInputs, target: u64) -> PoolMessage {
    PoolMessage::TaskDispatch {
        task: Task {
            task_id: task_id.to_string(),
            contribution_kind: kind,
            inputs,
            target,
        },
    }
}
