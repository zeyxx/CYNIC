//! Inference domain types — backend status, errors, and request/response values.
//!
//! No backend implementation belongs here. This module carries only domain data.

use serde::{Deserialize, Serialize};

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
    /// Residual unknowns: what was not observed during inference.
    /// Empty when all metrics are proven; non-empty when e.g. latency
    /// was not measured or usage data was not returned.
    pub residual_unknowns: Vec<String>,
    /// Phase latency breakdown (prefill vs decode).
    pub phase_latency: PhaseLatency,
}

/// Latency breakdown by inference phase.
///
/// From Ahmad's "LLMs 101": Prefill is compute-bound, decode is
/// bandwidth-bound. Tracking them separately is essential for
/// diagnosing bottlenecks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseLatency {
    /// Time to first token (prefill) in milliseconds.
    pub prefill_ms: Option<u64>,
    /// Total decode time in milliseconds.
    pub decode_ms: Option<u64>,
    /// Decoded tokens per second.
    pub tokens_per_second: Option<f64>,
}

impl PhaseLatency {
    pub fn unknown() -> Self {
        Self {
            prefill_ms: None,
            decode_ms: None,
            tokens_per_second: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_response_default_has_empty_residual_unknowns() {
        let resp = InferResponse {
            text: "hello".into(),
            model: "test".into(),
            prompt_tokens: 1,
            completion_tokens: 2,
            residual_unknowns: Vec::new(),
            phase_latency: PhaseLatency::unknown(),
        };
        assert!(resp.residual_unknowns.is_empty());
    }

    #[test]
    fn infer_response_can_carry_residual_unknowns() {
        let resp = InferResponse {
            text: "partial".into(),
            model: "test".into(),
            prompt_tokens: 1,
            completion_tokens: 0,
            residual_unknowns: vec!["latency_ms_not_observed".into()],
            phase_latency: PhaseLatency::unknown(),
        };
        assert_eq!(resp.residual_unknowns.len(), 1);
        assert_eq!(resp.residual_unknowns[0], "latency_ms_not_observed");
    }

    #[test]
    fn backend_status_is_available_matches_healthy_and_degraded() {
        assert!(BackendStatus::Healthy.is_available());
        assert!(BackendStatus::Degraded { latency_ms: 50.0 }.is_available());
        assert!(!BackendStatus::Unknown.is_available());
        assert!(!BackendStatus::Critical.is_available());
        assert!(!BackendStatus::Recovering.is_available());
    }
}
