//! Error types for CYNIC Scheduler

use thiserror::Error;

/// Result type for scheduler operations
pub type Result<T> = std::result::Result<T, SchedulerError>;

/// Scheduler errors
#[derive(Error, Debug)]
pub enum SchedulerError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// CYNIC API error
    #[error("CYNIC API error: {0}")]
    CynicApi(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Transaction parsing error
    #[error("Transaction parsing error: {0}")]
    TransactionParse(String),

    /// Queue error
    #[error("Queue error: {0}")]
    Queue(String),

    /// Shared memory error
    #[error("Shared memory error: {0}")]
    SharedMemory(String),

    /// Worker communication error
    #[error("Worker communication error: {0}")]
    WorkerComm(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl SchedulerError {
    /// Create a config error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a CYNIC API error
    pub fn cynic_api(msg: impl Into<String>) -> Self {
        Self::CynicApi(msg.into())
    }

    /// Create a transaction parse error
    pub fn tx_parse(msg: impl Into<String>) -> Self {
        Self::TransactionParse(msg.into())
    }

    /// Create a queue error
    pub fn queue(msg: impl Into<String>) -> Self {
        Self::Queue(msg.into())
    }

    /// Create a shared memory error
    pub fn shared_memory(msg: impl Into<String>) -> Self {
        Self::SharedMemory(msg.into())
    }

    /// Create a worker communication error
    pub fn worker_comm(msg: impl Into<String>) -> Self {
        Self::WorkerComm(msg.into())
    }

    /// Create a timeout error
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}
