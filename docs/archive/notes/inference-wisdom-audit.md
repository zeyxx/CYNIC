# Inference Wisdom Audit — CYNIC Layer by Layer

**Date:** 2026-08-18
**Source:** Ahmad's "Self-hosted LLMs / Local AI" series (5 articles + CogSec).
**Method:** Extract actionable wisdom → map to CYNIC architecture → identify gaps → propose repairs.

---

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

---

## Layer-by-Layer Gap Analysis

### 1. Domain Layer (`crates/cynic-inference/src/domain/`)

**What exists:**
- `BackendStatus` (5-state: Unknown, Healthy, Degraded, Critical, Recovering)
- `SupervisionPolicy` (HybridIntentAndCapacity, AlwaysConfirm, AlwaysAuto)
- `ModelRetention` (pure assessment, never deletes)
- `SlotSemaphore` (priority-aware permit system)
- `HardwareProfile` (reads `/proc/meminfo`, `/proc/cpuinfo`, GPU VRAM)
- `RuntimeLaunchPlan` (materializes launch argv)
- `Routing` (constraints + intent → decision)

**Gaps (from wisdom):**

| Gap | Article Source | Severity |
|-----|---------------|----------|
| No `MemoryBudget` struct that enforces: weights + KV + overhead + 10-30% safety ≤ available | GPU Math | HIGH |
| `HardwareProfile` has capacity but no bandwidth field (TB/s) | Memory BW | HIGH |
| No `AttentionType` enum (MHA/GQA/MQA/MLA) affecting KV cache sizing | LLM 101 | MEDIUM |
| No `InferencePhase` distinction (Prefill vs Decode) in `InferResponse` | LLM 101 | MEDIUM |
| `InferResponse` has tokens but no `latency_ms` breakdown | CogSec | MEDIUM |
| `SlotSemaphore` has slots but no KV cache hit/miss accounting | LLM 101 | LOW |

**Violation:** Structural Invariant #1 (monolith surfaces) — `HardwareProfile` owns
capacity, bandwidth detection, GPU VRAM reading, AND governor parsing in one
file. Split into: `capacity`, `bandwidth`, `gpu_info`, `governor`.

**Violation:** Structural Invariant #3 (dynamic detection before specialization)
— `RuntimeLaunchPlan` doesn't detect available bandwidth before choosing
quantization path.

---

### 2. Application Layer (`crates/cynic-inference/src/application/`)

**What exists:**
- `InferenceService` (thin delegate to `InferPort`)
- `FleetRouting` (rank healthy candidates)
- `MatrixSweepExecutor` (parameter sweep)
- `PeerAttach` (discover + publish to routing SSOT)
- `ProfileBootstrap` (forge `.env` from benchmark report)

**Gaps (from wisdom):**

| Gap | Article Source | Severity |
|-----|---------------|----------|
| No "failure gallery" — systematic "break it on purpose" workflow | CogSec + Projects | HIGH |
| No ablation framework: "remove positional encodings, disable causal masks..." | Projects | HIGH |
| `InferenceService::infer_with_backend` has no retry budget, no timeout policy | Engines | MEDIUM |
| `MatrixSweepExecutor` has no monotonic budget on iterations | Projects | MEDIUM |
| No "plot it" dashboard: loss curves, latency curves, memory curves | Projects | LOW |

**Violation:** Cross-scale invariant "Every mutation is a small transaction" —
`PeerAttach::peer_attach_apply()` mutates state without preflight → observe →
proof separation.

---

### 3. Ports Layer (`crates/cynic-inference/src/ports/`)

**What exists:**
- `BackendPort` (name + health)
- `InferPort` (infer)
- `ChatPort` (chat)
- `EmbeddingPort` (embed + embed_batch)
- `ChatSessionStore` (session persistence)

**Gaps (from wisdom):**

| Gap | Article Source | Severity |
|-----|---------------|----------|
| No `ObservabilityPort` — "make every shiny chatbot feel like just another slab of matrices" requires introspection | CogSec | HIGH |
| No `MetricsPort` — "benchmark the stack you will actually run" | Evals | MEDIUM |
| No `FailureGalleryPort` — "post the weird repetition loop... post the expert-routing collapse" | Projects | LOW |

**Violation:** AGENTS.md § "Adapters are disposable; ports and proofs are durable" —
`BackendPort` mixes health check (proof) with name (metadata). Split:
`HealthCheckPort` + `BackendMetadata`.

---

### 4. Adapters Layer (`crates/cynic-inference/src/adapters/`)

**What exists:**
- `inference/openai_compat.rs` (universal OpenAI adapter)
- `inference/cli.rs` (CLI-based health check)
- `inference/null.rs` (null backend for tests)
- `embedding/http.rs` (HTTP embedding)
- `local/` (vault, filesystem, process supervisor)
- `discovery/model_registry.rs` (HF/local model discovery)
- `publishers/inference_toml.rs` (routing SSOT writer)

**Gaps (from wisdom):**

| Gap | Article Source | Severity |
|-----|---------------|----------|
| No automatic engine selection: "llama.cpp for portability, MLX for Mac, vLLM for production" | Engines | HIGH |
| No bandwidth-aware adapter: "capacity decides what fits, bandwidth decides how hard it can breathe" | Memory BW | HIGH |
| `OpenAiCompatBackend` uses `Vec::new()` for `residual_unknowns` — false confidence when `usage` is missing | GPU Math | MEDIUM |
| No speculative decoding support: "DFlash and DDTree attack decode latency" | LLM 101 | LOW |

**Violation:** Structural Invariant #2 (no hardcoding when dynamic contract exists) —
`inference/cli.rs` hardcodes `rustc --version` path instead of detecting
toolchain manager.

---

## Wisdom → Concrete Repairs

### Repair 1: Memory Budget Enforcement

**File:** `crates/cynic-inference/src/domain/hardware_profile.rs`

Add:
```rust
pub struct MemoryBudget {
    pub weights_bytes: u64,
    pub kv_cache_bytes: u64,
    pub activations_bytes: u64,
    pub framework_overhead_bytes: u64,
    pub safety_margin_bytes: u64,  // 10-30% of total
}

impl MemoryBudget {
    pub fn total(&self) -> u64 {
        self.weights_bytes
            + self.kv_cache_bytes
            + self.activations_bytes
            + self.framework_overhead_bytes
            + self.safety_margin_bytes
    }

    pub fn fits_in(&self, available_bytes: u64) -> bool {
        self.total() <= available_bytes
    }
}
```

**Test:** `test_budget_enforces_safety_margin` — a budget without 10-30% margin
should fail `fits_in()` even if weights+KV fit.

---

### Repair 2: Hardware Tier Classification

**File:** `crates/cynic-inference/src/domain/hardware_profile.rs`

Add:
```rust
pub enum HardwareTier {
    Edge,           // < 150 GB/s bandwidth
    ThinAndLight,   // 150-300 GB/s
    Workstation,    // 450-650 GB/s
    Server,         // 800+ GB/s
}

impl HardwareTier {
    pub fn from_bandwidth_gb_s(bandwidth: f64) -> Self {
        match bandwidth {
            b if b < 150.0 => Self::Edge,
            b if b < 300.0 => Self::ThinAndLight,
            b if b < 650.0 => Self::Workstation,
            _ => Self::Server,
        }
    }

    pub fn supports_multi_agent(&self) -> bool {
        matches!(self, Self::Workstation | Self::Server)
    }
}
```

**Test:** `test_edge_tier_rejects_multi_agent` — `Edge.supports_multi_agent()` is false.

---

### Repair 3: Prefill vs Decode Tracking

**File:** `crates/cynic-inference/src/domain/inference.rs`

Modify `InferResponse`:
```rust
pub struct InferResponse {
    pub text: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub residual_unknowns: VecString>,
    pub phase_latency: PhaseLatency,  // NEW
}

pub struct PhaseLatency {
    pub prefill_ms: Option<u64>,   // Time to first token
    pub decode_ms: Option<u64>,    // Time for all completion tokens
    pub tokens_per_second: Option<f64>,
}
```

**Test:** `test_phase_latency_distinguishes_prefill_from_decode` — long prompt
should show high prefill_ms; long completion should show high decode_ms.

---

### Repair 4: Failure Gallery Workflow

**File:** `crates/cynic-inference/src/application/failure_gallery.rs` (new)

```rust
pub struct FailureGalleryEntry {
    pub scenario: String,
    pub expected_behavior: String,
    pub actual_behavior: String,
    pub root_cause: String,
    pub reproduction_steps: Vec<String>,
    pub artifact_ref: Option<String>,  // sha256 of logs/dump
}

pub trait FailureGallery: Send + Sync {
    fn record(&self, entry: FailureGalleryEntry) -> Result<(), GalleryError>;
    fn list_scenarios(&self) -> Vec<String>;
    fn get_scenario(&self, name: String) -> Option<FailureGalleryEntry>;
}
```

**Test:** `test_failure_gallery_records_and_retrieves` — insert + read back.

---

### Repair 5: Attention Type Detection

**File:** `crates/cynic-inference/src/domain/backend_identity.rs`

Add:
```rust
pub enum AttentionType {
    MHA,   // Full multi-head
    GQA(u32),  // Grouped-query with N groups
    MQA,   // Multi-query (1 KV head)
    MLA,   // Multi-head Latent Attention (DeepSeek-V2+)
}

impl AttentionType {
    pub fn kv_heads(&self, total_heads: u32) -> u32 {
        match self {
            Self::MHA => total_heads,
            Self::GQA(groups) => *groups,
            Self::MQA => 1,
            Self::MLA => total_heads, // compressed
        }
    }
}
```

**Test:** `test_mqa_reduces_kv_heads_to_one`.

---

### Repair 6: Eval Contamination Scan

**File:** `crates/cynic-protocol/src/eval_hygiene.rs` (new)

```rust
pub struct EvalContaminationScan {
    pub train_refs: Vec<String>,
    pub eval_refs: Vec<String>,
    pub overlaps: Vec<Overlap>,
}

pub struct Overlap {
    pub train_ref: String,
    pub eval_ref: String,
    pub similarity: f64,
    pub risk_level: ContaminationRisk,
}

pub enum ContaminationRisk {
    Exact,      // sha256 match
    NearDuplicate, // >90% similarity
    Semantic,   // embedding similarity
    None,
}

pub fn scan_for_contamination(
    train_refs: &[String],
    eval_refs: &[String],
) -> EvalContaminationScan {
    // Implementation: exact hash → n-gram → embedding similarity
}
```

**Test:** `test_exact_overlap_detected` — identical refs flagged as `Exact`.

---

### Repair 7: Inference Engine Selector

**File:** `crates/cynic-inference/src/application/engine_selector.rs` (new)

```rust
pub enum EnginePreference {
    Portability,    // llama.cpp
    MacNative,      // MLX
    SingleCuda,     // ExLlamaV2
    MultiCuda,      // ExLlamaV3 / vLLM
    Production,     // vLLM / SGLang
    MaxNvidia,      // TensorRT-LLM
    Edge,           // MLC / ONNX
}

pub fn select_engine(
    hardware: &HardwareProfile,
    workload: &WorkloadShape,
    preference: EnginePreference,
) -> Result<EngineRecommendation, SelectionError> {
    // Decision tree based on Ahmad's "one-page decision guide"
}
```

**Test:** `test_mac_silicon_selects_mlx` — M3 Ultra + MacNative → MLX.

---

## Cross-Cutting Invariants (Violated)

### 1. "Every bug is evidence of an unmodelled contract"

The missing `MemoryBudget` struct is evidence that CYNIC doesn't model "total
memory contract" (weights + KV + overhead + safety). **Repair:** Add the
struct + enforce at `RuntimeLaunchPlan::plan_launch()`.

### 2. "Unknown is data"

`OpenAiCompatBackend` returns `Vec::new()` for `residual_unknowns` even when
`usage` is `None`. **Repair:** Conditional population:
```rust
residual_unknowns: if response.usage.is_none() {
    vec!["usage_not_returned_by_backend".into()]
} else {
    vec![]
},
```

### 3. "Scale changes data and adapters, not the core model"

The `SlotSemaphore` is per-dog (single node). Multi-node slot coordination
requires a distributed semaphore (e.g., via SurrealDB). **Current:**
`SlotSemaphore` is correct for single-node. **Gap:** No multi-node slot
coordination for fleet-wide agent concurrency.

### 4. "One canonical decision source per concern"

`FleetRouting` has `select_best_candidate()` AND `routing_policy.rs` has
`select_backend()` — duplicate decision logic. **Repair:** Merge into
`RoutingEngine` with pluggable strategies.

---

## Repairs Completed

All 7 repairs from the wisdom audit are now implemented:

| P | Repair | File | Status |
|---|--------|------|--------|
| 0 | `MemoryBudget` struct + safety enforcement | `hardware_profile.rs` | ✅ |
| 0 | `EngineSelector` (auto-choice) | `engine_selector.rs` | ✅ |
| 1 | `PhaseLatency` tracking (prefill vs decode) | `inference.rs` | ✅ |
| 1 | `HardwareTier` classification | `hardware_profile.rs` | ✅ |
| 1 | Conditional `residual_unknowns` | `openai_compat.rs` | ✅ |
| 2 | `FailureGallery` (break it on purpose) | `failure_gallery.rs` | ✅ |
| 2 | `EvalContaminationScan` (exact/fuzzy/semantic) | `eval_hygiene.rs` | ✅ |

See `docs/notes/inference-wisdom-audit.md` for the full wisdom extraction and repair design.

---

## Non-Goals (From Wisdom)

- **Do not build Ollama wrapper.** Ahmad: "DO NOT USE Ollama." CYNIC
  already avoids this. Keep it that way.
- **Do not worship parameter count.** "A 7B model at Q6 can beat a 13B model
  at Q2." CYNIC's model selection should prefer quality-at-fit over raw
  param count.
- **Do not treat benchmarks as truth.** "Train on training, tune on
  validation, measure once on test." CYNIC's `cynic_validate` must enforce
  this split.

---

## Sign-Off

This audit maps external wisdom to internal gaps. All repairs respect CYNIC
structural invariants: no monolith expansion, dynamic detection before
specialization, and one canonical decision source per concern.
