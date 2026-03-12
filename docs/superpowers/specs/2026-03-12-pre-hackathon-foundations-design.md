# Pre-Hackathon Foundations — Design Spec

**Date:** 2026-03-12
**Status:** Approved design, pending implementation plan
**Scope:** Foundational refactor + new capabilities before Gemini 3 Paris Hackathon (2026-03-14)
**Constraint:** Every component must respect hexagonal architecture (CYNIC-ARCHITECTURE-TRUTHS.md) and crystallized cognitive architecture (CYNIC-CRYSTALLIZED-TRUTH.md). Zero technical debt.

---

## Crystallized Truths (informing this design)

| T# | Truth | Confidence | Design impact |
|----|-------|------------|---------------|
| T1 | OpenAI `/v1/chat/completions` is the universal de facto standard for LLM inference in 2026 | 58% | Standardize wire format on OpenAI-compat. No custom format. |
| T1a | Gemini has official OpenAI-compat endpoint (`/v1beta/openai/`). HuggingFace too (`router.huggingface.co/v1`). Benchmark: ~80ms overhead (7.5%), negligible. | 56% | GeminiBackend = OpenAiCompatBackend with Gemini base_url |
| T2 | A single `OpenAiCompatBackend` parameterized by (url, key, model, auth_style) replaces ALL specific backends | 52% | One Rust type, N instances. Zero duplication. |
| T3 | `InferencePort` (MCTS/compute) and `ChatPort` (prompt→text) are separate domain concerns — Dogs use ChatPort, MCTS uses InferencePort | 55% | New `ChatPort` trait. `OpenAiCompatBackend` implements both. `InferenceDog` depends on `ChatPort` only. |
| T4 | Sovereignty is in swap capability, not in wire protocol | 54% | Universal format IS sovereignty. |
| T5 | SGLang replaces TGI as HuggingFace-recommended inference server. Landscape: llama.cpp (edge), vLLM/SGLang (prod), Ollama (dev) | 50% | Don't invest in TGI adapter. |

---

## Architecture

```
                        DOMAIN CORE
                        =====================

  Dog trait                          ChatPort trait              InferencePort trait
    |-- DeterministicDog               |-- OpenAiCompatBackend     |-- OpenAiCompatBackend
    |-- InferenceDog(ChatPort)         |-- MockChatBackend         |-- MockBackend

  Judge
    |-- parallel eval (join_all)
    |-- geometric mean (phi-bounded)
    |-- residual detection (disagreement > phi^-2)

  BackendRouter
    |-- circuit breaker per backend
    |-- health probes

                        COMPOSITION ROOT (main.rs)
                        =====================

  1. Load backends.toml -> Vec<BackendConfig>
  2. For each config: OpenAiCompatBackend::connect(config)
  3. Health check -> register in BackendRouter
  4. Create InferenceDog per healthy backend
  5. Add DeterministicDog (always)
  6. Build Judge with all Dogs
  7. Start REST API + static frontend on :3030
  8. Report boot health (HEALTHY/DEGRADED/CRITICAL)

                        DRIVEN ADAPTERS (out)
                        =====================

  OpenAiCompatBackend instances:
    gemini  -> generativelanguage.googleapis.com/v1beta/openai/
    hf      -> router.huggingface.co/v1
    local   -> localhost:8080/v1 (llama-server)
    remote  -> <tailscale-ip>:8080/v1 (future)
```

---

## Components

### A1. OpenAiCompatBackend

**File:** `cynic-kernel/src/backend_openai.rs` (replaces `backend_llamacpp.rs`)

**Struct:**
```rust
pub struct OpenAiCompatBackend {
    client: reqwest::Client,
    config: BackendConfig,
    health_state: AtomicU8, // circuit breaker state
}

pub struct BackendConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub auth_style: AuthStyle,
}

pub enum AuthStyle {
    Bearer,
    QueryParam(String),
    None,
}
```

**Implements:** `ChatPort` trait (new, for Dogs) AND `InferencePort` trait (existing, for MCTS).

`ChatPort` (domain core, `chat_port.rs`):
```rust
#[async_trait]
pub trait ChatPort: Send + Sync {
    async fn chat(&self, system: &str, user: &str) -> Result<String, ChatError>;
    async fn health(&self) -> BackendStatus;
    fn name(&self) -> &str;
}
```

- `chat()`: POST `{base_url}/chat/completions`, extract `choices[0].message.content`
- `health()`: GET `{base_url}/models` or HEAD request
- `infer()` (InferencePort): same HTTP call but maps to `InferenceRequest`/`InferenceResponse` with MCTS fields

**Separation of concerns:** `ChatPort` is the minimal contract for text-in/text-out (what Dogs need). `InferencePort` carries MCTS semantics (num_branches, trace_id, hypotheses). Same backend, two interfaces. Interface Segregation Principle.

**`BackendConfig` and `AuthStyle` live in `config.rs` (infrastructure layer), NEVER in domain core.**

**Auth handling:**
- `Bearer`: `Authorization: Bearer {api_key}` header
- `QueryParam(k)`: append `?{k}={api_key}` to URL
- `None`: no auth (local servers)

**Replaces:** `LlamaCppBackend` (same logic, more flexible config). `GeminiDog` direct API calls (now goes through InferencePort).

### A2. InferenceDog

**File:** `cynic-kernel/src/inference_dog.rs` (replaces `gemini_dog.rs`)

**Struct:**
```rust
pub struct InferenceDog {
    chat: Arc<dyn ChatPort>,
    name: String,
}
```

**Implements:** `Dog` trait.
- `id()`: returns `&self.name`
- `evaluate()`: builds axiom prompt -> calls `chat.chat(system_prompt, user_prompt)` -> parses JSON scores from response text
- Prompt template: migrated from `GeminiDog::build_prompt()` (unchanged logic)
- JSON extraction: `extract_json()` function migrated from `gemini_dog.rs`

**Key design decision:** The axiom evaluation prompt lives in this file. ONE prompt for all backends. If a model produces bad JSON, `extract_json()` handles markdown fences. If it fails entirely, `DogError::ParseError` is returned and the Judge excludes that Dog's scores.

### B. Backend Config & Discovery

**File:** `cynic-kernel/src/config.rs` (new)

**Config format:** TOML at `~/.config/cynic/backends.toml`
```toml
[backend.gemini]
base_url = "https://generativelanguage.googleapis.com/v1beta/openai"
api_key_env = "GEMINI_API_KEY"
model = "gemini-2.5-flash"
auth_style = "bearer"

[backend.hf]
base_url = "https://router.huggingface.co/v1"
api_key_env = "HF_TOKEN"
model = "mistralai/Mistral-7B-Instruct-v0.3"
auth_style = "bearer"

[backend.local]
base_url = "http://localhost:8080/v1"
model = "phi-3-mini"
auth_style = "none"
```

**`api_key_env`** references environment variable names, never raw secrets. The TOML file can be committed. Secrets stay in systemd EnvironmentFile.

**Boot sequence change in `main.rs`:**
1. Load `backends.toml` via `config::load_backends()`
2. Resolve `api_key_env` -> `std::env::var()`
3. For each config with resolved key: attempt `OpenAiCompatBackend::connect()`
4. Health probe each backend
5. Register healthy backends in `BackendRouter`
6. Create `InferenceDog` per healthy backend
7. Always add `DeterministicDog`
8. Build `Judge` with all Dogs
9. Report: "N Dogs active: [list]"

**Fallback:** If `backends.toml` doesn't exist, fall back to env-var-based config (backward compat with current GEMINI_API_KEY approach).

### C. llama.cpp on forge

**Install:** Compile from source with AVX2 support (confirmed available on forge).
**Model:** Phi-3-mini-4k-instruct Q4_K_M (~2.3GB). Fits in 10Gi available RAM.
**Service:** systemd user unit `llama-server.service`.
**Port:** 8080 (no conflict with existing services).
**Config:** Entry in `backends.toml` as `[backend.local]`.
**Performance:** Expected ~3-8 tok/s on 3x i5-6500T cores. Slow but sovereign. Prompt is short (~200 tokens), response is short (~200 tokens). Total ~30-60s per inference.

### D1. Judge Parallel Evaluation

**File:** `cynic-kernel/src/judge.rs`

**Change:** Replace sequential `for dog in dogs` loop with:
```rust
let futures = self.dogs.iter().map(|dog| dog.evaluate(stimulus));
let results = futures::future::join_all(futures).await;
```

**Impact:** Latency drops from sum(dog_latencies) to max(dog_latencies).
With 3 Dogs (deterministic ~0ms, gemini ~1s, local ~30s): 30s instead of 31s.

### D2. Residual Detection

**File:** `cynic-kernel/src/judge.rs`

**Logic:** Post-aggregation, compute max inter-Dog disagreement per axiom:
```rust
// Compute per-Dog Q-Scores, compare in verdict space (not raw axiom space)
let dog_qscores: Vec<f64> = dog_scores.iter()
    .map(|s| compute_qscore(&phi_bound_scores(s)).total)
    .collect();
let consensus_q = verdict.q_score.total;
let max_disagreement = dog_qscores.iter()
    .map(|q| (q - consensus_q).abs())
    .fold(0.0_f64, f64::max);

let anomaly_detected = max_disagreement > PHI_INV2; // 0.382
```

**In Verdict:** Add fields `anomaly_detected: bool`, `max_disagreement: f64`, `anomaly_axiom: Option<String>`.

This implements the "Residual Detection" innovation from CYNIC-CRYSTALLIZED-TRUTH.md: "When Dogs DISAGREE beyond 38.2% threshold -> ANOMALY flagged. Treats disagreement as DISCOVERY SIGNAL, not noise."

### D3. Per-Dog Scores in Response

**Verdict struct change:**
```rust
pub struct DogScore {
    pub dog_id: String,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub reasoning: AxiomReasoning,
}

// Added to Verdict:
#[serde(default)]
pub dog_scores: Vec<DogScore>,  // backward compat: defaults to empty vec for old records
#[serde(default)]
pub anomaly_detected: bool,
#[serde(default)]
pub max_disagreement: f64,
```

**REST response:** `/judge` now includes full per-Dog breakdown. Frontend uses this for timeline, radar, and hypercube.

### E. Contract Tests

**Dog contract test** (`tests/dog_contract.rs`):
```rust
async fn dog_contract(dog: &dyn Dog) {
    assert!(!dog.id().is_empty(), "Dog must have non-empty id");

    let stimulus = Stimulus {
        content: "The Earth orbits the Sun.".into(),
        context: None,
        domain: Some("science".into()),
    };

    let result = dog.evaluate(&stimulus).await;
    match result {
        Ok(scores) => {
            assert!(scores.fidelity >= 0.0 && scores.fidelity <= 1.0);
            assert!(scores.phi >= 0.0 && scores.phi <= 1.0);
            assert!(scores.verify >= 0.0 && scores.verify <= 1.0);
        }
        Err(e) => {
            // Errors must be well-formed DogError variants
            match e {
                DogError::ApiError(_) | DogError::ParseError(_)
                | DogError::RateLimited(_) => {}
                // NOTE: DogError::Timeout must be added to dog.rs before this compiles
            }
        }
    }
}
```

**InferencePort contract test:** same pattern, verifying capability(), health(), infer().

Both `MockBackend` and `OpenAiCompatBackend` must pass the same suite.

### F. Frontend

**Location:** `cynic-kernel/static/`

**Files:**
- `index.html` — layout, form, dark theme with phi-golden proportions
- `verdict.js` — fetch `/judge`, render verdict card with Q-score and reasoning
- `radar.js` — Canvas 2D radar chart. 3 axes (FIDELITY, PHI, VERIFY). One polygon per Dog + consensus overlay. Colors: each Dog gets a distinct color.
- `hypercube.js` — Three.js (CDN). 3D cube bounded by [0, phi^-1]^3. Each Dog = colored sphere. Consensus = golden sphere. Anomaly = red highlight when distance > phi^-2. Rotatable.

**Serving:** Axum `tower_http::services::ServeDir` on `/` route. Single port 3030.

**No build step.** No npm. No framework. Vanilla HTML/JS/CSS. Agnostic.

### G. Infrastructure

**G1. Cloudflare Tunnel:**
- Install `cloudflared` on forge
- Run: `cloudflared tunnel --url http://localhost:3030`
- Provides free public URL (*.trycloudflare.com)
- systemd service for persistence

**G2. Repo cleanup:**
- README with architecture diagram, usage examples, demo screenshots
- Ensure CI pipeline validates new components

---

## Dependency Graph

```
PARALLEL (no dependencies):
  C  (llama.cpp install on forge)
  E  (contract tests - TDD before refactor)
  F1 (basic frontend - works with current API)
  G  (tunnel + repo)

SEQUENTIAL (dependency chain):
  A0 (ChatPort trait + DogError::Timeout in domain core)
    -> A1 (OpenAiCompatBackend implements ChatPort + InferencePort)
      -> A2 (InferenceDog uses ChatPort)
        -> B (config.rs + backends.toml + boot discovery)
          -> D1 (judge parallel)
          -> D2 (residual detection in Q-Score space)
          -> D3 (per-dog scores with serde(default))
            -> F2+F3+F4 (advanced frontend)

PARALLEL (no dependencies, formerly deferred):
  P0 (probe.rs hexagonal refactor - inject GpuDetector trait)
  S0 (StoragePort trait - replace concrete CynicStorage import in rest.rs)
  H0 (5-state vs 3-state health reconciliation in CLAUDE.md)
```

### Spec Review Fixes Applied

| Issue | Severity | Fix |
|-------|----------|-----|
| #1 InferenceDog↔InferencePort mismatch | CRITICAL | New `ChatPort` trait. InferenceDog uses ChatPort, not InferencePort. |
| #2 DogScore backward compat | CRITICAL | `#[serde(default)]` on all new Verdict fields. |
| #3 BackendConfig in domain core | CRITICAL | BackendConfig/AuthStyle in `config.rs` (infra), never in `backend.rs` (domain). |
| #4 Residual in wrong space | IMPORTANT | Compare Dog Q-Scores (verdict space), not raw axiom means. |
| #5 DogError::Timeout missing | IMPORTANT | Explicit note: add variant to `dog.rs` before contract tests. |
| #6 Probe refactor missing | IMPORTANT | Included as Task 12b (P0). GpuDetector trait injection, probe.rs split into modules. |
| #7 5-state vs 3-state health | IMPORTANT | Included as Task 12 (H0). Update CLAUDE.md to document 5-state lifecycle. |

---

## Future-Proof Validation

Every component from CYNIC-CRYSTALLIZED-TRUTH.md that comes AFTER this work:

| Future Feature | How current design supports it |
|---|---|
| 6 axioms | Extend AxiomScores, update prompt in InferenceDog. One file change. |
| 7-step cycle | Traits compose on top of Judge. No replacement needed. |
| CCM (Crystallization) | Reads Verdicts from StoragePort. Additive module. |
| Q-Learning | Per-Dog scores in Verdict = training signal. Additive module. |
| MCTS Temporal | InferenceDog with temporal prompts. Same trait, same backend. |
| 11 Dogs | N InferenceDogs + specialized Dogs. All implement Dog trait. |
| BFT consensus | Threshold in Judge post-aggregation. Additive logic. |
| EventBus | Inserts between components. Nothing coupled that blocks it. |
| Consciousness levels | LODController reads probe, adjusts Dog count. Additive. |
| Triple learning | Consumes Verdicts. Three independent modules. |

**Zero debt. Every future feature ADDS to these foundations.**

---

## Non-Goals (explicitly excluded)

- No Ollama support (user considers it unnecessary dependency)
- No TGI support (deprecated, replaced by SGLang)
- No custom wire format (OpenAI-compat is universal)
- No npm/Node tooling (vanilla frontend)
- No GitHub Actions/Pages (user constraint)
- No Docker (baremetal is optimal for current topology)

---

*Designed through: brainstorming (intent + constraints) -> crystallize-truth (inference API landscape) -> empirical validation (Gemini benchmark) -> cynic-wisdom (philosophical grounding). Maximum confidence: 61.8%.*
