# Inference Wisdom Audit — Ahmad's 5-Article Mapping

This reference maps Ahmad's "Self-hosted LLMs / Local AI" series (5 articles + CogSec)
to CYNIC inference architecture. Use when the user asks about inference stack design,
model selection, or hardware-aware deployment.

## Wisdom Extraction Matrix

| Article | Core Principle | CYNIC Equivalent | Gap |
|---------|---------------|------------------|-----|
| CogSec | Run models locally to build intuition | `cynic-inference` local-first adapters | No "failure gallery" workflow; no systematic "break it" culture |
| CogSec | Friction is a feature (extra step = moment to think) | `--yes` gate on all mutations | Missing: operator-facing "system prompt" visibility |
| GPU Math | `VRAM ≈ B × (bits ÷ 8)` then add KV cache + overhead + 10-30% safety | `HardwareProfile` + `RuntimeLaunchPlan` | No automatic total-memory estimate; no headroom enforcement |
| GPU Math | GGUF is runtime-specific truth | GGUF via llama.cpp adapter | Missing: framework-agnostic memory calculator |
| Memory BW | `Capacity × Bandwidth × Software Stack` | `HardwareProfile` reads `/proc/meminfo` | No bandwidth detection; no hardware tier classification |
| Memory BW | "Fit is not speed" | `RuntimeLaunchPlan` assumes fit=speed | Missing: bandwidth-bottleneck detection |
| Engines | Engine follows hardware + workload + serving model | Adapter pattern (llama.cpp, vLLM, etc.) | No automatic engine selection; no "scheduler" abstraction |
| Engines | "DO NOT USE Ollama" | CYNIC never used Ollama | ✓ Clean |
| LLM 101 | "Start with the loop: tokens in, probabilities out" | `InferRequest → InferResponse` | Missing: token-flow observability; no streaming dashboard |
| LLM 101 | KV cache is the working memory bill | `SlotSemaphore` for slot management | Missing: KV cache hit/miss metrics; no paged attention |
| LLM 101 | Prefill is compute-bound; decode is bandwidth-bound | No phase distinction in `InferResponse` | Missing: prefill vs decode latency tracking |
| LLM 101 | GQA/MQA/MLA reduce KV memory | `BackendStatus` only | Missing: attention-type detection; no KV-head counting |
| Projects | "Build it, plot it, break it, explain it, ship it" | No systematic "break on purpose" workflow | Missing: ablation testing framework; no failure gallery |
| Projects | "Ablate the thing you just built" | Tests exist but no ablation harness | Missing: structured mutation testing |
| Evals | "Train on training, tune on validation, measure once on test" | `cynic_validate` as gate | Missing: eval exclusion list; no contamination scanning |
| Evals | "Freeze the protocol before testing" | `make check` gate | Missing: hyperparameter freeze enforcement |
| Evals | "Any information path from test to model = contamination" | No explicit contamination control | Missing: eval-trace separation from training data |
| Evals | "Report uncertainty, not just point numbers" | Metrics are point estimates | Missing: confidence intervals on benchmarks |

## Repairs (All Implemented)

### Repair 1: Memory Budget Enforcement

```rust
pub struct MemoryBudget {
    pub weights_bytes: u64,
    pub kv_cache_bytes: u64,
    pub activations_bytes: u64,
    pub framework_overhead_bytes: u64,
    pub safety_margin_bytes: u64,  // 10-30% of total
}

impl MemoryBudget {
    pub fn total(&self) -> u64 { /* sum of all fields */ }
    pub fn fits_in(&self, available_bytes: u64) -> bool { self.total() <= available_bytes }
    pub fn has_adequate_safety(&self) -> bool { /* safety_ratio >= 0.15 */ }
}
```

File: `crates/cynic-inference/src/domain/hardware_profile.rs`

### Repair 2: Hardware Tier Classification

```rust
pub enum HardwareTier {
    Edge,           // < 150 GB/s bandwidth
    ThinAndLight,   // 150-300 GB/s
    Workstation,    // 450-650 GB/s
    Server,         // 800+ GB/s
}

impl HardwareTier {
    pub fn from_bandwidth_gb_s(bandwidth: f64) -> Self { /* ... */ }
    pub fn supports_multi_agent(&self) -> bool { /* Workstation | Server */ }
    pub fn supports_moe(&self) -> bool { /* Server only */ }
}
```

File: `crates/cynic-inference/src/domain/hardware_profile.rs`

### Repair 3: Prefill vs Decode Tracking

```rust
pub struct PhaseLatency {
    pub prefill_ms: Option<u64>,   // Time to first token
    pub decode_ms: Option<u64>,    // Total decode time
    pub tokens_per_second: Option<f64>,
}

pub struct InferResponse {
    // ... existing fields ...
    pub phase_latency: PhaseLatency,
}
```

File: `crates/cynic-inference/src/domain/inference.rs`

### Repair 4: Failure Gallery Workflow

```rust
pub struct FailureGalleryEntry {
    pub scenario: String,
    pub expected_behavior: String,
    pub actual_behavior: String,
    pub root_cause: String,
    pub reproduction_steps: Vec<String>,
    pub artifact_ref: Option<String>,
    pub resolved: bool,
    pub fix_description: Option<String>,
}

pub fn classify_failure(symptom: &str) -> FailureCategory { /* ... */ }
pub fn suggested_fix(category: FailureCategory) -> &'static str { /* ... */ }
```

File: `crates/cynic-inference/src/application/failure_gallery.rs`

### Repair 5: Eval Contamination Scan

```rust
pub struct EvalContaminationScan {
    pub train_refs: Vec<String>,
    pub eval_refs: Vec<String>,
    pub overlaps: Vec<Overlap>,
}

pub fn scan_for_contamination(
    train_refs: &[String],
    eval_refs: &[String],
) -> EvalContaminationScan {
    // Layered: exact hash → n-gram → embedding similarity
}
```

File: `crates/cynic-protocol/src/eval_hygiene.rs`

### Repair 6: Inference Engine Selector

```rust
pub enum InferenceEngine {
    Portable,       // llama.cpp
    AppleNative,    // MLX
    ConsumerCuda,   // ExLlamaV2
    MultiCuda,      // ExLlamaV3 / vLLM
    Production,     // vLLM / SGLang
    NvidiaMax,      // TensorRT-LLM
    Cluster,        // NVIDIA Dynamo
}

pub fn select_engine(
    hardware_tier: HardwareTier,
    has_apple_silicon: bool,
    has_nvidia_gpu: bool,
    workload: &WorkloadShape,
) -> EngineRecommendation { /* decision tree */ }
```

File: `crates/cynic-inference/src/application/engine_selector.rs`

## Cross-Cutting Invariants (Violated)

### 1. "Every bug is evidence of an unmodelled contract"
The missing `MemoryBudget` struct is evidence that CYNIC doesn't model "total
memory contract" (weights + KV + overhead + safety). Repaired.

### 2. "Unknown is data"
`OpenAiCompatBackend` returns `Vec::new()` for `residual_unknowns` even when
`usage` is `None`. Repaired with conditional population.

### 3. "Scale changes data and adapters, not the core model"
The `SlotSemaphore` is per-dog (single node). Multi-node slot coordination
requires a distributed semaphore (e.g., via SurrealDB). Current: correct for
single-node. Gap: No multi-node slot coordination for fleet-wide agent concurrency.

### 4. "One canonical decision source per concern"
`FleetRouting` has `select_best_candidate()` AND `routing_policy.rs` has
`select_backend()` — duplicate decision logic. Repair: Merge into
`RoutingEngine` with pluggable strategies.

## Pitfall

The user wants data-centric design, NOT scripts. When asked for "scripts" for
claim management, correct: use MCP surfaces (`cynic_coord_*`) and CLI surfaces
(`cynic agent *`). Scripts reintroduce the adapter pattern we are deleting.

## Non-Goals (From Wisdom)

- **Do not build Ollama wrapper.** Ahmad: "DO NOT USE Ollama." CYNIC
  already avoids this. Keep it that way.
- **Do not worship parameter count.** "A 7B model at Q6 can beat a 13B model
  at Q2." CYNIC's model selection should prefer quality-at-fit over raw
  param count.
- **Do not treat benchmarks as truth.** "Train on training, tune on
  validation, measure once on test." CYNIC's `cynic_validate` must enforce
  this split.
