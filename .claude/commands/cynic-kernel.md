<!-- Auto-invocation via ~/.claude/commands/cynic-skills/cynic-kernel/ — no frontmatter here to avoid duplication -->

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
| **forge** (kairos) | Kernel, orchestration, gRPC server | i5-6500T, 11GB RAM | Linux |
| **Desktop** (Windows) | llama-server, GPU worker | Ryzen 7 5700G APU, 32GB RAM | Windows |
| **Laptop** | Lighter models | Ryzen 7 5700U, 16GB RAM | Windows |

**Network:** Tailscale VPN between nodes — but NOT the only topology. Localhost is equally valid. A machine capable of its own inference does NOT proxy to another.

---

## HEXAGONAL ARCHITECTURE

```
        DRIVING ADAPTERS (in)               DOMAIN CORE                DRIVEN ADAPTERS (out)
        ====================               ============               ====================
        gRPC MuscleHAL ──────┐
        gRPC KPulse ─────────┤         InferenceRequest/Response     LlamaCppBackend ── HTTP ── llama-server
        gRPC Vascular ───────┼────────▶ BackendCapability             SurrealDbAdapter ── WS ── SurrealDB
        gRPC CogMemory ──────┤         BackendStatus (3-state)       SysfsDetector ── /sys ── Linux kernel
        CLI (future) ────────┘         NodeConfig (value object)     WmiDetector ── PowerShell ── Windows WMI
                                       HealthState enum              NvidiaSmiDetector ── subprocess ── nvidia-smi
                                       SovereigntyAdvice (pure fn)   TokioProcessAdapter ── tokio ── OS process
                                                                     VascularAdapter ── mpsc ── event bus
```

**Dependency Rule:** Adapters depend on Ports (traits) which live in Domain Core. Domain Core depends on NOTHING external. `main.rs` (composition root) wires adapters to ports. `main.rs` is THE ONLY FILE with concrete types.

---

## 5 PORT CONTRACTS

### Port 1: InferencePort (EXISTS — backend.rs)
```rust
#[async_trait]
pub trait InferencePort: Send + Sync {
    fn capability(&self) -> &BackendCapability;
    async fn infer(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError>;
    async fn health(&self) -> BackendStatus;
}
```
**Adapters:** LlamaCppBackend (exists), MockBackend (exists), future: OllamaBackend, VllmBackend, RemoteBackend

### Port 2: StoragePort (NEW)
```rust
#[async_trait]
pub trait StoragePort: Send + Sync {
    async fn store_fact(&self, fact: Fact) -> Result<(), StorageError>;
    async fn query_facts(&self, key: &str) -> Result<Vec<Fact>, StorageError>;
    async fn register_trust(&self, entry: TrustEntry) -> Result<(), StorageError>;
    async fn verify_trust(&self, entry: &TrustEntry) -> Result<bool, StorageError>;
}
```

### Port 3: GpuDetector (NEW)
```rust
pub trait GpuDetector: Send + Sync {
    fn detect(&self) -> Option<ComputeInfo>;
    fn name(&self) -> &str;
}
```
**Selection:** `cfg!(target_os)` in composition root. Domain has ZERO `#[cfg]` gates.

### Port 4: ProcessSpawner (NEW)
```rust
#[async_trait]
pub trait ProcessSpawner: Send + Sync {
    async fn spawn(&self, cmd: &str, args: &[&str]) -> Result<ProcessHandle, SpawnError>;
    async fn kill(&self, handle: &ProcessHandle) -> Result<(), SpawnError>;
    async fn wait(&self, handle: &ProcessHandle) -> Result<ExitStatus, SpawnError>;
}
```

### Port 5: EventBus (NEW)
```rust
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, topic: &str, payload: &str) -> Result<(), BusError>;
    async fn subscribe(&self, topics: &[&str]) -> Result<EventReceiver, BusError>;
}
```

### NOT ports (cold paths — good functions):
- Config persistence (dirs::home_dir() + toml)
- IO benchmark (pure function)
- Sovereignty advisor (pure function)
- Model scanner (walkdir — cross-platform)
- Server scanner (reqwest — cross-platform)

---

## BOOT SEQUENCE (composition root)

```
main.rs:
1. Parse config from env vars
2. Select platform adapters (cfg!(target_os) for GPU detectors)
3. Run probe (injected GpuDetectors) → NodeConfig
4. Connect storage (circuit breaker) → Box<dyn StoragePort>
5. Create inference pipeline → BackendRouter with discovered backends
6. Create event bus → Box<dyn EventBus>
7. Wire gRPC services (driving adapters)
8. Start gRPC server
9. Report boot health: HEALTHY / DEGRADED / CRITICAL
```

**3-State Health:** HEALTHY, DEGRADED, CRITICAL — not boolean. Boot with whatever is available. Report degradation via HeresyNotice. Refuse inference only at CRITICAL.

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

**Geometric mean** enforces tension: one weak axiom drags everything down. Q = 100 × ⁶√(F × Φ × V × C × B × S / 100⁶)

### Dog Trait (model-agnostic evaluator)
```rust
#[async_trait]
pub trait Dog: Send + Sync {
    fn name(&self) -> &str;
    async fn evaluate(&self, stimulus: Stimulus) -> AxiomScores;
}
```
Each Dog wraps any LLM (open-source, API, deterministic code). The trait IS the contract.

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

## PROBE REFACTOR (probe.rs 897 LOC → 10 modules)

| Module | Responsibility | Platform-specific? |
|---|---|---|
| `probe/mod.rs` | Orchestration | No |
| `probe/gpu.rs` | GPU detection via trait | Via trait |
| `probe/gpu_linux.rs` | SysfsDetector | Linux only |
| `probe/gpu_windows.rs` | WmiDetector | Windows only |
| `probe/gpu_nvidia.rs` | NvidiaSmiDetector | Cross-platform |
| `probe/models.rs` | GGUF + Ollama discovery | No (walkdir) |
| `probe/servers.rs` | Running server discovery | No (reqwest) |
| `probe/system.rs` | Hardware info | No (sysinfo) |
| `probe/advisor.rs` | SovereigntyAdvisor | No |
| `probe/persistence.rs` | Config load/save | No (dirs + toml) |

**Key:** `probe/mod.rs::run()` takes `Vec<Box<dyn GpuDetector>>`. Composition root provides platform detectors. Probe has ZERO `#[cfg]` gates.

---

## WINDOWS FIXES

| Problem | Fix |
|---|---|
| `$HOME` fallback to `/tmp` | `dirs::home_dir()` |
| Drive scan C-F only | `sysinfo::Disks::new_with_refreshed_list()` |
| AVX2 hardcoded true | `std::arch::is_x86_feature_detected!("avx2")` |
| `[::1]:50051` fails without IPv6 | Try IPv6, fallback to `127.0.0.1:50051` |
| GPU detection | WmiDetector behind GpuDetector trait |
| `/proc/cpuinfo` | `#[cfg(target_os = "linux")]` gate |

---

## TESTING PYRAMID

```
E2E (grpcurl → kernel → llama-server) ............. 1-2 tests
Integration (real adapter + real system) ........... per adapter
Port Contract (trait test suite) ................... per port
Unit (pure domain logic) .......................... per domain fn
```

**Critical pattern:** Port contract tests. ANY implementation of a port must pass the SAME test suite. MockBackend AND LlamaCppBackend both pass `inference_port_contract()`.

---

## CURRENT STATE

Implementation status is volatile — check `git log` and source code for ground truth.
Do NOT rely on a static list here. The architecture above is the target; the code is the reality.

## DISTRIBUTED INFERENCE OPTIONS

| Approach | When to use | Tradeoff |
|----------|------------|----------|
| **llama.cpp RPC** | Offload model layers across nodes (tensor parallelism) | Built-in, zero custom code. Requires llama.cpp on all nodes. |
| **OpenAI-compatible HTTP** | Route requests to heterogeneous backends (llama.cpp, Ollama, vLLM) | Our BackendRouter + circuit breaker. Backends are independent processes. |
| **llama.cpp router mode** | Multi-model on single node (`--model-dir`) | Simple, but single-node only. |

**CYNIC uses OpenAI-compatible HTTP** as the primary interface (InferencePort trait) because it supports heterogeneous backends. llama.cpp RPC is an option for single-model tensor parallelism across nodes.

---

## PHI CONSTANTS

```
phi    = 1.618034  — golden ratio
phi^-1 = 0.618034  — crystallization threshold, max confidence
phi^-2 = 0.382     — decay threshold, anomaly trigger
HOWL   ≥ 82.0      — exceptional
WAG    ≥ 61.8      — passes
GROWL  ≥ 38.2      — needs work
BARK   < 38.2      — reject
```

---

## CHECKLIST (apply on EVERY CYNIC task)

```
Before coding:
□ Does this touch domain core? If yes, zero external dependencies.
□ Does this need a port trait? If it talks to external systems, yes.
□ Will this work on Windows AND Linux without #[cfg] in domain code?
□ Does this close a loop? Open loops are V1's failure mode.

During coding:
□ Composition root (main.rs) is the ONLY file with concrete types
□ Every driven dependency goes through a port trait
□ Health is 3-state (HEALTHY/DEGRADED/CRITICAL), never boolean
□ Every backend has a circuit breaker
□ Structured logging on every state change

After coding:
□ Can I swap the implementation without touching domain code?
□ Does MockAdapter pass the same contract tests as RealAdapter?
□ Does the system boot with this component unavailable? (graceful degradation)
□ Would a hostile expert find a SOLID violation here?
□ Confidence ≤ 61.8% on any claim about this code
```