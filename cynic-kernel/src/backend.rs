//! P0 — InferencePort: the sovereign abstraction over all compute backends.
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

/// Circuit breaker state for a backend.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendStatus {
    Healthy,
    Degraded { latency_ms: f64 },
    Unreachable,
}

/// Domain request — decoupled from MCTSInferenceRequest proto.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub trace_id: String,
    pub system_prompt: String,
    pub context: String,
    pub num_branches: u32,
    pub temperature: f32,
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

#[derive(Debug)]
pub enum BackendError {
    Unreachable(String),
    ModelNotLoaded(String),
    Timeout { backend_id: String, ms: u64 },
    Protocol(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(id)       => write!(f, "Backend unreachable: {}", id),
            Self::ModelNotLoaded(m)     => write!(f, "Model not loaded: {}", m),
            Self::Timeout { backend_id, ms } => write!(f, "Backend {} timed out after {}ms", backend_id, ms),
            Self::Protocol(msg)         => write!(f, "Protocol error: {}", msg),
        }
    }
}

// ============================================================
// THE PORT — the only way CYNIC talks to compute
// ============================================================

/// Every compute backend implements this trait.
/// Local GPU, remote MCP node, mock — all identical to the caller.
pub trait InferencePort: Send + Sync {
    fn capability(&self) -> &BackendCapability;

    fn infer(
        &self,
        req: InferenceRequest,
    ) -> impl Future<Output = Result<InferenceResponse, BackendError>> + Send;

    fn health(&self) -> impl Future<Output = BackendStatus> + Send;
}

use std::future::Future;

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

impl InferencePort for MockBackend {
    fn capability(&self) -> &BackendCapability {
        &self.capability
    }

    async fn infer(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError> {
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

    async fn health(&self) -> BackendStatus {
        if self.force_unreachable {
            BackendStatus::Unreachable
        } else {
            BackendStatus::Healthy
        }
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
    async fn mock_unreachable_reports_unreachable() {
        let backend = MockBackend::unreachable();
        assert_eq!(backend.health().await, BackendStatus::Unreachable);
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
