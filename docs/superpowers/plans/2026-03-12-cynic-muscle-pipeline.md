# cynic-muscle Pipeline — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a working end-to-end pipeline: gRPC request → BackendRouter → LlamaCppBackend → llama-server HTTP → real inference response back through the kernel.

**Architecture:** The kernel on forge dispatches inference requests through a `BackendRouter` that selects from registered `InferencePort` backends. The first (and MVP) backend is `LlamaCppBackend`, which calls llama-server's OpenAI-compatible HTTP API. Everything runs through the existing gRPC service on `[::1]:50051`.

**Tech Stack:** Rust (edition 2024), tonic 0.12 (gRPC), reqwest 0.13 (HTTP), prost 0.13 (protobuf), tokio 1.50, async-trait

**Target:** forge (`/home/kairos/CYNIC-validate/cynic-kernel/`)

**The Pipeline:**
```
grpcurl / Claude Code
    │
    ▼
[::1]:50051  MuscleHAL.RequestInference(MCTSInferenceRequest)
    │
    ▼
hal.rs  MuscleService → BackendRouter.route(req)
    │
    ▼
router.rs  BackendRouter → selects LlamaCppBackend (health + capability)
    │
    ▼
backend_llamacpp.rs  POST http://<llama-server>:11435/v1/chat/completions
    │
    ▼
llama-server  Qwen3.5-2B (or whatever is loaded) → completion
    │
    ▼
MCTSInferenceResponse { hypotheses, latency_ms, model_used }
```

---

## Chunk 1: Make InferencePort Object-Safe + LlamaCppBackend

### Task 1: Add async-trait dependency and make InferencePort object-safe

**Files:**
- Modify: `/home/kairos/CYNIC-validate/cynic-kernel/Cargo.toml`
- Modify: `/home/kairos/CYNIC-validate/cynic-kernel/src/backend.rs`

- [ ] **Step 1: Add `async-trait` to Cargo.toml**

In `cynic-kernel/Cargo.toml`, add to `[dependencies]`:
```toml
async-trait = "0.1"
```

- [ ] **Step 2: Convert InferencePort to use async-trait**

In `backend.rs`, replace the trait definition (lines ~90-102):

```rust
use std::future::Future;
```
becomes:
```rust
use async_trait::async_trait;
```

And the trait:
```rust
pub trait InferencePort: Send + Sync {
    fn capability(&self) -> &BackendCapability;
    fn infer(&self, req: InferenceRequest) -> impl Future<Output = Result<InferenceResponse, BackendError>> + Send;
    fn health(&self) -> impl Future<Output = BackendStatus> + Send;
}
```
becomes:
```rust
#[async_trait]
pub trait InferencePort: Send + Sync {
    fn capability(&self) -> &BackendCapability;
    async fn infer(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError>;
    async fn health(&self) -> BackendStatus;
}
```

- [ ] **Step 3: Add `#[async_trait]` to MockBackend impl**

The `impl InferencePort for MockBackend` block needs the attribute:
```rust
#[async_trait]
impl InferencePort for MockBackend {
    // ... existing code unchanged
}
```

- [ ] **Step 4: Add `model_hint` to InferenceRequest**

In `backend.rs`, add to the `InferenceRequest` struct:
```rust
pub struct InferenceRequest {
    pub trace_id: String,
    pub system_prompt: String,
    pub context: String,
    pub num_branches: u32,
    pub temperature: f32,
    pub model_hint: Option<String>,  // NEW: preferred model, None = router decides
}
```

Update the test constructors to add `model_hint: None` to all `InferenceRequest` literals in the `#[cfg(test)]` block.

- [ ] **Step 5: Compile and run tests**

```bash
cd /home/kairos/CYNIC-validate && cargo test -p cynic-kernel -- backend
```
Expected: all 5 existing backend tests pass. `Box<dyn InferencePort>` is now valid.

- [ ] **Step 6: Commit**

```bash
cd /home/kairos/CYNIC-validate && git add -A && git commit -m "feat(backend): make InferencePort object-safe via async-trait + add model_hint"
```

---

### Task 2: Write LlamaCppBackend

**Files:**
- Create: `/home/kairos/CYNIC-validate/cynic-kernel/src/backend_llamacpp.rs`
- Modify: `/home/kairos/CYNIC-validate/cynic-kernel/src/main.rs` (add `pub mod backend_llamacpp;`)

- [ ] **Step 1: Write the failing test first**

Create `backend_llamacpp.rs` with the test at the bottom:

```rust
//! LlamaCppBackend — InferencePort implementation for llama.cpp's OpenAI-compatible HTTP API.

use crate::backend::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Calls llama.cpp's /v1/chat/completions endpoint.
pub struct LlamaCppBackend {
    endpoint: String,           // e.g. "http://127.0.0.1:11435"
    client: Client,
    capability: BackendCapability,
}

// -- OpenAI-compatible request/response structs for serde --

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    n: u32,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    model: String,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

#[derive(Deserialize)]
struct HealthResponse {
    status: String,
}

impl LlamaCppBackend {
    /// Create a new backend. Probes /v1/models to discover what's loaded.
    /// Returns Err if the server is unreachable after retries.
    pub async fn connect(endpoint: &str, backend_id: &str) -> Result<Self, BackendError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        // Cold-start polling: retry /health with backoff
        let mut delay = std::time::Duration::from_secs(1);
        let deadline = Instant::now() + std::time::Duration::from_secs(60);
        loop {
            match client.get(format!("{}/health", endpoint)).send().await {
                Ok(resp) if resp.status().is_success() => break,
                _ if Instant::now() + delay < deadline => {
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, std::time::Duration::from_secs(16));
                }
                _ => return Err(BackendError::Unreachable(backend_id.to_string())),
            }
        }

        // Discover loaded model
        let models: ModelsResponse = client
            .get(format!("{}/v1/models", endpoint))
            .send()
            .await
            .map_err(|e| BackendError::Protocol(e.to_string()))?
            .json()
            .await
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        let loaded_models: Vec<String> = models.data.iter().map(|m| m.id.clone()).collect();

        Ok(Self {
            endpoint: endpoint.to_string(),
            client,
            capability: BackendCapability {
                id: backend_id.to_string(),
                kind: BackendKind::Local,
                device_name: "llama.cpp".to_string(),
                vram_total_gb: 0.0,  // Unknown until probe wires in
                vram_available_gb: 0.0,
                latency_ms: 0.0,
                loaded_models,
            },
        })
    }
}

#[async_trait]
impl InferencePort for LlamaCppBackend {
    fn capability(&self) -> &BackendCapability {
        &self.capability
    }

    async fn infer(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError> {
        let start = Instant::now();

        let mut messages = Vec::new();
        if !req.system_prompt.is_empty() {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: req.system_prompt.clone(),
            });
        }
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: req.context.clone(),
        });

        // Use loaded model name, or model_hint if provided
        let model = req.model_hint.clone()
            .unwrap_or_else(|| {
                self.capability.loaded_models.first()
                    .cloned()
                    .unwrap_or_default()
            });

        let chat_req = ChatRequest {
            model: model.clone(),
            messages,
            temperature: req.temperature,
            n: req.num_branches.max(1),
        };

        let resp = self.client
            .post(format!("{}/v1/chat/completions", self.endpoint))
            .json(&chat_req)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    BackendError::Timeout {
                        backend_id: self.capability.id.clone(),
                        ms: start.elapsed().as_millis() as u64,
                    }
                } else {
                    BackendError::Unreachable(format!("{}: {}", self.capability.id, e))
                }
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(BackendError::Protocol(format!("HTTP {}: {}", status, body)));
        }

        let chat_resp: ChatResponse = resp
            .json()
            .await
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        let hypotheses: Vec<String> = chat_resp
            .choices
            .into_iter()
            .map(|c| c.message.content)
            .collect();

        Ok(InferenceResponse {
            trace_id: req.trace_id,
            hypotheses,
            latency_ms: start.elapsed().as_millis() as f64,
            model_used: chat_resp.model,
            backend_id: self.capability.id.clone(),
        })
    }

    async fn health(&self) -> BackendStatus {
        match self.client
            .get(format!("{}/health", self.endpoint))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => BackendStatus::Healthy,
            Ok(_) => BackendStatus::Degraded { latency_ms: 0.0 },
            Err(_) => BackendStatus::Unreachable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration test — requires a running llama-server on localhost:11435
    // Skip in CI with: cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn llama_backend_real_inference() {
        let backend = LlamaCppBackend::connect("http://127.0.0.1:11435", "local-llama")
            .await
            .expect("llama-server must be running on :11435");

        // Verify discovery
        assert!(!backend.capability().loaded_models.is_empty(),
            "Should discover at least one loaded model");

        // Health check
        assert_eq!(backend.health().await, BackendStatus::Healthy);

        // Real inference
        let req = InferenceRequest {
            trace_id: "test-001".to_string(),
            system_prompt: "Reply in one short sentence.".to_string(),
            context: "What is 2+2?".to_string(),
            num_branches: 1,
            temperature: 0.1,
            model_hint: None,
        };
        let resp = backend.infer(req).await.expect("Inference should succeed");

        assert!(!resp.hypotheses.is_empty(), "Should return at least one hypothesis");
        assert!(!resp.model_used.is_empty(), "Should report model name");
        assert!(resp.latency_ms > 0.0, "Should report positive latency");
        println!("[TEST] Model: {} | Latency: {}ms | Response: {}",
            resp.model_used, resp.latency_ms, resp.hypotheses[0]);
    }

    #[tokio::test]
    async fn llama_backend_unreachable() {
        let result = LlamaCppBackend::connect("http://127.0.0.1:99999", "bad-backend").await;
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Register the module**

In `main.rs`, add after `pub mod backend;`:
```rust
pub mod backend_llamacpp;
```

- [ ] **Step 3: Compile**

```bash
cd /home/kairos/CYNIC-validate && cargo build -p cynic-kernel
```
Expected: compiles clean.

- [ ] **Step 4: Run the unreachable test (no server needed)**

```bash
cd /home/kairos/CYNIC-validate && cargo test -p cynic-kernel -- backend_llamacpp::tests::llama_backend_unreachable
```
Expected: PASS

- [ ] **Step 5: Run the real inference test (needs llama-server)**

llama-server is on Windows localhost:11435. From forge, we need the Tailscale IP. If the test is running on the same machine as llama-server, run:
```bash
cd /home/kairos/CYNIC-validate && cargo test -p cynic-kernel -- backend_llamacpp::tests::llama_backend_real_inference --ignored --nocapture
```
If llama-server is remote (Windows via Tailscale), temporarily change the endpoint in the test to the Tailscale IP.

Expected: PASS with actual model response printed.

- [ ] **Step 6: Commit**

```bash
cd /home/kairos/CYNIC-validate && git add -A && git commit -m "feat(muscle): add LlamaCppBackend — InferencePort for llama.cpp HTTP API"
```

---

## Chunk 2: BackendRouter + hal.rs Rewrite (The Wiring)

### Task 3: Write BackendRouter

**Files:**
- Create: `/home/kairos/CYNIC-validate/cynic-kernel/src/router.rs`
- Modify: `/home/kairos/CYNIC-validate/cynic-kernel/src/main.rs` (add `pub mod router;`)

- [ ] **Step 1: Write router.rs with tests**

```rust
//! BackendRouter — selects and dispatches to registered InferencePort backends.

use crate::backend::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct BackendRouter {
    backends: Arc<RwLock<Vec<Arc<dyn InferencePort>>>>,
    round_robin: AtomicUsize,
}

impl BackendRouter {
    pub fn new(backends: Vec<Arc<dyn InferencePort>>) -> Self {
        Self {
            backends: Arc::new(RwLock::new(backends)),
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
                if b.capability().loaded_models.iter().any(|m| m.contains(hint.as_str())) {
                    if matches!(b.health().await, BackendStatus::Healthy | BackendStatus::Degraded { .. }) {
                        return b.infer(req).await;
                    }
                }
            }
            // model_hint didn't match any healthy backend — fall through to round-robin
        }

        // 2. Round-robin over healthy backends
        let count = backends.len();
        for offset in 0..count {
            let idx = (self.round_robin.fetch_add(1, Ordering::Relaxed) + offset) % count;
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
        let healthy: Vec<_> = {
            let mut h = Vec::new();
            for b in backends.iter() {
                if matches!(b.health().await, BackendStatus::Healthy | BackendStatus::Degraded { .. }) {
                    h.push(Arc::clone(b));
                }
            }
            h
        };
        drop(backends); // release read lock

        if healthy.is_empty() {
            return Err(BackendError::Unreachable("all backends unreachable".into()));
        }

        // Dispatch N requests round-robin across healthy backends
        let mut handles = Vec::new();
        for i in 0..n {
            let backend = Arc::clone(&healthy[i as usize % healthy.len()]);
            let branch_req = InferenceRequest {
                trace_id: format!("{}-branch-{}", req.trace_id, i),
                temperature: req.temperature + (i as f32 * 0.1), // slight variation
                ..req.clone()
            };
            handles.push(tokio::spawn(async move {
                backend.infer(branch_req).await
            }));
        }

        // Collect results — partial success is OK
        let mut all_hypotheses = Vec::new();
        let mut latency_ms = 0.0f64;
        let mut model_used = String::new();
        let mut backend_id = String::new();

        for handle in handles {
            if let Ok(Ok(resp)) = handle.await {
                all_hypotheses.extend(resp.hypotheses);
                latency_ms = latency_ms.max(resp.latency_ms); // worst-case latency
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

    #[tokio::test]
    async fn route_to_single_healthy_backend() {
        let mock = Arc::new(MockBackend::healthy()) as Arc<dyn InferencePort>;
        let router = BackendRouter::new(vec![mock]);

        let req = InferenceRequest {
            trace_id: "t1".into(),
            system_prompt: "".into(),
            context: "hello world test context here".into(),
            num_branches: 1,
            temperature: 0.5,
            model_hint: None,
        };

        let resp = router.route(req).await.unwrap();
        assert_eq!(resp.backend_id, "mock");
        assert!(!resp.hypotheses.is_empty());
    }

    #[tokio::test]
    async fn route_skips_unreachable_backend() {
        let dead = Arc::new(MockBackend::unreachable()) as Arc<dyn InferencePort>;
        let alive = Arc::new(MockBackend::healthy()) as Arc<dyn InferencePort>;
        let router = BackendRouter::new(vec![dead, alive]);

        let req = InferenceRequest {
            trace_id: "t2".into(),
            system_prompt: "".into(),
            context: "test fallback routing behavior".into(),
            num_branches: 1,
            temperature: 0.5,
            model_hint: None,
        };

        let resp = router.route(req).await.unwrap();
        assert_eq!(resp.backend_id, "mock"); // reached the healthy one
    }

    #[tokio::test]
    async fn route_with_no_backends_returns_error() {
        let router = BackendRouter::new(vec![]);
        let req = InferenceRequest {
            trace_id: "t3".into(),
            system_prompt: "".into(),
            context: "should fail with no backends".into(),
            num_branches: 1,
            temperature: 0.5,
            model_hint: None,
        };
        assert!(router.route(req).await.is_err());
    }

    #[tokio::test]
    async fn fan_out_collects_multiple_hypotheses() {
        let mock = Arc::new(MockBackend::healthy()) as Arc<dyn InferencePort>;
        let router = BackendRouter::new(vec![mock]);

        let req = InferenceRequest {
            trace_id: "t4".into(),
            system_prompt: "".into(),
            context: "fan out test across multiple branches".into(),
            num_branches: 3,
            temperature: 0.5,
            model_hint: None,
        };

        let resp = router.fan_out(req, 3).await.unwrap();
        // MockBackend returns 2 hypotheses per call, 3 branches = 6 total
        assert!(resp.hypotheses.len() >= 3, "Should have multiple hypotheses from fan-out");
    }

    #[tokio::test]
    async fn register_adds_backend_at_runtime() {
        let router = BackendRouter::new(vec![]);

        // Empty — should fail
        let req = InferenceRequest {
            trace_id: "t5".into(),
            system_prompt: "".into(),
            context: "before registration test case".into(),
            num_branches: 1,
            temperature: 0.5,
            model_hint: None,
        };
        assert!(router.route(req).await.is_err());

        // Register one
        router.register(Arc::new(MockBackend::healthy())).await;

        let req2 = InferenceRequest {
            trace_id: "t6".into(),
            system_prompt: "".into(),
            context: "after registration test case".into(),
            num_branches: 1,
            temperature: 0.5,
            model_hint: None,
        };
        assert!(router.route(req2).await.is_ok());
    }
}
```

- [ ] **Step 2: Register the module**

In `main.rs`, add after `pub mod backend_llamacpp;`:
```rust
pub mod router;
```

- [ ] **Step 3: Run router tests**

```bash
cd /home/kairos/CYNIC-validate && cargo test -p cynic-kernel -- router::tests
```
Expected: all 5 tests PASS.

- [ ] **Step 4: Commit**

```bash
cd /home/kairos/CYNIC-validate && git add -A && git commit -m "feat(muscle): add BackendRouter — route + fan_out with round-robin and health filtering"
```

---

### Task 4: Rewrite hal.rs — Connect BackendRouter to gRPC

**Files:**
- Modify: `/home/kairos/CYNIC-validate/cynic-kernel/src/hal.rs` (full rewrite)
- Modify: `/home/kairos/CYNIC-validate/cynic-kernel/src/main.rs` (change MuscleService construction)

- [ ] **Step 1: Rewrite hal.rs**

Replace the entire contents of `hal.rs` with:

```rust
//! MuscleHAL gRPC service — dispatches inference requests through BackendRouter.

use crate::cynic_v2::muscle_hal_server::MuscleHal;
use crate::cynic_v2::{
    MctsInferenceRequest, MctsInferenceResponse,
    HalProfile, PulseRequest, MessageMeta,
};
use crate::backend::{InferenceRequest, BackendStatus};
use crate::router::BackendRouter;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct MuscleService {
    router: Arc<BackendRouter>,
}

impl MuscleService {
    pub fn new(router: Arc<BackendRouter>) -> Self {
        Self { router }
    }
}

/// Map proto MCTSInferenceRequest → domain InferenceRequest
fn to_domain(req: &MctsInferenceRequest) -> InferenceRequest {
    InferenceRequest {
        trace_id: req.meta.as_ref().map(|m| m.trace_id.clone()).unwrap_or_default(),
        system_prompt: req.system_prompt.clone(),
        context: req.context.clone(),
        num_branches: req.num_branches.max(1) as u32,
        temperature: req.temperature,
        model_hint: None, // TODO: add model_hint field to proto in Task 6
    }
}

#[tonic::async_trait]
impl MuscleHal for MuscleService {
    async fn request_inference(
        &self,
        request: Request<MctsInferenceRequest>,
    ) -> Result<Response<MctsInferenceResponse>, Status> {
        let req = request.into_inner();
        let meta = req.meta.clone();
        let domain_req = to_domain(&req);
        let n = req.num_branches.max(1) as u32;

        let trace = meta.as_ref().map(|m| m.trace_id.as_str()).unwrap_or("none");
        println!("[MuscleHAL] Inference | trace={} branches={}", trace, n);

        let result = if n > 1 {
            self.router.fan_out(domain_req, n).await
        } else {
            self.router.route(domain_req).await
        };

        match result {
            Ok(resp) => {
                println!("[MuscleHAL] OK | model={} latency={}ms hypotheses={}",
                    resp.model_used, resp.latency_ms, resp.hypotheses.len());
                Ok(Response::new(MctsInferenceResponse {
                    meta,
                    hypotheses: resp.hypotheses,
                    latency_ms: resp.latency_ms as f32,
                    model_used: resp.model_used,
                }))
            }
            Err(e) => {
                println!("[MuscleHAL] ERROR | {}", e);
                Err(Status::unavailable(e.to_string()))
            }
        }
    }

    async fn get_active_hal(
        &self,
        request: Request<PulseRequest>,
    ) -> Result<Response<HalProfile>, Status> {
        let req = request.into_inner();
        // TODO: aggregate real data from router backends
        Ok(Response::new(HalProfile {
            meta: req.meta,
            backend: "llama.cpp".to_string(),
            gpu_name: "discovered-at-runtime".to_string(),
            vram_used_gb: 0.0,
            vram_total_gb: 0.0,
        }))
    }
}
```

- [ ] **Step 2: Update main.rs to wire BackendRouter into MuscleService**

Replace the muscle_service construction in `main.rs`. The relevant section:

Old:
```rust
let muscle_service = hal::MuscleService::new(Arc::clone(&storage));
```

New:
```rust
// ─── RING 1: Muscle HAL (Inference Router) ──────────────────
let llama_endpoint = std::env::var("CYNIC_LLAMA_ENDPOINT")
    .unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());

let router = {
    let r = router::BackendRouter::new(vec![]);
    let r = Arc::new(r);

    // Try to connect to llama-server (non-blocking — if it's not up, we start without it)
    let r_clone = Arc::clone(&r);
    let endpoint = llama_endpoint.clone();
    tokio::spawn(async move {
        match backend_llamacpp::LlamaCppBackend::connect(&endpoint, "local-llama").await {
            Ok(backend) => {
                println!("[Ring 1] LlamaCpp backend connected: {} | models: {:?}",
                    endpoint, backend.capability().loaded_models);
                r_clone.register(Arc::new(backend)).await;
            }
            Err(e) => {
                println!("[Ring 1] WARNING: LlamaCpp backend unavailable: {}", e);
                println!("[Ring 1] Kernel starts without inference. Set CYNIC_LLAMA_ENDPOINT to connect.");
            }
        }
    });

    r
};

let muscle_service = hal::MuscleService::new(Arc::clone(&router));
```

- [ ] **Step 3: Compile**

```bash
cd /home/kairos/CYNIC-validate && cargo build -p cynic-kernel
```
Expected: compiles clean.

- [ ] **Step 4: Commit**

```bash
cd /home/kairos/CYNIC-validate && git add -A && git commit -m "feat(hal): rewrite — route through BackendRouter instead of hardcoded stub"
```

---

## Chunk 3: End-to-End Smoke Test

### Task 5: Test the full pipeline with grpcurl

**Files:** None — this is a verification task.

- [ ] **Step 1: Start the kernel on forge**

```bash
cd /home/kairos/CYNIC-validate && CYNIC_LLAMA_ENDPOINT=http://127.0.0.1:11435 cargo run -p cynic-kernel
```

If llama-server is on Windows (not forge), use its Tailscale IP instead:
```bash
CYNIC_LLAMA_ENDPOINT=http://<windows-tailscale-ip>:11435 cargo run -p cynic-kernel
```

Expected output includes:
```
[Ring 1] LlamaCpp backend connected: http://...:11435 | models: ["Qwen3.5-2B-Q4_K_M.gguf"]
[Ring 1] Vascular Law enforced on [::1]:50051
```

- [ ] **Step 2: Install grpcurl if not present**

```bash
which grpcurl || (curl -sSL https://github.com/fullstorydev/grpcurl/releases/download/v1.9.1/grpcurl_1.9.1_linux_x86_64.tar.gz | tar xz -C /usr/local/bin/)
```

Or use `grpc_cli`, or a proto-based curl. The key is sending a gRPC request.

- [ ] **Step 3: Send an inference request**

```bash
grpcurl -plaintext -d '{
  "meta": {"trace_id": "smoke-test-001", "node_id": "forge"},
  "system_prompt": "Reply in one short sentence.",
  "context": "What is the capital of France?",
  "num_branches": 1,
  "temperature": 0.3
}' '[::1]:50051' cynic.v2.MuscleHAL/RequestInference
```

Expected: a real response from Qwen with `hypotheses` containing an actual answer, `model_used` = `"Qwen3.5-2B-Q4_K_M.gguf"`, and positive `latency_ms`.

- [ ] **Step 4: Test fan-out with num_branches=3**

```bash
grpcurl -plaintext -d '{
  "meta": {"trace_id": "fan-out-001", "node_id": "forge"},
  "system_prompt": "Give a creative one-sentence answer.",
  "context": "Why is the sky blue?",
  "num_branches": 3,
  "temperature": 0.5
}' '[::1]:50051' cynic.v2.MuscleHAL/RequestInference
```

Expected: `hypotheses` array with 3 entries (one per branch), each with a slightly different answer due to temperature variation.

- [ ] **Step 5: Document the result**

If both requests succeed: **the pipeline is live.** The stub is dead. Real inference flows through the kernel.

```bash
cd /home/kairos/CYNIC-validate && git add -A && git commit -m "test(smoke): verified end-to-end pipeline — kernel → router → llama.cpp → real inference" --allow-empty
```

---

## Chunk 4: Proto Extensions (model_hint field)

### Task 6: Add model_hint to MCTSInferenceRequest proto

**Files:**
- Modify: `/home/kairos/CYNIC-validate/protos/cynic.proto`
- Modify: `/home/kairos/CYNIC-validate/cynic-kernel/src/hal.rs` (wire model_hint)

- [ ] **Step 1: Add field 6 to MCTSInferenceRequest**

In `cynic.proto`, the `MCTSInferenceRequest` message, add after field 5:
```protobuf
message MCTSInferenceRequest {
    MessageMeta meta = 1;
    string system_prompt = 2;
    string context = 3;
    int32 num_branches = 4;
    float temperature = 5;
    string model_hint = 6;         // Preferred model name, empty = router decides
}
```

- [ ] **Step 2: Wire model_hint in hal.rs**

In `hal.rs`, update the `to_domain` function:
```rust
fn to_domain(req: &MctsInferenceRequest) -> InferenceRequest {
    InferenceRequest {
        trace_id: req.meta.as_ref().map(|m| m.trace_id.clone()).unwrap_or_default(),
        system_prompt: req.system_prompt.clone(),
        context: req.context.clone(),
        num_branches: req.num_branches.max(1) as u32,
        temperature: req.temperature,
        model_hint: if req.model_hint.is_empty() { None } else { Some(req.model_hint.clone()) },
    }
}
```

- [ ] **Step 3: Compile and test**

```bash
cd /home/kairos/CYNIC-validate && cargo build -p cynic-kernel && cargo test -p cynic-kernel
```

- [ ] **Step 4: Commit**

```bash
cd /home/kairos/CYNIC-validate && git add -A && git commit -m "feat(proto): add model_hint to MCTSInferenceRequest — runtime model selection"
```

---

## Summary: What Each Task Ships

| Task | What Dies | What Lives | Pipeline Status |
|------|-----------|------------|-----------------|
| 1 | RPITIT `impl Future` | `#[async_trait]` + `Box<dyn InferencePort>` works | Compiles |
| 2 | — | LlamaCppBackend talks to llama-server | HTTP bridge works |
| 3 | — | BackendRouter routes + fan-out | Selection logic works |
| 4 | `"Reflexive thought simulated."` | Real inference through gRPC | **Pipeline wired** |
| 5 | — | — | **Smoke tested end-to-end** |
| 6 | Hardcoded model | `model_hint` proto field | Model-agnostic |

After Task 5, the full pipeline is live: `gRPC → BackendRouter → LlamaCppBackend → llama-server → real response`.

Tasks 1-5 are the MVP. Task 6 is the first extension. Further work (MemoryGuard, supervisor refactor, Windows Service, VascularSystem telemetry) builds on this working pipeline.
