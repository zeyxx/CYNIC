//! BackendRouter — selects and dispatches to registered InferencePort backends.

use crate::backend::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct BackendRouter {
    backends: RwLock<Vec<Arc<dyn InferencePort>>>,
    round_robin: AtomicUsize,
}

impl BackendRouter {
    pub fn new(backends: Vec<Arc<dyn InferencePort>>) -> Self {
        Self {
            backends: RwLock::new(backends),
            round_robin: AtomicUsize::new(0),
        }
    }

    /// Register a new backend at runtime.
    pub async fn register(&self, backend: Arc<dyn InferencePort>) {
        self.backends.write().await.push(backend);
    }

    /// Route a single request to the best available backend.
    pub async fn route(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError> {
        let backends = self.backends.read().await;

        if backends.is_empty() {
            return Err(BackendError::Unreachable("no backends registered".into()));
        }

        // 1. Filter by model_hint if provided
        if let Some(ref hint) = req.model_hint {
            for b in backends.iter() {
                if b.capability().loaded_models.iter().any(|m| m.contains(hint.as_str()))
                    && matches!(b.health().await, BackendStatus::Healthy | BackendStatus::Degraded { .. })
                {
                    return b.infer(req).await;
                }
            }
        }

        // 2. Round-robin over healthy backends
        let count = backends.len();
        let start = self.round_robin.fetch_add(1, Ordering::Relaxed);
        for offset in 0..count {
            let idx = (start + offset) % count;
            let b = &backends[idx];
            match b.health().await {
                BackendStatus::Healthy | BackendStatus::Degraded { .. } => {
                    return b.infer(req).await;
                }
                BackendStatus::Unreachable => continue,
            }
        }

        Err(BackendError::Unreachable("all backends unreachable".into()))
    }

    /// Fan-out: dispatch to N backends in parallel, collect partial results.
    pub async fn fan_out(&self, req: InferenceRequest, n: u32) -> Result<InferenceResponse, BackendError> {
        if n <= 1 {
            return self.route(req).await;
        }

        let backends = self.backends.read().await;
        let mut healthy = Vec::new();
        for b in backends.iter() {
            if matches!(b.health().await, BackendStatus::Healthy | BackendStatus::Degraded { .. }) {
                healthy.push(Arc::clone(b));
            }
        }
        drop(backends);

        if healthy.is_empty() {
            return Err(BackendError::Unreachable("all backends unreachable".into()));
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
        let resp = router.route(req("t1")).await.unwrap();
        assert_eq!(resp.backend_id, "mock");
        assert!(!resp.hypotheses.is_empty());
    }

    #[tokio::test]
    async fn route_skips_unreachable() {
        let dead = Arc::new(MockBackend::unreachable()) as Arc<dyn InferencePort>;
        let alive = Arc::new(MockBackend::healthy()) as Arc<dyn InferencePort>;
        let router = BackendRouter::new(vec![dead, alive]);
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
        let resp = router.fan_out(req("t4"), 3).await.unwrap();
        assert!(resp.hypotheses.len() >= 3);
    }

    #[tokio::test]
    async fn register_adds_backend() {
        let router = BackendRouter::new(vec![]);
        assert!(router.route(req("t5")).await.is_err());
        router.register(Arc::new(MockBackend::healthy())).await;
        assert!(router.route(req("t6")).await.is_ok());
    }
}
