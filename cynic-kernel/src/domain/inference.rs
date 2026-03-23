//! Inference domain types — backend capabilities, requests, responses, errors.
//!
//! No backend (local GPU, remote MCP, mock) ever touches the domain directly.
//! The domain speaks only to this trait. The substrate is irrelevant.

// ============================================================
// DOMAIN TYPES (not proto-generated — owned by the domain)
// ============================================================

/// What a backend can do. Discovered at boot, refreshed on health check.
#[derive(Debug, Clone)]
pub struct BackendCapability {
    /// Unique identifier: "local", "tailscale:<node>", "mock"
    pub id: String,
    /// Where to reach this backend
    pub kind: BackendKind,
    /// GPU/CPU model name
    pub device_name: String,
    /// Total VRAM in GB (0.0 = CPU or shared RAM)
    pub vram_total_gb: f64,
    /// Currently available VRAM in GB
    pub vram_available_gb: f64,
    /// Measured round-trip latency in ms (updated on health check)
    pub latency_ms: f64,
    /// Models currently loaded and ready
    pub loaded_models: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackendKind {
    /// llama-server on localhost
    Local,
    /// llama-server reachable via Tailscale
    Remote { url: String },
    /// In-process mock for tests
    Mock,
}

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

/// Domain request — decoupled from MCTSInferenceRequest proto.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub trace_id: String,
    pub system_prompt: String,
    pub context: String,
    pub num_branches: u32,
    pub temperature: f32,
    pub model_hint: Option<String>,
}

/// Domain response — decoupled from MCTSInferenceResponse proto.
#[derive(Debug, Clone)]
pub struct InferenceResponse {
    pub trace_id: String,
    pub hypotheses: Vec<String>,
    pub latency_ms: f64,
    pub model_used: String,
    /// Which backend actually served this request
    pub backend_id: String,
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
// THE PORT — the only way CYNIC talks to compute
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

// ============================================================
// MOCK BACKEND — P0 test implementation, no GPU required
// ============================================================

pub struct MockBackend {
    capability: BackendCapability,
    /// Simulated latency in ms
    pub latency_ms: f64,
    /// If true, health() returns Unreachable
    pub force_unreachable: bool,
}

impl MockBackend {
    pub fn healthy() -> Self {
        Self {
            capability: BackendCapability {
                id: "mock".to_string(),
                kind: BackendKind::Mock,
                device_name: "MockGPU-8G".to_string(),
                vram_total_gb: 8.0,
                vram_available_gb: 6.0,
                latency_ms: 10.0,
                loaded_models: vec!["phi-3-mini".to_string()],
            },
            latency_ms: 10.0,
            force_unreachable: false,
        }
    }

    pub fn unreachable() -> Self {
        Self {
            force_unreachable: true,
            ..Self::healthy()
        }
    }
}

#[async_trait]
impl BackendPort for MockBackend {
    fn name(&self) -> &str {
        &self.capability.id
    }

    async fn health(&self) -> BackendStatus {
        if self.force_unreachable {
            BackendStatus::Critical
        } else {
            BackendStatus::Healthy
        }
    }
}

impl MockBackend {
    pub fn capability(&self) -> &BackendCapability {
        &self.capability
    }

    pub async fn infer(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError> {
        if self.force_unreachable {
            return Err(BackendError::Unreachable("mock".to_string()));
        }

        Ok(InferenceResponse {
            trace_id: req.trace_id,
            hypotheses: vec![
                format!("Mock hypothesis A for: {}", &req.context[..req.context.len().min(40)]),
                format!("Mock hypothesis B for: {}", &req.context[..req.context.len().min(40)]),
            ],
            latency_ms: self.latency_ms,
            model_used: "phi-3-mini".to_string(),
            backend_id: "mock".to_string(),
        })
    }
}

// ============================================================
// TESTS — P0 contract tests, zero GPU required
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_healthy_returns_response() {
        let backend = MockBackend::healthy();
        let req = InferenceRequest {
            trace_id: "trace-001".to_string(),
            system_prompt: "You are CYNIC.".to_string(),
            context: "What is the price of SOL?".to_string(),
            num_branches: 2,
            temperature: 0.7,
            model_hint: None,
        };

        let res = backend.infer(req).await.unwrap();

        assert_eq!(res.backend_id, "mock");
        assert_eq!(res.hypotheses.len(), 2);
        assert_eq!(res.trace_id, "trace-001");
    }

    #[tokio::test]
    async fn mock_unreachable_returns_error() {
        let backend = MockBackend::unreachable();
        let req = InferenceRequest {
            trace_id: "trace-002".to_string(),
            system_prompt: "".to_string(),
            context: "ping".to_string(),
            num_branches: 1,
            temperature: 0.0,
            model_hint: None,
        };

        let err = backend.infer(req).await.unwrap_err();
        assert!(matches!(err, BackendError::Unreachable(_)));
    }

    #[tokio::test]
    async fn mock_healthy_reports_healthy() {
        let backend = MockBackend::healthy();
        assert_eq!(backend.health().await, BackendStatus::Healthy);
    }

    #[tokio::test]
    async fn mock_unreachable_reports_critical() {
        let backend = MockBackend::unreachable();
        assert_eq!(backend.health().await, BackendStatus::Critical);
    }

    #[tokio::test]
    async fn capability_exposes_correct_metadata() {
        let backend = MockBackend::healthy();
        let cap = backend.capability();

        assert_eq!(cap.id, "mock");
        assert_eq!(cap.kind, BackendKind::Mock);
        assert!(cap.vram_total_gb > 0.0);
        assert!(!cap.loaded_models.is_empty());
    }
}
