//! BackendRouter — selects and dispatches to registered InferencePort backends.
//!
//! Circuit breaker per backend. Health probed periodically, NOT per request.

use crate::backend::*;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Circuit breaker state tracked per backend.
struct TrackedBackend {
    backend: Arc<dyn InferencePort>,
    status: BackendStatus,
    consecutive_failures: u32,
    last_probe: Instant,
}

impl TrackedBackend {
    fn new(backend: Arc<dyn InferencePort>) -> Self {
        Self {
            backend,
            status: BackendStatus::Unknown,
            consecutive_failures: 0,
            last_probe: Instant::now(),
        }
    }

    fn is_available(&self) -> bool {
        self.status.is_available()
    }

    fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.status = BackendStatus::Healthy;
        self.last_probe = Instant::now();
    }

    fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.last_probe = Instant::now();
        if self.consecutive_failures >= FAILURE_THRESHOLD {
            self.status = BackendStatus::Critical;
        } else {
            self.status = BackendStatus::Degraded { latency_ms: 0.0 };
        }
    }
}

/// After this many consecutive failures, backend goes CRITICAL.
const FAILURE_THRESHOLD: u32 = 3;
/// Cooldown before probing a CRITICAL backend.
const COOLDOWN: Duration = Duration::from_secs(15);
/// How often to re-probe HEALTHY backends.
const PROBE_INTERVAL: Duration = Duration::from_secs(30);

pub struct BackendRouter {
    backends: RwLock<Vec<TrackedBackend>>,
    round_robin: AtomicUsize,
}

impl BackendRouter {
    pub fn new(backends: Vec<Arc<dyn InferencePort>>) -> Self {
        Self {
            backends: RwLock::new(
                backends.into_iter().map(TrackedBackend::new).collect()
            ),
            round_robin: AtomicUsize::new(0),
        }
    }

    /// Register a new backend at runtime. Starts as UNKNOWN until first probe.
    pub async fn register(&self, backend: Arc<dyn InferencePort>) {
        self.backends.write().await.push(TrackedBackend::new(backend));
    }

    /// Periodic health probe — call this from a background task, not per request.
    pub async fn probe_all(&self) {
        let mut backends = self.backends.write().await;
        for tracked in backends.iter_mut() {
            let should_probe = match tracked.status {
                BackendStatus::Unknown => true,
                BackendStatus::Healthy | BackendStatus::Degraded { .. } => {
                    tracked.last_probe.elapsed() >= PROBE_INTERVAL
                }
                BackendStatus::Critical => {
                    tracked.last_probe.elapsed() >= COOLDOWN
                }
                BackendStatus::Recovering => true,
            };

            if should_probe {
                if matches!(tracked.status, BackendStatus::Critical) {
                    tracked.status = BackendStatus::Recovering;
                }
                match tracked.backend.health().await {
                    BackendStatus::Healthy => tracked.record_success(),
                    BackendStatus::Degraded { latency_ms } => {
                        tracked.status = BackendStatus::Degraded { latency_ms };
                        tracked.consecutive_failures = 0;
                        tracked.last_probe = Instant::now();
                    }
                    _ => tracked.record_failure(),
                }
            }
        }
    }

    /// Route a single request to the best available backend.
    /// Does NOT call health() — relies on cached circuit breaker state.
    pub async fn route(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError> {
        let backends = self.backends.read().await;

        if backends.is_empty() {
            return Err(BackendError::Unreachable("no backends registered".into()));
        }

        // 1. Filter by model_hint if provided
        if let Some(ref hint) = req.model_hint {
            for tracked in backends.iter() {
                if tracked.is_available()
                    && tracked.backend.capability().loaded_models.iter().any(|m| m.contains(hint.as_str()))
                {
                    return tracked.backend.infer(req).await;
                }
            }
        }

        // 2. Round-robin over available backends (cached state, no health() call)
        let count = backends.len();
        let start = self.round_robin.fetch_add(1, Ordering::Relaxed);
        for offset in 0..count {
            let idx = (start + offset) % count;
            let tracked = &backends[idx];
            if tracked.is_available() {
                return tracked.backend.infer(req).await;
            }
        }

        Err(BackendError::Unreachable("all backends unavailable".into()))
    }

    /// Record inference result for circuit breaker updates.
    pub async fn record_result(&self, backend_id: &str, success: bool) {
        let mut backends = self.backends.write().await;
        if let Some(tracked) = backends.iter_mut().find(|t| t.backend.capability().id == backend_id) {
            if success {
                tracked.record_success();
            } else {
                tracked.record_failure();
            }
        }
    }

    /// Fan-out: dispatch to N backends in parallel, collect partial results.
    pub async fn fan_out(&self, req: InferenceRequest, n: u32) -> Result<InferenceResponse, BackendError> {
        if n <= 1 {
            return self.route(req).await;
        }

        let backends = self.backends.read().await;
        let healthy: Vec<_> = backends.iter()
            .filter(|t| t.is_available())
            .map(|t| Arc::clone(&t.backend))
            .collect();
        drop(backends);

        if healthy.is_empty() {
            return Err(BackendError::Unreachable("all backends unavailable".into()));
        }

        let mut handles = Vec::new();
        for i in 0..n {
            let backend = Arc::clone(&healthy[i as usize % healthy.len()]);
            let branch_req = InferenceRequest {
                trace_id: format!("{}-branch-{}", req.trace_id, i),
                temperature: req.temperature + (i as f32 * 0.1),
                ..req.clone()
            };
            handles.push(tokio::spawn(async move {
                backend.infer(branch_req).await
            }));
        }

        let mut all_hypotheses = Vec::new();
        let mut latency_ms = 0.0f64;
        let mut model_used = String::new();
        let mut backend_id = String::new();

        for handle in handles {
            if let Ok(Ok(resp)) = handle.await {
                all_hypotheses.extend(resp.hypotheses);
                latency_ms = latency_ms.max(resp.latency_ms);
                if model_used.is_empty() {
                    model_used = resp.model_used;
                    backend_id = resp.backend_id;
                }
            }
        }

        if all_hypotheses.is_empty() {
            return Err(BackendError::Unreachable("all fan-out branches failed".into()));
        }

        Ok(InferenceResponse {
            trace_id: req.trace_id,
            hypotheses: all_hypotheses,
            latency_ms,
            model_used,
            backend_id,
        })
    }

    /// Get current status of all backends (for health reporting).
    pub async fn backend_statuses(&self) -> Vec<(String, BackendStatus)> {
        self.backends.read().await.iter()
            .map(|t| (t.backend.capability().id.clone(), t.status.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MockBackend;

    fn req(id: &str) -> InferenceRequest {
        InferenceRequest {
            trace_id: id.into(),
            system_prompt: "".into(),
            context: "test context for router unit tests".into(),
            num_branches: 1,
            temperature: 0.5,
            model_hint: None,
        }
    }

    #[tokio::test]
    async fn route_to_single_healthy_backend() {
        let mock = Arc::new(MockBackend::healthy()) as Arc<dyn InferencePort>;
        let router = BackendRouter::new(vec![mock]);
        // Probe first to transition from Unknown → Healthy
        router.probe_all().await;
        let resp = router.route(req("t1")).await.unwrap();
        assert_eq!(resp.backend_id, "mock");
        assert!(!resp.hypotheses.is_empty());
    }

    #[tokio::test]
    async fn route_skips_critical() {
        let dead = Arc::new(MockBackend::unreachable()) as Arc<dyn InferencePort>;
        let alive = Arc::new(MockBackend::healthy()) as Arc<dyn InferencePort>;
        let router = BackendRouter::new(vec![dead, alive]);
        router.probe_all().await;
        let resp = router.route(req("t2")).await.unwrap();
        assert_eq!(resp.backend_id, "mock");
    }

    #[tokio::test]
    async fn route_no_backends_errors() {
        let router = BackendRouter::new(vec![]);
        assert!(router.route(req("t3")).await.is_err());
    }

    #[tokio::test]
    async fn fan_out_collects_hypotheses() {
        let mock = Arc::new(MockBackend::healthy()) as Arc<dyn InferencePort>;
        let router = BackendRouter::new(vec![mock]);
        router.probe_all().await;
        let resp = router.fan_out(req("t4"), 3).await.unwrap();
        assert!(resp.hypotheses.len() >= 3);
    }

    #[tokio::test]
    async fn register_adds_backend() {
        let router = BackendRouter::new(vec![]);
        assert!(router.route(req("t5")).await.is_err());
        router.register(Arc::new(MockBackend::healthy())).await;
        router.probe_all().await;
        assert!(router.route(req("t6")).await.is_ok());
    }

    #[tokio::test]
    async fn backends_start_unknown() {
        let mock = Arc::new(MockBackend::healthy()) as Arc<dyn InferencePort>;
        let router = BackendRouter::new(vec![mock]);
        let statuses = router.backend_statuses().await;
        assert_eq!(statuses[0].1, BackendStatus::Unknown);
        // Unknown is NOT available — route should fail
        assert!(router.route(req("t7")).await.is_err());
    }

    #[tokio::test]
    async fn probe_transitions_unknown_to_healthy() {
        let mock = Arc::new(MockBackend::healthy()) as Arc<dyn InferencePort>;
        let router = BackendRouter::new(vec![mock]);
        assert_eq!(router.backend_statuses().await[0].1, BackendStatus::Unknown);
        router.probe_all().await;
        assert_eq!(router.backend_statuses().await[0].1, BackendStatus::Healthy);
    }
}
