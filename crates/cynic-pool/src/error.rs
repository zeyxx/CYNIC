//! Typed error codes for the CYNIC pool.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PoolError {
    #[error("Worker already registered: {0}")]
    WorkerAlreadyRegistered(String),

    #[error("Worker not found: {0}")]
    WorkerNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Contribution validation failed: {0}")]
    ValidationFailed(String),

    #[error("Difficulty adjustment failed: {0}")]
    DifficultyAdjustmentFailed(String),

    #[error("Epoch not complete")]
    EpochNotComplete,

    #[error("Reward claim invalid: {0}")]
    RewardClaimInvalid(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Node not authorized: {0}")]
    NodeNotAuthorized(String),
}
