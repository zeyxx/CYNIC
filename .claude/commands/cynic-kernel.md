
# CYNIC Kernel — Canonical Architecture Reference

*Stable truths and relationships — not copied signatures. For exact APIs, read the source files pointed to below. A broken pointer is a loud signal; a stale signature is a silent lie.*

---

## IDENTITY

CYNIC is an **epistemic immune system** — independent AI validators reaching consensus under mathematical doubt. φ-bounded confidence (max 61.8%). Sovereignty is non-negotiable.

**Triple Agnosticism (non-negotiable):**
- **Hardware-agnostic:** RPi, gaming PC, VPS, cluster. Probe detects capabilities.
- **Model-agnostic:** Swap any LLM. Dog interface is the contract. Open-source models are default.
- **Domain-agnostic:** Chess, trading, code review, geopolitics. Axioms are universal.

---

## TOPOLOGY

| Node | Role | OS |
|---|---|---|
| **`<TAILSCALE_UBUNTU>`** | Kernel, REST API, MCP server | Linux |
| **`<TAILSCALE_FORGE>`** | GPU inference worker (sovereign dog) | Windows |

Hardware specs are volatile — use `probe::run()` output or Tailscale MCP for current state. **Network:** Tailscale VPN between nodes — but NOT the only topology. Localhost is equally valid.

---

## HEXAGONAL ARCHITECTURE

```
        DRIVING ADAPTERS (in)               DOMAIN CORE                DRIVEN ADAPTERS (out)
        ====================               ============               ====================
        REST (axum) ─────────┐                                        OpenAiCompatBackend ── HTTP ── llama-server
        MCP (rmcp/stdio) ────┼────────▶ Domain types + Port traits    SurrealHttpStorage ── HTTP ── SurrealDB
                              │                                        probe ── /sys ── Linux kernel
                              │
        [gRPC: feature-gated, NOT live — --features grpc to enable]
```

**Dependency Rule:** Adapters depend on Ports (traits) in `domain/`. Domain depends on NOTHING external. `main.rs` (composition root) wires adapters to ports — it is THE ONLY FILE with concrete adapter types.

---

## PORT CONTRACTS (relationships — read source for signatures)

The port hierarchy is the stable architecture. Read the actual trait definitions from source.

### Trait Hierarchy

```
BackendPort (name + health)
├── ChatPort (+ chat method) — used by Dogs for axiom evaluation
└── InferencePort (+ capability + infer) — used by gRPC/MCTS
    └── InferenceRouter — routes across multiple InferencePort backends
```

- `BackendPort` defines `name()` and `health()` ONCE — both children inherit
- `ChatPort` adds a single `chat(system, user)` method → returns text + token counts
- `InferencePort` adds `capability()` + `infer(request)` → structured inference
- Dogs return **raw** `AxiomScores` (not phi-bounded). The kernel phi-bounds and aggregates.

### Storage & Coordination

- `StoragePort` — persistence (verdicts, crystals, observations). `SurrealHttpStorage` implements it via HTTP to SurrealDB. `NullStorage` provides graceful degradation (always Ok, never persists).
- `CoordPort` — multi-agent coordination (register, claim, release, audit). Same `SurrealHttpStorage` implements both ports. `NullCoord` for degradation.

### Key Adapters

| Adapter | Implements | Notes |
|---|---|---|
| `OpenAiCompatBackend` | `BackendPort` + `ChatPort` | Universal: Gemini, HF, llama.cpp, vLLM — configured via `BackendConfig` |
| `SurrealHttpStorage` | `StoragePort` + `CoordPort` | One struct, dual impl. Write-through crystal cache. |
| `DeterministicDog` | `Dog` | Heuristic evaluator, no LLM. Evaluates FORM (PHI/BURN/SOVEREIGNTY). |
| `InferenceDog` | `Dog` | Wraps any `Arc<dyn ChatPort>` with a prompt template |
| `BackendRouter` | `InferenceRouter` | gRPC feature-gated only |

### NOT ports (do not exist — never add):
- `GpuDetector` — probe handles detection directly
- `ProcessSpawner` — not implemented
- `EventBus` — not implemented

---

## BOOT SEQUENCE (principles — read `main.rs` for current implementation)

Boot follows a Ring metaphor — lower ring = more fundamental:

```
Ring 0 — Probe:     Detect hardware + environment → NodeConfig
Ring 1 — Storage:   Connect to SurrealDB → graceful degrade to NullStorage if unavailable
Ring 2 — Dogs:      Load backend configs → health-check → build DeterministicDog + InferenceDogs
         Judge:     Wire dogs → seed hash chain from last stored verdict
         Background: Coord expiry, usage flush, CCM aggregation (all periodic)
Ring 3 — Serve:     --mcp → stdio MCP | otherwise → REST on CYNIC_REST_ADDR
```

**Invariants:**
- Boot with whatever is available — unreachable backends are skipped, not fatal
- `DeterministicDog` is ALWAYS present (free, no LLM dependency)
- `SurrealHttpStorage` implements BOTH `StoragePort` and `CoordPort` — one struct
- Health is 5-state: UNKNOWN → HEALTHY → DEGRADED → CRITICAL → RECOVERING

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

### 7-Step Cycle
```
PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE
    └───────────────────────────────────────────────────┘
```
Loop closes when EMERGE feeds back to PERCEIVE.

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

## TESTING PYRAMID

```
E2E (curl → REST /judge → Dogs → backends) ........ 1-2 tests
Integration (real adapter + real system) ........... per adapter
Port Contract (trait test suite) ................... per port
Unit (pure domain logic) .......................... per domain fn
```

**Critical pattern:** Port contract tests. ANY implementation of a port must pass the SAME test suite.

---

## DISTRIBUTED INFERENCE OPTIONS

| Approach | When to use | Tradeoff |
|----------|------------|----------|
| **llama.cpp RPC** | Offload model layers across nodes (tensor parallelism) | Built-in, zero custom code. Requires llama.cpp on all nodes. |
| **OpenAI-compatible HTTP** | Route requests to heterogeneous backends (llama.cpp, Ollama, vLLM) | ChatPort + circuit breaker. Backends are independent processes. |
| **llama.cpp router mode** | Multi-model on single node (`--model-dir`) | Simple, but single-node only. |

**CYNIC uses OpenAI-compatible HTTP** as the primary interface because it supports heterogeneous backends.

---

## CURRENT STATE

Implementation status is volatile — check `git log` and source code for ground truth.
Do NOT rely on a static list here. The architecture above is the target; the code is the reality.
**gRPC stack (tonic/prost) is feature-gated behind `--features grpc`. Default build is REST + MCP only.**

---

## SOURCE MAP (read these for current signatures — pointers, not copies)

When you need exact trait signatures, struct definitions, or implementation details, read these files. If a path doesn't resolve, grep for the trait/type name — the file was likely moved.

| What | Where | Why you'd read it |
|---|---|---|
| `BackendPort`, `InferencePort`, `InferenceRouter` | `domain/inference.rs` | Port contracts for inference backends |
| `ChatPort` | `domain/chat.rs` | Port contract for Dog ↔ LLM communication |
| `StoragePort` | `domain/storage.rs` | Persistence port (verdicts, crystals, observations) |
| `CoordPort` | `domain/coord.rs` | Multi-agent coordination port |
| `Dog` trait, `Stimulus`, `AxiomScores`, `Verdict` | `domain/dog.rs` | Core domain types + evaluator contract |
| `Crystal`, `aggregate_observations` | `domain/ccm.rs` | CCM crystallization logic |
| `DogUsageTracker` | `domain/usage.rs` | Per-dog usage tracking |
| `DeterministicDog` | `dogs/deterministic.rs` | Heuristic evaluator (no LLM) |
| `InferenceDog` | `dogs/inference.rs` | LLM-backed evaluator |
| `OpenAiCompatBackend` | `backends/openai.rs` | Universal backend adapter |
| `BackendRouter` | `backends/router.rs` | gRPC-only inference router |
| `SurrealHttpStorage` | `storage/mod.rs` + `storage/surreal.rs` | DB adapter (struct + trait impls) |
| `Judge` | `judge.rs` | Orchestrator (fan-out to Dogs, hash chain, aggregation) |
| `AppState`, request/response types | `api/rest/types.rs` | REST API shared state |
| REST route handlers | `api/rest/*.rs` | Individual endpoint implementations |
| `CynicMcp` | `api/mcp/mod.rs` | MCP server (rmcp, stdio) |
| `BackendConfig`, `load_backends` | `infra/config.rs` | Backend configuration loading |
| `CircuitBreaker` | `infra/circuit_breaker.rs` | Per-backend circuit breaker |
| Boot sequence | `main.rs` | Composition root — Ring 0→3 |
| Probe (hardware detection) | `probe/mod.rs` | `probe::run()` → `NodeConfig` |

All paths relative to `cynic-kernel/src/`. Prefixed `domain/` = pure domain, zero external deps.

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
