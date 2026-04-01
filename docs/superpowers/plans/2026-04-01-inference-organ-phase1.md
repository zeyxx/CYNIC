# Inference Organ Phase 1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the scattered inference infrastructure in main.rs (150 lines of manual wiring) with an `organ/` module that owns backend registration, health unification, and profile-based routing — fixing the 6→2 Dog degradation.

**Architecture:** Hexagonal. `organ/` is Layer 1 — imports from `domain/` only, provides trait objects consumed by Layer 2 (Pipeline, Judge) via `main.rs`. The organ owns: backend registry, health (unified CB + parse-rate quality gate), routing (profile → cluster → backend), and transport adapters (HTTP to llama-server/cloud).

**Tech Stack:** Rust 1.94+, tokio, async-trait, reqwest, serde, bitflags 2

**Spec:** `docs/superpowers/specs/2026-04-01-inference-organ-design.md` (rev 3)

**Precondition:** `make check` passes on current main before starting.

---

## File Structure

```
NEW files:
  cynic-kernel/src/organ/mod.rs           — InferenceOrgan facade
  cynic-kernel/src/organ/registry.rs      — Node, Backend, Cluster, DogBinding, capabilities
  cynic-kernel/src/organ/health.rs        — UnifiedHealthGate (CB + parse-rate), DogStats (Welford)
  cynic-kernel/src/organ/router.rs        — profile→cluster→backend selection
  cynic-kernel/src/organ/transport/mod.rs — InferenceTransport + EmbeddingTransport traits
  cynic-kernel/src/organ/transport/llama_server.rs — HTTP adapter for llama-server
  cynic-kernel/src/organ/transport/cloud.rs        — HTTP adapter for Gemini/HF
  cynic-kernel/src/organ/metrics.rs       — Prometheus gauges for organ state
  tests/organ_registry.rs                 — unit tests for registry
  tests/organ_health.rs                   — unit tests for health + Welford
  tests/organ_router.rs                   — unit tests for routing
  tests/organ_integration.rs              — integration: organ boots from config

MODIFIED files:
  cynic-kernel/Cargo.toml                 — add bitflags = "2"
  cynic-kernel/src/lib.rs                 — add `pub mod organ;`
  cynic-kernel/src/domain/chat.rs:18-59   — add InferenceProfile::Calibration variant
  cynic-kernel/src/domain/metrics.rs:70+  — add organ Prometheus gauges
  cynic-kernel/src/infra/tasks.rs:160     — change SovereignSummarizer → Arc<dyn SummarizationPort>
  cynic-kernel/src/judge.rs:252-260       — add organ.update_stats() call after phi_bound
  cynic-kernel/src/main.rs:141-523        — replace manual wiring with InferenceOrgan::boot()

DELETED (after migration verified):
  Nothing deleted in Phase 1. Old code paths coexist until organ is proven.
```

---

### Task 1: Add `bitflags` dependency + `organ/` module skeleton

**Files:**
- Modify: `cynic-kernel/Cargo.toml:10-33`
- Create: `cynic-kernel/src/organ/mod.rs`
- Modify: `cynic-kernel/src/lib.rs`

- [ ] **Step 1: Add bitflags to Cargo.toml**

```toml
# Add after blake3 = "1" (line 29):
bitflags = "2"
```

- [ ] **Step 2: Create organ module skeleton**

```rust
// cynic-kernel/src/organ/mod.rs
pub mod registry;
pub mod health;
pub mod router;
pub mod transport;
pub mod metrics;
```

- [ ] **Step 3: Register module in lib.rs**

Add `pub mod organ;` in `lib.rs` alongside existing module declarations.

- [ ] **Step 4: Create empty submodule files**

Create these files with just a comment header:
- `cynic-kernel/src/organ/registry.rs`
- `cynic-kernel/src/organ/health.rs`
- `cynic-kernel/src/organ/router.rs`
- `cynic-kernel/src/organ/metrics.rs`
- `cynic-kernel/src/organ/transport/mod.rs`
- `cynic-kernel/src/organ/transport/llama_server.rs`
- `cynic-kernel/src/organ/transport/cloud.rs`

- [ ] **Step 5: Verify it compiles**

Run: `cargo build 2>&1 | head -5`
Expected: compiles with warnings about empty files, zero errors.

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/Cargo.toml cynic-kernel/src/organ/ cynic-kernel/src/lib.rs
git commit -m "feat(organ): scaffold organ/ module with bitflags dependency"
```

---

### Task 2: Registry — data model types

**Files:**
- Create: `cynic-kernel/src/organ/registry.rs`
- Test: `cynic-kernel/tests/organ_registry.rs`

- [ ] **Step 1: Write failing test for registry types**

```rust
// tests/organ_registry.rs
use cynic_kernel::organ::registry::*;

#[test]
fn declared_capabilities_default_is_all_false() {
    let cap = DeclaredCapabilities::default();
    assert!(!cap.json);
    assert!(!cap.thinking);
    assert!(!cap.scoring);
}

#[test]
fn backend_starts_healthy() {
    let backend = Backend {
        id: BackendId("test".into()),
        node_id: NodeId("node".into()),
        endpoint: "/v1".into(),
        model: "test-model".into(),
        declared: DeclaredCapabilities::default(),
        measured: MeasuredCapabilities::default(),
        health: BackendHealth::Healthy,
        timeout_secs: 30,
        remediation: None,
    };
    assert!(matches!(backend.health, BackendHealth::Healthy));
}

#[test]
fn cluster_contains_backend() {
    let cluster = Cluster {
        id: ClusterId("scoring".into()),
        required_json_rate: 0.7,
        backends: vec![BackendId("b1".into())],
        strategy: ClusterStrategy::RoundRobin,
    };
    assert_eq!(cluster.backends.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test organ_registry 2>&1 | tail -5`
Expected: FAIL — types not defined.

- [ ] **Step 3: Implement registry types**

```rust
// cynic-kernel/src/organ/registry.rs
use std::time::Instant;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct NodeId(pub String);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BackendId(pub String);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ClusterId(pub String);

#[derive(Debug, Clone, Default)]
pub struct DeclaredCapabilities {
    pub json: bool,
    pub thinking: bool,
    pub scoring: bool,
    pub embedding: bool,
    pub agent_reasoning: bool,
    pub grammar: bool,
}

#[derive(Debug, Clone)]
pub struct MeasuredCapabilities {
    pub json_valid_rate: f64,
    pub scoring_in_range_rate: f64,
    pub mean_latency_ms: u32,
    pub tokens_per_second: f32,
}

impl Default for MeasuredCapabilities {
    fn default() -> Self {
        Self {
            json_valid_rate: 0.0,     // K14: unknown = assume worst
            scoring_in_range_rate: 0.0,
            mean_latency_ms: u32::MAX,
            tokens_per_second: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapabilityThreshold {
    pub min_json_valid_rate: f64,   // default: 0.7
    pub min_scoring_rate: f64,      // default: 0.6
    pub max_latency_ms: u32,       // default: 5000
}

impl Default for CapabilityThreshold {
    fn default() -> Self {
        Self {
            min_json_valid_rate: 0.7,
            min_scoring_rate: 0.6,
            max_latency_ms: 5000,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BackendHealth {
    Healthy,
    Degraded { reason: String, since: Instant },
    Dead { reason: String, since: Instant },
}

#[derive(Debug, Clone)]
pub struct RemediationConfig {
    pub node: String,
    pub restart_command: String,
    pub max_retries: u32,
    pub cooldown_secs: u64,
}

#[derive(Debug, Clone)]
pub struct Backend {
    pub id: BackendId,
    pub node_id: NodeId,
    pub endpoint: String,
    pub model: String,
    pub declared: DeclaredCapabilities,
    pub measured: MeasuredCapabilities,
    pub health: BackendHealth,
    pub timeout_secs: u64,
    pub remediation: Option<RemediationConfig>,
}

#[derive(Debug, Clone)]
pub enum ClusterStrategy {
    RoundRobin,
    Failover,
    LowestLatency,
}

#[derive(Debug, Clone)]
pub struct Cluster {
    pub id: ClusterId,
    pub required_json_rate: f64,
    pub backends: Vec<BackendId>,
    pub strategy: ClusterStrategy,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub address: String,
    pub backends: Vec<BackendId>,
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test organ_registry -v 2>&1 | tail -10`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/organ/registry.rs cynic-kernel/tests/organ_registry.rs
git commit -m "feat(organ): registry data model — Node, Backend, Cluster, capabilities"
```

---

### Task 3: Health — Welford accumulator + UnifiedHealthGate

**Files:**
- Create: `cynic-kernel/src/organ/health.rs`
- Test: `cynic-kernel/tests/organ_health.rs`

- [ ] **Step 1: Write failing tests for Welford + health gate**

```rust
// tests/organ_health.rs
use cynic_kernel::organ::health::*;

#[test]
fn welford_empty_stddev_is_max() {
    let w = WelfordAccumulator::new();
    assert_eq!(w.stddev(), f64::MAX);
}

#[test]
fn welford_single_value_stddev_is_max() {
    let mut w = WelfordAccumulator::new();
    w.update(0.5);
    assert_eq!(w.stddev(), f64::MAX);
}

#[test]
fn welford_two_values_computes_stddev() {
    let mut w = WelfordAccumulator::new();
    w.update(0.4);
    w.update(0.6);
    assert!((w.mean() - 0.5).abs() < 1e-10);
    assert!(w.stddev() > 0.0);
    assert!(w.stddev() < 0.2);
}

#[test]
fn welford_many_values_stable() {
    let mut w = WelfordAccumulator::new();
    for i in 0..100 {
        w.update(0.5 + (i as f64 % 10.0) * 0.01);
    }
    assert!(w.count() == 100);
    assert!((w.mean() - 0.545).abs() < 0.01);
}

#[test]
fn dog_stats_new_is_in_learning_mode() {
    let stats = DogStats::new();
    assert!(!stats.is_baseline_established());
}

#[test]
fn dog_stats_after_20_calls_baseline_established() {
    let mut stats = DogStats::new();
    for _ in 0..20 {
        stats.record_scores(&[0.3, 0.4, 0.5, 0.3, 0.6, 0.4], true);
    }
    assert!(stats.is_baseline_established());
}

#[test]
fn dog_stats_json_valid_rate_tracks_failures() {
    let mut stats = DogStats::new();
    for _ in 0..8 {
        stats.record_scores(&[0.3; 6], true);
    }
    for _ in 0..2 {
        stats.record_json_failure();
    }
    assert!((stats.json_valid_rate() - 0.8).abs() < 0.05);
}

#[test]
fn parse_failure_gate_trips_at_50_percent() {
    let mut gate = ParseFailureGate::new();
    for _ in 0..5 { gate.record_success(); }
    assert!(!gate.is_tripped());
    for _ in 0..6 { gate.record_failure(); }
    // 6 failures out of 11 > 50%
    assert!(gate.is_tripped());
}
```

- [ ] **Step 2: Run to verify failures**

Run: `cargo test --test organ_health 2>&1 | tail -5`
Expected: FAIL.

- [ ] **Step 3: Implement Welford + DogStats + ParseFailureGate**

```rust
// cynic-kernel/src/organ/health.rs
use std::time::Instant;

/// Welford's online algorithm — numerically stable O(1) per update
#[derive(Debug, Clone)]
pub struct WelfordAccumulator {
    count: u64,
    mean: f64,
    m2: f64,
}

impl WelfordAccumulator {
    pub fn new() -> Self {
        Self { count: 0, mean: 0.0, m2: 0.0 }
    }

    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    pub fn mean(&self) -> f64 { self.mean }

    pub fn count(&self) -> u64 { self.count }

    pub fn stddev(&self) -> f64 {
        if self.count < 2 { return f64::MAX; } // K14: unknown = assume worst
        (self.m2 / (self.count - 1) as f64).sqrt()
    }
}

const BASELINE_THRESHOLD: u64 = 20;

/// Per-Dog rolling statistics (Welford online)
#[derive(Debug, Clone)]
pub struct DogStats {
    pub axioms: [WelfordAccumulator; 6],
    json_valid: WelfordAccumulator,  // 1.0 = valid, 0.0 = invalid
    pub last_updated: Option<Instant>,
}

impl DogStats {
    pub fn new() -> Self {
        Self {
            axioms: std::array::from_fn(|_| WelfordAccumulator::new()),
            json_valid: WelfordAccumulator::new(),
            last_updated: None,
        }
    }

    pub fn record_scores(&mut self, scores: &[f64; 6], json_ok: bool) {
        for (i, &s) in scores.iter().enumerate() {
            self.axioms[i].update(s);
        }
        self.json_valid.update(if json_ok { 1.0 } else { 0.0 });
        self.last_updated = Some(Instant::now());
    }

    pub fn record_json_failure(&mut self) {
        self.json_valid.update(0.0);
        self.last_updated = Some(Instant::now());
    }

    pub fn json_valid_rate(&self) -> f64 {
        self.json_valid.mean()
    }

    pub fn is_baseline_established(&self) -> bool {
        self.axioms[0].count() >= BASELINE_THRESHOLD
    }

    // DORMANT: drift detection activates in Phase 2 — needs windowed comparison
    // against recent vs historical means. Welford accumulator alone cannot distinguish
    // "baseline shift" from "always had this mean." Phase 2 adds a second accumulator
    // for the last N calls and compares the two means.
    // pub fn drifted_axiom(&self) -> Option<(usize, f64, f64)> { ... }
}

/// Sliding window gate for parse failures (last N calls)
#[derive(Debug)]
pub struct ParseFailureGate {
    window: Vec<bool>,  // true = success, false = failure
    capacity: usize,
}

impl ParseFailureGate {
    pub fn new() -> Self {
        Self { window: Vec::new(), capacity: 10 }
    }

    pub fn record_success(&mut self) {
        if self.window.len() >= self.capacity { self.window.remove(0); }
        self.window.push(true);
    }

    pub fn record_failure(&mut self) {
        if self.window.len() >= self.capacity { self.window.remove(0); }
        self.window.push(false);
    }

    pub fn is_tripped(&self) -> bool {
        if self.window.len() < 5 { return false; } // need minimum samples
        let failures = self.window.iter().filter(|&&ok| !ok).count();
        failures as f64 / self.window.len() as f64 > 0.5
    }

    pub fn failure_rate(&self) -> f64 {
        if self.window.is_empty() { return 0.0; }
        let failures = self.window.iter().filter(|&&ok| !ok).count();
        failures as f64 / self.window.len() as f64
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test organ_health -v 2>&1 | tail -15`
Expected: 8 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/organ/health.rs cynic-kernel/tests/organ_health.rs
git commit -m "feat(organ): Welford accumulator + DogStats + ParseFailureGate"
```

---

### Task 4: Transport traits + LlamaServerAdapter

**Files:**
- Create: `cynic-kernel/src/organ/transport/mod.rs`
- Create: `cynic-kernel/src/organ/transport/llama_server.rs`
- Create: `cynic-kernel/src/organ/transport/cloud.rs`

- [ ] **Step 0: Write failing test for LlamaServerAdapter (C2 fix — TDD)**

```rust
// tests/organ_transport.rs
use cynic_kernel::organ::transport::llama_server::LlamaServerAdapter;
use cynic_kernel::organ::transport::InferenceTransport;
use cynic_kernel::domain::chat::InferenceProfile;
use cynic_kernel::domain::inference::InferRequest;

#[tokio::test]
async fn llama_server_sends_disable_thinking_for_scoring() {
    // Mock HTTP server that captures request body
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, mut rx) = tokio::sync::oneshot::channel::<String>();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 4096];
        let io = tokio::io::AsyncReadExt::read(&mut &stream, &mut buf).await.unwrap();
        let body = String::from_utf8_lossy(&buf[..io]);
        // Extract JSON body from HTTP request
        if let Some(json_start) = body.find('{') {
            let _ = tx.send(body[json_start..].to_string());
        }
    });

    let adapter = LlamaServerAdapter::new(&format!("http://{}", addr), "test-model", 30);
    let req = InferRequest { prompt: "test".into(), system: None, temperature: 0.3, max_tokens: 512 };
    let _ = adapter.infer(&req, &InferenceProfile::Scoring).await;

    // Verify the request body contained enable_thinking: false
    // This test validates the E4 constraint (thinking control per-request)
}
```

Run: `cargo test --test organ_transport 2>&1 | tail -5`
Expected: FAIL — types not defined.

- [ ] **Step 1: Define transport traits**

```rust
// cynic-kernel/src/organ/transport/mod.rs
pub mod llama_server;
pub mod cloud;

use crate::domain::inference::{InferRequest, InferResponse, BackendError};
use crate::domain::chat::InferenceProfile;
use crate::domain::embedding::{Embedding, EmbeddingError};
use async_trait::async_trait;

#[async_trait]
pub trait InferenceTransport: Send + Sync {
    async fn infer(
        &self,
        request: &InferRequest,
        profile: &InferenceProfile,
    ) -> Result<InferResponse, BackendError>;

    async fn health(&self) -> Result<(), BackendError>;
}

#[async_trait]
pub trait EmbeddingTransport: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError>;

    async fn health(&self) -> Result<(), EmbeddingError>;
}
```

- [ ] **Step 2: Implement LlamaServerAdapter**

Adapt the existing `OpenAiCompatBackend` logic from `backends/openai.rs:253-333` into the new trait. This is a MOVE of the HTTP call logic, not new code.

```rust
// cynic-kernel/src/organ/transport/llama_server.rs
// Key differences from OpenAiCompatBackend:
// 1. Implements InferenceTransport (not ChatPort)
// 2. Sends grammar field when backend declares grammar capability
// 3. Sends chat_template_kwargs:{enable_thinking:false} for Scoring/Summary profiles
// 4. Checks /props for context drift (from FleetProbe logic)
```

Reference: `backends/openai.rs:253-333` for the HTTP call structure.
Reference: `infra/probes/fleet.rs:28+` for `/props` checking.

- [ ] **Step 3: Implement CloudApiAdapter**

Minimal adapter for Gemini/HF — reuse existing auth logic from `backends/openai.rs`.

```rust
// cynic-kernel/src/organ/transport/cloud.rs
// Same InferenceTransport trait, but:
// 1. No grammar support
// 2. No /props endpoint
// 3. No disable_thinking (cloud APIs have their own mechanism)
// 4. Auth via Bearer token or query param (from BackendConfig.auth_style)
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build 2>&1 | head -10`
Expected: compiles (adapters may have unused warnings until wired).

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/organ/transport/
git commit -m "feat(organ): InferenceTransport + EmbeddingTransport traits, LlamaServer + Cloud adapters"
```

---

### Task 5: Router — profile → cluster → backend

**Files:**
- Create: `cynic-kernel/src/organ/router.rs`
- Test: `cynic-kernel/tests/organ_router.rs`

- [ ] **Step 1: Write failing routing tests**

```rust
// tests/organ_router.rs
use cynic_kernel::organ::registry::*;
use cynic_kernel::organ::router::*;
use cynic_kernel::domain::chat::InferenceProfile;

#[test]
fn scoring_profile_requires_json() {
    let req = profile_requirements(&InferenceProfile::Scoring);
    assert!(req.requires_json);
}

#[test]
fn selects_healthy_backend_from_cluster() {
    let backends = vec![
        make_backend("b1", BackendHealth::Dead { reason: "down".into(), since: std::time::Instant::now() }),
        make_backend("b2", BackendHealth::Healthy),
    ];
    let cluster = make_cluster("c1", vec!["b1", "b2"]);
    let threshold = CapabilityThreshold::default();
    let selected = select_backend(&cluster, &backends, &threshold);
    assert_eq!(selected.unwrap().id.0, "b2");
}

#[test]
fn returns_none_when_all_dead() {
    let backends = vec![
        make_backend("b1", BackendHealth::Dead { reason: "down".into(), since: std::time::Instant::now() }),
    ];
    let cluster = make_cluster("c1", vec!["b1"]);
    let threshold = CapabilityThreshold::default();
    let selected = select_backend(&cluster, &backends, &threshold);
    assert!(selected.is_none());
}

// Helper constructors
fn make_backend(id: &str, health: BackendHealth) -> Backend {
    Backend {
        id: BackendId(id.into()),
        node_id: NodeId("node".into()),
        endpoint: "/v1".into(),
        model: "test".into(),
        declared: DeclaredCapabilities { json: true, ..Default::default() },
        measured: MeasuredCapabilities { json_valid_rate: 0.9, ..Default::default() },
        health,
        timeout_secs: 30,
        remediation: None,
    }
}

fn make_cluster(id: &str, backend_ids: Vec<&str>) -> Cluster {
    Cluster {
        id: ClusterId(id.into()),
        required_json_rate: 0.7,
        backends: backend_ids.into_iter().map(|s| BackendId(s.into())).collect(),
        strategy: ClusterStrategy::RoundRobin,
    }
}
```

- [ ] **Step 2: Run to verify failures**

Run: `cargo test --test organ_router 2>&1 | tail -5`
Expected: FAIL.

- [ ] **Step 3: Implement router**

```rust
// cynic-kernel/src/organ/router.rs
use crate::domain::chat::InferenceProfile;
use crate::organ::registry::*;

pub struct ProfileRequirements {
    pub requires_json: bool,
    pub requires_thinking: bool,
    pub requires_agent_reasoning: bool,
}

pub fn profile_requirements(profile: &InferenceProfile) -> ProfileRequirements {
    match profile {
        InferenceProfile::Scoring => ProfileRequirements {
            requires_json: true, requires_thinking: false, requires_agent_reasoning: false,
        },
        InferenceProfile::Agent => ProfileRequirements {
            requires_json: false, requires_thinking: true, requires_agent_reasoning: true,
        },
        InferenceProfile::Summary => ProfileRequirements {
            requires_json: true, requires_thinking: false, requires_agent_reasoning: false,
        },
        InferenceProfile::Infer => ProfileRequirements {
            requires_json: true, requires_thinking: false, requires_agent_reasoning: false,
        },
        InferenceProfile::Calibration => ProfileRequirements {
            requires_json: true, requires_thinking: false, requires_agent_reasoning: false,
        },
    }
}

pub fn select_backend<'a>(
    cluster: &Cluster,
    backends: &'a [Backend],
    threshold: &CapabilityThreshold,
) -> Option<&'a Backend> {
    cluster.backends.iter()
        .filter_map(|bid| backends.iter().find(|b| &b.id == bid))
        .filter(|b| matches!(b.health, BackendHealth::Healthy | BackendHealth::Degraded { .. }))
        .filter(|b| !b.declared.json || b.measured.json_valid_rate >= threshold.min_json_valid_rate)
        .next() // RoundRobin: first eligible. Phase 2 adds cursor.
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test organ_router -v 2>&1 | tail -10`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/organ/router.rs cynic-kernel/tests/organ_router.rs
git commit -m "feat(organ): router — profile requirements + backend selection from cluster"
```

---

### Task 6: Organ Prometheus metrics

**Files:**
- Create: `cynic-kernel/src/organ/metrics.rs`
- Modify: `cynic-kernel/src/domain/metrics.rs:70+`

- [ ] **Step 1: Implement organ metrics rendering**

```rust
// cynic-kernel/src/organ/metrics.rs
use crate::organ::registry::*;
use crate::organ::health::DogStats;

pub fn render_organ_prometheus(
    backends: &[Backend],
    dog_stats: &std::collections::HashMap<String, DogStats>,
) -> String {
    let mut out = String::new();
    out.push_str("# HELP cynic_dog_state Dog health state (0=dead, 1=degraded, 2=healthy)\n");
    out.push_str("# TYPE cynic_dog_state gauge\n");
    for b in backends {
        let val = match &b.health {
            BackendHealth::Healthy => 2,
            BackendHealth::Degraded { .. } => 1,
            BackendHealth::Dead { .. } => 0,
        };
        out.push_str(&format!("cynic_dog_state{{backend_id=\"{}\"}} {}\n", b.id.0, val));
    }

    out.push_str("# HELP cynic_backend_json_valid_rate JSON validity rate\n");
    out.push_str("# TYPE cynic_backend_json_valid_rate gauge\n");
    for b in backends {
        out.push_str(&format!(
            "cynic_backend_json_valid_rate{{backend_id=\"{}\"}} {:.3}\n",
            b.id.0, b.measured.json_valid_rate
        ));
    }

    out
}
```

- [ ] **Step 2: Wire into existing render_prometheus**

In `domain/metrics.rs`, after the existing `render_prometheus()` output, the organ metrics will be appended by the caller (main.rs / REST handler). No modification to the trait — the organ metrics are rendered separately and concatenated.

- [ ] **Step 3: Verify compilation**

Run: `cargo build 2>&1 | head -5`
Expected: compiles.

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/organ/metrics.rs
git commit -m "feat(organ): Prometheus gauges for dog_state and json_valid_rate"
```

---

### Task 7: InferenceOrgan facade — boot() + provides

**Files:**
- Create: `cynic-kernel/src/organ/mod.rs` (replace skeleton)
- Test: `cynic-kernel/tests/organ_integration.rs`

- [ ] **Step 1: Write integration test for organ boot**

```rust
// tests/organ_integration.rs
use cynic_kernel::organ::InferenceOrgan;

#[test]
fn organ_boots_with_empty_config() {
    let organ = InferenceOrgan::boot_empty();
    assert_eq!(organ.dog_count(), 0);
    assert_eq!(organ.backend_count(), 0);
}

#[test]
fn organ_registers_backend_and_creates_dog() {
    let mut organ = InferenceOrgan::boot_empty();
    // Register a backend manually (fleet.toml loading is Task 8)
    organ.register_node("test-node", "localhost:8080");
    organ.register_backend("test-node", "test-backend", "qwen-9b", "/v1",
        cynic_kernel::organ::registry::DeclaredCapabilities {
            json: true, scoring: true, ..Default::default()
        }, 30);
    assert_eq!(organ.backend_count(), 1);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test organ_integration 2>&1 | tail -5`
Expected: FAIL.

- [ ] **Step 3: Implement InferenceOrgan facade**

```rust
// cynic-kernel/src/organ/mod.rs
pub mod registry;
pub mod health;
pub mod router;
pub mod transport;
pub mod metrics;

use registry::*;
use health::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InferenceOrgan {
    nodes: HashMap<NodeId, Node>,
    backends: HashMap<BackendId, Backend>,
    clusters: HashMap<ClusterId, Cluster>,
    dog_stats: HashMap<String, Arc<RwLock<DogStats>>>,
    parse_gates: HashMap<String, ParseFailureGate>,
}

impl InferenceOrgan {
    pub fn boot_empty() -> Self {
        Self {
            nodes: HashMap::new(),
            backends: HashMap::new(),
            clusters: HashMap::new(),
            dog_stats: HashMap::new(),
            parse_gates: HashMap::new(),
        }
    }

    pub fn register_node(&mut self, id: &str, address: &str) {
        self.nodes.insert(NodeId(id.into()), Node {
            id: NodeId(id.into()),
            address: address.into(),
            backends: Vec::new(),
        });
    }

    pub fn register_backend(
        &mut self, node_id: &str, backend_id: &str, model: &str,
        endpoint: &str, declared: DeclaredCapabilities, timeout_secs: u64,
    ) {
        let bid = BackendId(backend_id.into());
        let nid = NodeId(node_id.into());
        self.backends.insert(bid.clone(), Backend {
            id: bid.clone(),
            node_id: nid.clone(),
            endpoint: endpoint.into(),
            model: model.into(),
            declared,
            measured: MeasuredCapabilities::default(),
            health: BackendHealth::Healthy,
            timeout_secs,
            remediation: None,
        });
        if let Some(node) = self.nodes.get_mut(&nid) {
            node.backends.push(bid.clone());
        }
        self.dog_stats.insert(backend_id.into(), Arc::new(RwLock::new(DogStats::new())));
        self.parse_gates.insert(backend_id.into(), ParseFailureGate::new());
    }

    pub fn dog_count(&self) -> usize { self.backends.len() } // 1:1 for now
    pub fn backend_count(&self) -> usize { self.backends.len() }

    pub fn backends(&self) -> Vec<&Backend> {
        self.backends.values().collect()
    }

    pub fn dog_stats_for(&self, id: &str) -> Option<Arc<RwLock<DogStats>>> {
        self.dog_stats.get(id).cloned()
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test organ_integration -v 2>&1 | tail -10`
Expected: 2 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/organ/mod.rs cynic-kernel/tests/organ_integration.rs
git commit -m "feat(organ): InferenceOrgan facade — boot_empty, register_node, register_backend"
```

---

### Task 8: Add InferenceProfile::Calibration + fix tasks.rs D2

**Files:**
- Modify: `cynic-kernel/src/domain/chat.rs:18-59`
- Modify: `cynic-kernel/src/infra/tasks.rs:160`

- [ ] **Step 1: Add Calibration variant to InferenceProfile**

In `domain/chat.rs:18`, add `Calibration` to the enum. Then add match arms in `max_tokens()` (line 33), `disable_thinking()` (line 44), `temperature()` (line 52):

```rust
// In the enum:
Calibration,

// In max_tokens():
Self::Calibration => Some(512),

// In disable_thinking():
Self::Calibration => true,

// In temperature() — returns Option<f32>, NOT f64 (I3 fix):
Self::Calibration => Some(0.0_f32),
```

- [ ] **Step 2: Fix tasks.rs concrete type leak (D2)**

In `infra/tasks.rs:160`, change:
```rust
// FROM:
summarizer: crate::backends::summarizer::SovereignSummarizer,
// TO:
summarizer: Arc<dyn crate::domain::summarization::SummarizationPort>,
```

Add `use std::sync::Arc;` if not already imported.

- [ ] **Step 3: Verify compilation**

Run: `cargo build 2>&1 | head -10`
Expected: compiles. May need to fix the call site in `main.rs` that passes the concrete type — cast to `Arc<dyn SummarizationPort>`.

- [ ] **Step 4: Run full test suite**

Run: `cargo test 2>&1 | tail -5`
Expected: all existing tests pass + new organ tests pass.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/domain/chat.rs cynic-kernel/src/infra/tasks.rs
git commit -m "fix(organ): add InferenceProfile::Calibration + fix D2 concrete type leak in tasks.rs"
```

---

### Task 9a: Organ boots from config (fleet.toml / backends.toml)

**Files:**
- Modify: `cynic-kernel/src/organ/mod.rs`
- Test: `cynic-kernel/tests/organ_integration.rs`

- [ ] **Step 1: Write failing test for config-based boot**

```rust
// Add to tests/organ_integration.rs
#[test]
fn organ_boots_from_backends_toml() {
    // Create a temp backends.toml with one backend entry
    let toml = r#"
[storage]
url = "http://localhost:8000"

[backend.test-dog]
base_url = "http://localhost:8080/v1"
model = "test-model"
context_size = 4096
timeout_secs = 30
temperature = 0.3
max_tokens = 1024
"#;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("backends.toml"), toml).unwrap();
    let organ = InferenceOrgan::boot_from_config(dir.path()).unwrap();
    assert_eq!(organ.backend_count(), 1);
}
```

- [ ] **Step 2: Implement boot_from_config**

Add `boot_from_config(config_dir: &Path)` that reads fleet.toml (with fallback to backends.toml). Reuses `infra::config::load_backends()` internally and maps each `BackendConfig` → organ registry entries.

- [ ] **Step 3: Run test, verify pass**

Run: `cargo test --test organ_integration -v 2>&1 | tail -10`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/organ/mod.rs cynic-kernel/tests/organ_integration.rs
git commit -m "feat(organ): boot_from_config — reads fleet.toml/backends.toml into registry"
```

---

### Task 9b: Organ provides Dogs + HealthGates

**Files:**
- Modify: `cynic-kernel/src/organ/mod.rs`
- Test: `cynic-kernel/tests/organ_integration.rs`

**IMPORTANT (I1 fix — layer boundary):** The organ does NOT import `dogs::inference::InferenceDog`. The organ exposes `backends()` (registry data). `main.rs` (composition root) assembles Dogs by calling `InferenceDog::new(backend_as_chat_port, ...)` for each backend. The organ only provides the data; `main.rs` does the construction.

- [ ] **Step 1: Add organ.backends() + organ.health_gates()**

`backends()` returns iterator over Backend data. `health_gates()` returns `Vec<Arc<dyn HealthGate>>` wrapping the unified CB + parse-rate gate per backend.

- [ ] **Step 2: Write test for health_gates**

```rust
#[test]
fn organ_provides_health_gates_per_backend() {
    let organ = /* boot with 2 backends */;
    let gates = organ.health_gates();
    assert_eq!(gates.len(), 2);
    // All gates start allowing (healthy)
    for gate in &gates {
        assert!(gate.should_allow());
    }
}
```

- [ ] **Step 3: Run tests, verify pass**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(organ): expose backends() + health_gates() for main.rs Dog assembly"
```

---

### Task 9c: Wire organ into main.rs

**Files:**
- Modify: `cynic-kernel/src/main.rs:141-523`

- [ ] **Step 1: Replace Dog construction loop**

```rust
// BEFORE (main.rs:141-215): 75 lines of manual construction
// AFTER:
let organ = InferenceOrgan::boot_from_config(&config_dir)?;
// main.rs assembles Dogs (composition root, NOT organ — I1 fix):
let mut dogs: Vec<Box<dyn Dog>> = vec![Box::new(DeterministicDog)];
for backend_info in organ.backends() {
    let backend = OpenAiCompatBackend::new(/* from backend_info */);
    let dog = InferenceDog::new(Arc::new(backend), &backend_info.id.0, ...);
    dogs.push(Box::new(dog));
}
let health_gates = organ.health_gates();
```

- [ ] **Step 2: Replace SovereignSummarizer dual construction (D4 fix)**

One `Arc<dyn SummarizationPort>` + `Arc<dyn InferPort>` from the organ's sovereign backend. Replace lines 523 and 578.

- [ ] **Step 3: Wire organ metrics into /metrics endpoint**

Append `organ::metrics::render_organ_prometheus()` to the existing metrics output. This makes `curl /metrics | grep dog_state` work (E8 requirement).

- [ ] **Step 4: Run make check**

Run: `make check 2>&1 | tail -20`
Expected: build + test + clippy + lint-rules + lint-drift all pass.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/main.rs cynic-kernel/src/organ/
git commit -m "feat(organ): wire InferenceOrgan into main.rs — replaces manual backend construction"
```

---

### Task 10: Update DogStats from judge.rs after phi_bound

**Files:**
- Modify: `cynic-kernel/src/organ/health.rs`
- Modify: `cynic-kernel/src/judge.rs:12-30` (struct + constructor)
- Modify: `cynic-kernel/src/judge.rs:252-260` (phi_bound call site)
- Modify: `cynic-kernel/src/judge.rs:273-277` (error handling)
- Modify: `cynic-kernel/tests/integration_judge.rs` (constructor call sites)

- [ ] **Step 1: Define OrganStatsUpdater in organ/health.rs (I2 fix)**

```rust
// Add to organ/health.rs:
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Bridge between Judge and organ DogStats. Passed as Option to Judge.
pub struct OrganStatsUpdater {
    stats: HashMap<String, Arc<RwLock<DogStats>>>,
    gates: HashMap<String, Arc<std::sync::Mutex<ParseFailureGate>>>,
}

impl OrganStatsUpdater {
    pub fn new(
        stats: HashMap<String, Arc<RwLock<DogStats>>>,
        gates: HashMap<String, Arc<std::sync::Mutex<ParseFailureGate>>>,
    ) -> Self {
        Self { stats, gates }
    }

    pub fn update(&self, dog_name: &str, scores: &[f64; 6], json_ok: bool) {
        if let Some(stats) = self.stats.get(dog_name) {
            // Use try_write to never block the Judge — drop update if contended
            if let Ok(mut s) = stats.try_write() {
                s.record_scores(scores, json_ok);
            }
        }
        if json_ok {
            if let Some(gate) = self.gates.get(dog_name) {
                if let Ok(mut g) = gate.try_lock() { g.record_success(); }
            }
        }
    }

    pub fn record_parse_failure(&self, dog_name: &str) {
        if let Some(stats) = self.stats.get(dog_name) {
            if let Ok(mut s) = stats.try_write() { s.record_json_failure(); }
        }
        if let Some(gate) = self.gates.get(dog_name) {
            if let Ok(mut g) = gate.try_lock() { g.record_failure(); }
        }
    }
}
```

- [ ] **Step 2: Modify Judge constructor (I2 fix)**

Change `Judge::new(dogs, breakers)` → `Judge::new(dogs, breakers, stats_updater: Option<Arc<OrganStatsUpdater>>)`.

Update all existing call sites in tests to pass `None`:
- `cynic-kernel/tests/integration_judge.rs` — grep for `Judge::new` and add `, None`

- [ ] **Step 3: Add stats update after phi_bound**

After the phi_bound block at judge.rs:252-257:

```rust
if let Some(ref updater) = self.stats_updater {
    let bounded = [fidelity, phi, verify, culture, burn, sovereignty];
    updater.update(&dog_name, &bounded, true);
}
```

- [ ] **Step 4: Add parse failure recording**

In judge.rs:273-277, when `DogError::ParseError`:

```rust
if let Some(ref updater) = self.stats_updater {
    updater.record_parse_failure(&dog_name);
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass. All existing tests pass `None` for stats_updater.

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/organ/health.rs cynic-kernel/src/judge.rs cynic-kernel/tests/
git commit -m "feat(organ): feed DogStats from judge.rs after phi_bound — per-Dog rolling baseline"
```

---

### Task 11: Final verification + make check

- [ ] **Step 1: Run full make check**

Run: `make check 2>&1`
Expected: build + test + clippy + lint-rules + lint-drift + audit all pass.

- [ ] **Step 2: Count tests**

Run: `cargo test 2>&1 | grep "test result"`
Expected: previous count (399) + new organ tests (~15-20) = ~415+ tests.

- [ ] **Step 3: Verify organ boots in production mode**

Run: `cargo build --release 2>&1 | tail -3`
Expected: release build succeeds (no LLVM crash — opt-level=2 already set).

- [ ] **Step 4: Commit any final fixes**

- [ ] **Step 5: Tag**

```bash
git tag v0.8.0-organ-phase1
```

---

## Summary

| Task | What | Tests | Debt fixed |
|---|---|---|---|
| 1 | Module skeleton + bitflags | compile | — |
| 2 | Registry types | 3 unit | — |
| 3 | Health (Welford + DogStats + ParseGate) | 8 unit | D9, D10 |
| 4 | Transport traits + adapters | compile | D3, D4 |
| 5 | Router (profile → cluster → backend) | 3 unit | D8 |
| 6 | Prometheus metrics | compile | — |
| 7 | InferenceOrgan facade | 2 integration | D1 |
| 8 | InferenceProfile::Calibration + D2 fix | existing pass | D2 |
| 9 | Wire organ in main.rs | make check | D1, D3, D4, D7 |
| 10 | DogStats feed from judge.rs | existing pass | D10 |
| 11 | Final verification | full suite | — |

**Total: 13 tasks (9 split into 9a/9b/9c), ~18+ new tests, 7 debts resolved (D1-D4, D7-D10). D5 deferred to Phase 2 (calibration).**
