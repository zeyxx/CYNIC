//! Inference domain types — backend status, errors, and request/response values.
//!
//! No backend implementation belongs here. This module carries only domain data.

use serde::{Deserialize, Serialize};

/// Execution phase of inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferencePhase {
    /// Initial prompt processing (compute and parallelism bound).
    Prefill,
    /// Autoregressive token generation (memory bandwidth bound).
    Decode,
}

/// Structured latency decomposition for an inference operation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct LatencyBreakdown {
    /// Time to first token (prefill latency) in milliseconds.
    pub ttft_ms: f64,
    /// Time spent generating subsequent tokens in milliseconds.
    pub generation_ms: f64,
    /// Total execution time in milliseconds.
    pub total_ms: f64,
}

impl LatencyBreakdown {
    pub fn new(ttft_ms: f64, generation_ms: f64) -> Self {
        Self {
            ttft_ms,
            generation_ms,
            total_ms: ttft_ms + generation_ms,
        }
    }
}
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct InferResponse {
    pub text: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub latency: Option<LatencyBreakdown>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_breakdown_calculation() {
        let breakdown = LatencyBreakdown::new(120.0, 480.0);
        assert_eq!(breakdown.ttft_ms, 120.0);
        assert_eq!(breakdown.generation_ms, 480.0);
        assert_eq!(breakdown.total_ms, 600.0);
    }

    #[test]
    fn test_inference_phases() {
        assert_ne!(InferencePhase::Prefill, InferencePhase::Decode);
    }
}
