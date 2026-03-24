//! Inference domain types — backend status, errors, and port traits.
//!
//! No backend (local GPU, remote API, mock) ever touches the domain directly.
//! The domain speaks only to port traits. The substrate is irrelevant.

// ============================================================
// DOMAIN TYPES
// ============================================================

/// 5-state health lifecycle — φ-bounded: UNKNOWN until proven.
///
/// ```text
/// UNKNOWN ──(probe ok)──▶ HEALTHY
/// HEALTHY ──(N failures)──▶ DEGRADED
/// DEGRADED ──(threshold)──▶ CRITICAL
/// CRITICAL ──(cooldown)──▶ RECOVERING
/// RECOVERING ──(probe ok)──▶ HEALTHY
/// RECOVERING ──(probe fail)──▶ CRITICAL
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum BackendStatus {
    /// Boot default. No probe has run yet. Epistemic honesty: don't claim healthy without evidence.
    Unknown,
    /// Probe succeeded. Operating normally.
    Healthy,
    /// Partial failure or high latency. Still serving, reduced capacity.
    Degraded { latency_ms: f64 },
    /// Cannot serve requests. Circuit breaker open.
    Critical,
    /// Circuit breaker half-open. One probe request allowed.
    Recovering,
}

impl BackendStatus {
    /// Can this backend accept inference requests?
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

/// Error during backend construction (HTTP client init).
/// Isolates reqwest::Error from leaking into the domain.
#[derive(Debug, thiserror::Error)]
#[error("Backend initialization failed: {0}")]
pub struct BackendInitError(String);

impl BackendInitError {
    pub fn from_http(e: impl std::fmt::Display) -> Self {
        Self(e.to_string())
    }
}

use async_trait::async_trait;

// ============================================================
// PORT TRAITS — the only way CYNIC talks to compute
// ============================================================

/// Base identity + health contract shared by all backend abstractions.
/// ChatPort extends this — `health()` and `name()` are defined once.
#[async_trait]
pub trait BackendPort: Send + Sync {
    /// Human-readable name for this backend (e.g. "gemini", "hf-mistral", "sovereign-ubuntu").
    fn name(&self) -> &str;
    /// Health check this backend.
    async fn health(&self) -> BackendStatus;
}

/// Sovereign inference port — direct LLM access for cynic_infer MCP tool.
/// Distinct from domain::chat::ChatPort (Dog evaluation, requires BackendPort supertrait).
/// Named "Infer" to avoid collision with the Dog-facing ChatPort.
pub struct InferRequest {
    pub system: Option<String>,
    pub prompt: String,
    pub temperature: f64,
    pub max_tokens: u32,
}

pub struct InferResponse {
    pub text: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[async_trait]
pub trait InferPort: Send + Sync {
    async fn infer(&self, request: &InferRequest) -> Result<InferResponse, BackendError>;
}

/// Null implementation for graceful degradation when no sovereign LLM available.
pub struct NullInfer;

#[async_trait]
impl InferPort for NullInfer {
    async fn infer(&self, _request: &InferRequest) -> Result<InferResponse, BackendError> {
        Err(BackendError::Unreachable("No sovereign LLM available".into()))
    }
}
