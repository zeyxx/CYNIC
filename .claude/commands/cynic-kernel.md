
# CYNIC Kernel — Canonical Architecture Reference

*Two source documents: `docs/CYNIC-CRYSTALLIZED-TRUTH.md` (cognitive layer) + `docs/CYNIC-ARCHITECTURE-TRUTHS.md` (infrastructure layer). This skill encodes both so every session starts with full context.*

---

## IDENTITY

CYNIC is an **epistemic immune system** — independent AI validators reaching consensus under mathematical doubt. φ-bounded confidence (max 61.8%). Sovereignty is non-negotiable.

**Triple Agnosticism (non-negotiable):**
- **Hardware-agnostic:** RPi, gaming PC, VPS, cluster. Probe detects capabilities.
- **Model-agnostic:** Swap any LLM. Dog interface is the contract. Open-source models are default.
- **Domain-agnostic:** Chess, trading, code review, geopolitics. Axioms are universal.

---

## TOPOLOGY

| Node | Role | Hardware | OS |
|---|---|---|---|
| **`<TAILSCALE_UBUNTU>`** | Kernel, REST API, MCP server | Ryzen 7 5700U, 16GB RAM | Linux |
| **`<TAILSCALE_FORGE>`** | GPU inference worker (sovereign dog) | i5-14400F, 16GB RAM, RTX 4060 Ti | Windows |

**Network:** Tailscale VPN between nodes — but NOT the only topology. Localhost is equally valid. A machine capable of its own inference does NOT proxy to another.

---

## HEXAGONAL ARCHITECTURE

```
        DRIVING ADAPTERS (in)               DOMAIN CORE                DRIVEN ADAPTERS (out)
        ====================               ============               ====================
        REST (axum) ─────────┐                                        OpenAiCompatBackend ── HTTP ── llama-server
        MCP (rmcp/stdio) ────┼────────▶ InferenceRequest/Response     SurrealHttpStorage ── HTTP ── SurrealDB
                              │         BackendCapability              probe ── /sys ── Linux kernel
                              │         BackendStatus (5-state)
                              │         QScore, Verdict, Dog (trait)
                              │         NodeConfig (value object)
                              │
        [gRPC: feature-gated, NOT live — --features grpc to enable]
```

**Dependency Rule:** Adapters depend on Ports (traits) which live in Domain Core. Domain Core depends on NOTHING external. `main.rs` (composition root) wires adapters to ports. `main.rs` is THE ONLY FILE with concrete types.

---

## PORT CONTRACTS (actual — verified against source)

### BackendPort (base trait — `domain/inference.rs`)
```rust
#[async_trait]
pub trait BackendPort: Send + Sync {
    fn name(&self) -> &str;
    async fn health(&self) -> BackendStatus;
}
```
`ChatPort` and `InferencePort` both extend this — `health()` and `name()` defined once.

### InferencePort (`domain/inference.rs`)
```rust
#[async_trait]
pub trait InferencePort: BackendPort {
    fn capability(&self) -> &BackendCapability;
    async fn infer(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError>;
}
```
**Adapters:** `OpenAiCompatBackend` (primary), `MockBackend` (tests)

### ChatPort (`domain/chat.rs`)
```rust
#[async_trait]
pub trait ChatPort: BackendPort {
    async fn chat(&self, system: &str, user: &str) -> Result<ChatResponse, ChatError>;
}
```
Dogs use this for axiom evaluation. `ChatResponse` carries `text`, `prompt_tokens`, `completion_tokens`.

### StoragePort (`domain/storage.rs`)
```rust
#[async_trait]
pub trait StoragePort: Send + Sync {
    async fn ping(&self) -> Result<(), StorageError>;
    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError>;
    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError>;
    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError>;
    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError>;
    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError>;
    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError>;
    async fn observe_crystal(&self, id: &str, content: &str, domain: &str, score: f64, timestamp: &str) -> Result<(), StorageError>;
    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError>;
    async fn query_observations(&self, project: &str, domain: Option<&str>, limit: u32) -> Result<Vec<serde_json::Value>, StorageError>;
    async fn query_session_targets(&self, project: &str, limit: u32) -> Result<Vec<serde_json::Value>, StorageError>;
}
```
**Adapters:** `SurrealHttpStorage` (HTTP to SurrealDB 3.x), `NullStorage` (graceful degradation — always Ok, never persists).

### CoordPort (`domain/coord.rs`)
```rust
#[async_trait]
pub trait CoordPort: Send + Sync {
    async fn register_agent(&self, agent_id: &str, agent_type: &str, intent: &str) -> Result<(), CoordError>;
    async fn claim(&self, agent_id: &str, target: &str, claim_type: &str) -> Result<ClaimResult, CoordError>;
    async fn release(&self, agent_id: &str, target: Option<&str>) -> Result<String, CoordError>;
    async fn who(&self, agent_id_filter: Option<&str>) -> Result<CoordSnapshot, CoordError>;
    async fn store_audit(&self, tool: &str, agent_id: &str, details: &serde_json::Value) -> Result<(), CoordError>;
    async fn query_audit(&self, tool_filter: Option<&str>, agent_filter: Option<&str>, limit: u32) -> Result<Vec<serde_json::Value>, CoordError>;
    async fn heartbeat(&self, agent_id: &str) -> Result<(), CoordError>;
    async fn deactivate_agent(&self, agent_id: &str) -> Result<(), CoordError>;
    async fn expire_stale(&self) -> Result<(), CoordError>;
}
```
**Adapters:** `SurrealHttpStorage` (same struct implements both StoragePort and CoordPort), `NullCoord`.

### Dog trait (`domain/dog.rs`)
```rust
#[async_trait]
pub trait Dog: Send + Sync {
    fn id(&self) -> &str;
    fn max_context(&self) -> u32 { 0 }   // 0 = unlimited
    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError>;
}
```
Dogs return **raw** `AxiomScores` (not phi-bounded). The kernel phi-bounds and aggregates. Each Dog wraps any LLM (open-source, API, deterministic code).

### NOT ports (do not exist — never add them):
- `GpuDetector` trait — probe.rs handles detection directly
- `ProcessSpawner` trait — not implemented
- `EventBus` trait — not implemented

---

## BOOT SEQUENCE (actual — `main.rs`)

```
Ring 0 — Probe
  probe::run(force_reprobe) → NodeConfig
  Reports: Host OS, compute backend, VRAM

Ring 1 — Storage
  SurrealHttpStorage::init() → Option<Arc<SurrealHttpStorage>>
  If unavailable → NullStorage + NullCoord (graceful degradation)
  Both StoragePort AND CoordPort come from same SurrealHttpStorage instance

Ring 2 — Judge + Coord + Usage
  Load backends.toml (or env var fallback)
  Health-check each configured backend; skip unreachable ones
  Build DeterministicDog (always) + one InferenceDog per reachable backend
  Judge::new(dogs) — seed integrity hash chain from last stored verdict
  DogUsageTracker — load historical usage from DB (survives restarts)
  Background tasks spawned:
    - coord expiry (every 60s) — expire stale sessions + orphaned claims
    - usage flush (every 60s) — batch UPSERT to dog_usage table
    - CCM aggregator (every 5min, configurable via CYNIC_AGGREGATE_INTERVAL)

Ring 3 — REST + MCP
  If --mcp flag: serve MCP over stdio (JSON-RPC 2.0), exit after
  Otherwise: bind REST on CYNIC_REST_ADDR (default 127.0.0.1:3030)
  gRPC (feature-gated, --features grpc): tonic server on [::1]:50051
```

**5-State Health for backends:** UNKNOWN → HEALTHY → DEGRADED → CRITICAL → RECOVERING. Boot with whatever is available. Backends that fail health check at startup are skipped (not fatal).

---

## CIRCUIT BREAKER (per-backend)

```
Closed ──(N failures)──▶ Open ──(cooldown)──▶ HalfOpen ──(success)──▶ Closed
                           │                                              │
                           └──────────── (failure) ──────────────────────┘
```

Replaces per-request health checks (O(N*M)) with periodic probes (O(N)).

---

## COGNITIVE LAYER (the brain — from CRYSTALLIZED-TRUTH)

### 6 Axioms
| Axiom | Essence |
|---|---|
| FIDELITY | Truth loyalty — kenosis: emptying ego to receive truth |
| PHI | Structural harmony — coherence, elegance, proportion |
| VERIFY/FALSIFY | Evidence + Popper's falsification |
| CULTURE | Continuity + patterns — honor lineage |
| BURN | Simplicity + action — destroy excess |
| SOVEREIGNTY | Individual agency — the soul of CYNIC |

**Geometric mean** enforces tension: one weak axiom drags everything down. Q = ⁶√(F × Φ × V × C × B × S), then phi-bounded.

### Dog Trait (model-agnostic evaluator)
See PORT CONTRACTS above for the actual signature. The trait IS the contract.

### 7-Step Cycle
```
PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE
    └───────────────────────────────────────────────────┘
```
Loop closes when EMERGE feeds back to PERCEIVE. Each step = Rust trait.

### 5 Closed Loops (V1 failed from open loops)
1. **LNSP Proprioceptive:** Observe → Aggregate → Judge → Act → re-enter observation
2. **Training:** Verdict cache → extract → reason → train → improved judgment
3. **Q-Learning:** Judgment → outcome → Q-table update → better next judgment
4. **Value → Governance:** Creation → impact → weight emerges → affects decisions
5. **Health → Consciousness:** Metrics → LODController → degradation → stabilization

### Consciousness Levels
| Level | Name | Compute | Description |
|---|---|---|---|
| 0 | REFLEX | Minimal | Pattern match, no reasoning |
| 1 | MICRO | 1 Dog | Single evaluator, fast |
| 2 | MACRO | N Dogs | Multi-evaluator consensus |
| 3 | META | N Dogs + reflection | Self-evaluation of evaluation |

### Innovations (verified novel)
- **Phi-bounded confidence** — max 61.8%, enforced epistemic humility
- **MCTS Temporal** — 7 temporal perspectives, phi as exploration constant
- **CCM** — Ephemeral outputs → persistent wisdom (threshold 0.618, decay 0.382)
- **Triple learning** — Q-Learning + Thompson + EWC (anti-forgetting)
- **Residual Detection** — Dog disagreement > 38.2% = discovery signal

---

## PROBE (probe.rs)

`probe::run(force_reprobe)` — detects hardware capabilities at boot. Returns `NodeConfig` with compute backend and VRAM info. Implements graceful degradation: unknown hardware = safe defaults.

---

## TESTING PYRAMID

```
E2E (curl → REST /judge → Dogs → backends) ........ 1-2 tests
Integration (real adapter + real system) ........... per adapter
Port Contract (trait test suite) ................... per port
Unit (pure domain logic) .......................... per domain fn
```

**Critical pattern:** Port contract tests. ANY implementation of a port must pass the SAME test suite. MockBackend AND OpenAiCompatBackend both pass `inference_port_contract()`.

---

## CURRENT STATE

Implementation status is volatile — check `git log` and source code for ground truth.
Do NOT rely on a static list here. The architecture above is the target; the code is the reality.
**gRPC stack (tonic/prost) is feature-gated behind `--features grpc`. Default build is REST + MCP only.**

## DISTRIBUTED INFERENCE OPTIONS

| Approach | When to use | Tradeoff |
|----------|------------|----------|
| **llama.cpp RPC** | Offload model layers across nodes (tensor parallelism) | Built-in, zero custom code. Requires llama.cpp on all nodes. |
| **OpenAI-compatible HTTP** | Route requests to heterogeneous backends (llama.cpp, Ollama, vLLM) | InferenceDog + ChatPort + circuit breaker. Backends are independent processes. |
| **llama.cpp router mode** | Multi-model on single node (`--model-dir`) | Simple, but single-node only. |

**CYNIC uses OpenAI-compatible HTTP** as the primary interface (InferencePort trait) because it supports heterogeneous backends. llama.cpp RPC is an option for single-model tensor parallelism across nodes.

---

## PHI CONSTANTS

```
phi    = 1.618034  — golden ratio
phi^-1 = 0.618034  — crystallization threshold, max confidence
phi^-2 = 0.382     — decay threshold, anomaly trigger

Verdict thresholds (phi-bounded 0–1 scale):
  HOWL  > 0.5068  (= phi^-1 × 0.82)   — exceptional conviction
  WAG   > 0.382   (= phi^-2)           — positive, passes
  GROWL > 0.236   (= phi^-2 × phi^-1) — cautious, needs work
  BARK  ≤ 0.236                        — reject / insufficient confidence

As percentage of max-confidence (÷ 0.618):
  HOWL  > 82%   WAG  > 61.8%   GROWL > 38.2%   BARK ≤ 38.2%

Score floor: 0.05 (true zero = parsing failure, never real epistemic judgment)
```

---

## CHECKLIST (apply on EVERY CYNIC task)

```
Before coding:
□ Does this touch domain core? If yes, zero external dependencies.
□ Does this need a port trait? If it talks to external systems, yes.
□ Will this work on Linux without #[cfg] in domain code?
□ Does this close a loop? Open loops are V1's failure mode.

During coding:
□ Composition root (main.rs) is the ONLY file with concrete types
□ Every driven dependency goes through a port trait
□ Health is 5-state (UNKNOWN/HEALTHY/DEGRADED/CRITICAL/RECOVERING), never boolean
□ Every backend has a circuit breaker
□ Structured logging on every state change

After coding:
□ Can I swap the implementation without touching domain code?
□ Does MockAdapter pass the same contract tests as RealAdapter?
□ Does the system boot with this component unavailable? (graceful degradation)
□ Would a hostile expert find a SOLID violation here?
□ Confidence ≤ 61.8% on any claim about this code
```
