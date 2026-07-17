//! Inference domain types — backend status, errors, and request/response values.
//!
//! No backend implementation belongs here. This module carries only domain data.

/// 5-state health lifecycle — φ-bounded: UNKNOWN until proven.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendStatus {
    Unknown,
    Healthy,
    Degraded { latency_ms: f64 },
    Critical,
    Recovering,
}

impl BackendStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded { .. })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Backend unreachable: {0}")]
    Unreachable(String),
    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),
    #[error("Backend {backend_id} timed out after {ms}ms")]
    Timeout { backend_id: String, ms: u64 },
    #[error("Protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug, thiserror::Error)]
#[error("Backend initialization failed: {0}")]
pub struct BackendInitError(String);

impl BackendInitError {
    pub fn from_http(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

#[derive(Debug)]
pub struct InferRequest {
    pub system: Option<String>,
    pub prompt: String,
    pub temperature: f64,
    pub max_tokens: u32,
}

#[derive(Debug)]
pub struct InferResponse {
    pub text: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}
