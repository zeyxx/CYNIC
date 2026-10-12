//! Inference domain types — backend status and initialization errors.
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
#[error("Backend initialization failed: {0}")]
pub struct BackendInitError(String);

impl BackendInitError {
    pub fn from_http(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}
