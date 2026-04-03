# Inference Organ — Design Specification

**Date:** 2026-04-01
**Status:** Draft (rev 3 — post expert panel)
**Review:** Rev 2: 3C+7I resolved. Rev 3: 8 expert-panel fixes (see Appendix B)
**Authors:** T. + Claude Code (co-design session)
**Scope:** cynic-kernel — new `organ/` module for inference lifecycle, routing, health, calibration, and network transport

## Problem Statement

CYNIC's inference infrastructure is a weak foundation. 6 Dogs degraded to 2 because:

- Mistral 7B splits JSON, scores 0.0, inverts SOVEREIGNTY
- Qwen 9B thinking consumes all tokens before producing JSON
- gemini-flash has 94.7% failure rate
- FleetProbe and CircuitBreaker are disconnected (two health systems, zero coordination)
- `SovereignSummarizer` and `OpenAiCompatBackend` are separate HTTP clients to the same server
- No calibration, no capability verification, no quality-based circuit breaking
- No path for remote backends (S.'s node) or cluster-based inference sharing

The kernel cannot keep Dogs alive because it has no organ for inference lifecycle.

## Design Principles

1. **Compound organic growth** — build on the ONE proven loop (stimulus→dogs→verdict→crystal→prompt). Everything serves this loop.
2. **Strong foundation > weak breadth** — Rule 14. Fix the 6→2 degradation before adding features.
3. **Port contracts first** — Rule 8 / K2. Every adapter through a trait. The kernel never sees HTTP, Tailscale, or llama-server.
4. **Network abstractions from day 1** — the right separations for network (Dog ≠ Backend ≠ Node ≠ Cluster) are the same separations that fix the foundation.
5. **USE before architecture** — Rule 22. Asymmetric (one orchestrator) before symmetric (peer consensus). Phase 1 fixes real problems; Phase 3 enables the network.

## Architecture Overview

### Layer Model

```
Layer 0: PLATFORM — sense hardware, OS, network (PlatformPort)
Layer 1: ORGANS   — consume platform signals, manage lifecycle
                    InferenceOrgan, StorageOrgan (future), CoordOrgan (future)
Layer 2: APP      — orchestrate business logic
                    Pipeline, Judge, Introspection
Layer 3: API      — delivery: REST (Axum), MCP (rmcp), PeerTransport (future gRPC)
```

### Module Structure

```
cynic-kernel/src/organ/
├── mod.rs              — InferenceOrgan facade (boot, shutdown, provides)
├── registry.rs         — Node, Backend, Cluster, Dog registration + capability declaration
├── router.rs           — Profile-based routing → Cluster → Backend selection
├── health.rs           — Unified health: CircuitBreaker (availability + quality) + rolling stats
├── calibration.rs      — Golden stimulus library + statistical drift tracking
├── discovery.rs        — Announce/heartbeat + fleet.toml seed
└── transport/
    ├── mod.rs          — NodeTransport trait
    ├── backend.rs      — BackendAdapter (HTTP to inference engines)
    ├── llama_server.rs — LlamaServerAdapter (extends OpenAI-compat with grammar, /props, thinking)
    ├── cloud.rs        — CloudApiAdapter (Gemini, HF — non-sovereign)
    └── peer.rs         — NullPeer (Phase 1) → GrpcPeer (Phase 2, tonic)
```

## Section 0: Config Migration (C1 fix)

### `backends.toml` → `fleet.toml`

Rule 12: "One value, one source." The organ needs a richer config (nodes, clusters, capabilities) that `backends.toml` cannot express. Migration plan:

1. **Phase 1:** Introduce `~/.config/cynic/fleet.toml` as the new SoT. `load_backends()` in `infra/config.rs` reads `fleet.toml` first; if not found, falls back to `backends.toml` with a deprecation warning logged at WARN.
2. **Phase 1 deliverable:** A `migrate_config()` function that reads `backends.toml` and writes `fleet.toml` in the new format. Run once, manually.
3. **Phase 2:** Remove `backends.toml` fallback. `fleet.toml` is the only SoT.

### New Dependencies

- `bitflags = "2"` — for `CapabilitySet`. Zero async, zero monomorphization risk. Low-risk addition.
- `tonic` + `prost` — Phase 4 only (gRPC peer transport). NOT added until Phase 4.

### Layer Import Direction (I4 fix)

```
domain/ (Layer 0)  ← imports nothing from kernel
organ/  (Layer 1)  ← imports from domain/ only
dogs/   (Layer 1)  ← imports from domain/ only
backends/ (Layer 1) ← imports from domain/ + infra/config
pipeline.rs (Layer 2) ← imports from domain/, receives organ outputs via trait objects
judge.rs (Layer 2) ← imports from domain/, receives Dogs + HealthGates
api/ (Layer 3) ← imports from domain/, receives all via AppState trait objects
main.rs (composition root) ← imports everything, sole place that knows InferenceOrgan exists
```

`DogBinding` lives in `organ/registry.rs` (Layer 1). It references `domain::dog::Dog` and `domain::chat::InferenceProfile` — both downward imports. Pipeline (Layer 2) never sees `DogBinding` — it receives `Vec<Box<dyn Dog>>` from the organ via `main.rs`.

## Section 1: Data Model

### Four Entities

Today everything is fused in `BackendConfig`. The organ separates four concerns:

**Node** — a machine on the network.

```rust
pub struct Node {
    pub id: NodeId,
    pub transport: Transport,         // Tailscale, DirectLan, WireGuard
    pub status: NodeStatus,           // Reachable, Unreachable, Gone
    pub last_heartbeat: Option<Instant>,
    pub manifest: NodeManifest,       // services, resources
}

pub struct NodeManifest {
    pub services: Vec<ServiceDescriptor>,
    pub resources: NodeResources,     // vram_mb, ram_mb, cpu_cores, gpu_model
}
```

**Backend** — an inference endpoint on a node.

```rust
pub struct Backend {
    pub id: BackendId,
    pub node_id: NodeId,
    pub service_type: ServiceType,
    pub endpoint: String,             // relative to node address
    pub model: String,
    pub capabilities: CapabilitySet,  // declared
    pub measured: Option<BackendMetrics>, // observed by calibration
    pub health: BackendHealth,        // Healthy, Degraded, Dead
    pub config: BackendConfig,        // timeout, auth, max_tokens, temperature
    pub remediation: Option<RemediationConfig>,  // Expert Fix 5: NOT silently dropped from backends.toml
}

// RemediationConfig migrated from backends.toml — same fields, now per-Backend in fleet.toml
pub struct RemediationConfig {
    pub node: String,                 // SSH target for restart
    pub restart_command: String,      // e.g., "schtasks /run /tn CynicSovereign" (Windows)
    pub max_retries: u32,             // default: 3
    pub cooldown_secs: u64,           // default: 60
}

pub struct BackendMetrics {
    pub tokens_per_second: f32,
    pub json_valid_rate: f32,         // rolling 50-call
    pub mean_latency_ms: u32,
    pub last_calibration: Option<Instant>,
}
```

**Cluster** — a pool of backends sharing a capability set.

```rust
pub struct Cluster {
    pub id: ClusterId,
    pub required_capabilities: CapabilitySet,
    pub backends: Vec<BackendId>,
    pub strategy: ClusterStrategy,    // RoundRobin, Failover, LowestLatency
}

pub enum ClusterStrategy {
    RoundRobin,
    Failover,          // try first healthy, fall to next
    LowestLatency,     // pick measured-fastest
    // M1 fix: when backend.measured is None (pre-calibration),
    // LowestLatency falls back to RoundRobin for that cluster.
    // K14: missing measurement = assume degraded, not optimistic.
}
```

**Dog** — an epistemic validator with a persona.

```rust
// Existing Dog trait stays in domain/dog.rs — unchanged.
// The organ creates InferenceDogs and binds them to clusters.
pub struct DogBinding {
    pub dog_id: String,
    pub cluster_id: ClusterId,
    pub profile: InferenceProfile,
    pub domain_prompts: Arc<HashMap<String, String>>,
}
```

### Capabilities (Expert Fix 3 — rates, not booleans)

Binary capabilities (can/can't) are wrong for LLMs. Mistral produces valid JSON 85% of the time — is that "capable"? The ML engineer is right: this is fundamentally a distribution, not a flag.

**Design: declared capabilities with measured quality scores.**

```rust
/// Declared in config — what the backend CLAIMS to support
pub struct DeclaredCapabilities {
    pub json: bool,
    pub thinking: bool,
    pub scoring: bool,
    pub embedding: bool,
    pub agent_reasoning: bool,
    pub grammar: bool,
}

/// Measured at runtime — what the backend ACTUALLY delivers
/// Updated by DogStats (Phase 1) and calibration (Phase 2)
pub struct MeasuredCapabilities {
    pub json_valid_rate: f64,        // 0.0-1.0, rolling window
    pub scoring_in_range_rate: f64,  // 0.0-1.0, vs golden stimuli (Phase 2)
    pub mean_latency_ms: u32,
    pub tokens_per_second: f32,
}

/// Routing threshold — what the Router requires to consider a backend eligible
pub struct CapabilityThreshold {
    pub min_json_valid_rate: f64,    // default: 0.7 (not 0.8 — cliff avoidance)
    pub min_scoring_rate: f64,       // default: 0.6
    pub max_latency_ms: u32,        // default: 5000
}
```

**Routing uses measured rates, not boolean flags:**
```
eligible = backend.declared.json
        && backend.measured.json_valid_rate >= threshold.min_json_valid_rate
```

A backend at 69% JSON validity is NOT treated identically to one at 0% — the Router sees the actual rate and can prefer backends with higher rates within a cluster (weighted selection). The threshold is the MINIMUM to be eligible, not a cliff that erases all gradation.

Thresholds are configurable per-cluster in fleet.toml. Default is φ⁻¹ × 1.13 ≈ 0.7 for JSON (grounded in φ, not arbitrary).

### Capability (singular) for Events and Revocation (I6 fix)

`CapabilitySet` is a bitflags struct for efficient set operations. For events and revocation, a singular `Capability` enum is also needed:

```rust
#[derive(Debug, Clone, Copy)]
pub enum Capability {
    Json,
    Thinking,
    Scoring,
    Embedding,
    AgentReasoning,
    FastResponse,
    Grammar,
}

impl Capability {
    pub fn as_set(self) -> CapabilitySet { /* maps to corresponding bit */ }
}
```

Both `CapabilityRevoked` events and `DegradedReason::CapabilityRevoked(Capability)` use the singular enum.

### Service Types (extensible beyond inference) (M6 fix)

Phase 1 implements only `Inference` and `Embedding`. Other variants are included for forward-compatibility but annotated. Types referenced by future variants (`StorageQuery`, `StorageResult`) are NOT defined until their phase.

```rust
pub enum ServiceType {
    Inference,
    Embedding,
    // Phase 3+:
    // Storage,
    // Transcription,
    // Coordination,
}

pub enum ServiceCall {
    Inference(InferRequest),
    Embedding(EmbedRequest),
    // Phase 3+: Storage(StorageQuery), ModelManagement(ModelCommand)
}

pub enum ServiceResponse {
    Inference(InferResponse),
    Embedding(Vec<f32>),
    // Phase 3+: Storage(StorageResult), ModelManagement(ModelStatus)
}

/// Chunk type for streaming responses (I2 fix)
pub enum ServiceChunk {
    Token(String),            // single token from streaming inference
    Embedding(Vec<f32>),      // complete embedding (non-streaming)
    Done,                     // stream complete
    Error(TransportError),    // mid-stream error
}
```

Note: `Stream` trait is from `futures_util::Stream` (already in Cargo.toml as `futures-util = "0.3"`).
```

## Section 2: NodeTransport — Multi-Transport Abstraction

### Two Communication Patterns

1. **Kernel → Backend** (inference calls): request-response + streaming. llama-server speaks HTTP — unavoidable. But the kernel doesn't know it's HTTP.
2. **Kernel ↔ Kernel** (peer sync): event-driven, bidirectional, push-based. HTTP is the wrong protocol. gRPC (tonic) is the target for Phase 2.

### Trait Design

```rust
#[async_trait]
pub trait NodeTransport: Send + Sync {
    /// Lightweight health probe (shared across all service types)
    async fn probe(&self, node: &NodeId) -> Result<NodeHealth, TransportError>;

}

/// Separate trait for service lifecycle — not a transport concern (M5 fix)
#[async_trait]
pub trait DiscoveryPort: Send + Sync {
    /// Announce a local service to the network
    async fn announce(&self, descriptor: &ServiceDescriptor) -> Result<(), TransportError>;

    /// Withdraw a service announcement
    async fn withdraw(&self, service_id: &ServiceId) -> Result<(), TransportError>;

    /// Poll for known nodes (from fleet.toml seed + dynamic announces)
    async fn known_nodes(&self) -> Vec<RemoteNode>;
}

pub enum TransportKind {  // M2 fix: renamed from Transport to avoid collision with NodeTransport trait
    Tailscale,     // WAN, MagicDNS resolved
    DirectLan,     // same subnet, static
    WireGuard,     // future: raw WG or Headscale
}
```

### Adapters

| Adapter | Protocol | Discovery | Phase |
|---|---|---|---|
| `LlamaServerAdapter` | HTTP + SSE, OpenAI-compat + GBNF/props/thinking | fleet.toml + MagicDNS | Phase 1 |
| `CloudApiAdapter` | HTTP, provider-specific auth | Static config | Phase 1 |
| `NullPeer` | No-op | N/A | Phase 1 |
| `GrpcPeer` | gRPC (tonic), proto contracts | Tailscale Services | Phase 2+ |

**Expert Fix 2 — Per-service traits, not unified enum:**

Phase 1 uses separate traits per service type. The unified `ServiceCall` enum is deferred to Phase 3 when one transport actually handles multiple services.

```rust
/// Phase 1: separate traits per service type
#[async_trait]
pub trait InferenceTransport: Send + Sync {
    async fn infer(&self, node: &NodeId, request: &InferRequest, profile: &InferenceProfile)
        -> Result<InferResponse, TransportError>;
    async fn stream_infer(&self, node: &NodeId, request: &InferRequest, profile: &InferenceProfile)
        -> Result<Pin<Box<dyn futures_util::Stream<Item = ServiceChunk> + Send>>, TransportError>;
}

#[async_trait]
pub trait EmbeddingTransport: Send + Sync {
    async fn embed(&self, node: &NodeId, request: &EmbedRequest)
        -> Result<Vec<f32>, TransportError>;
}
```

**Expert Fix 6 — Transport selection deferred to Phase 3:**

Phase 1: all nodes use Tailscale. No multi-transport auto-selection. The <5ms LAN switching with hysteresis is a Phase 3 concern when there are actual same-subnet nodes. Attempting it now with 2 nodes on different subnets creates flapping for zero benefit.

## Section 3: Platform Layer (Layer 0)

```rust
/// Domain trait — zero deps on /proc/ or sysinfo
#[async_trait]
pub trait PlatformPort: Send + Sync {
    async fn hardware(&self) -> HardwareSnapshot;
    async fn pressure(&self) -> PressureSnapshot;
    async fn network(&self) -> NetworkSnapshot;
    async fn process_self(&self) -> ProcessSnapshot;
}
```

**Implementation:** `LinuxPlatform` owns `ResourceProbe`, `PressureProbe`, `ProcessProbe`, `NetworkProbe` (existing code, relocated from `infra/probes/`).

**Consumed by:**
- InferenceOrgan → VRAM availability for routing, pressure for throttling, `NodeManifest.resources` for announce
- Introspection → MAPE-K analysis
- Future StorageOrgan → disk space for backup decisions

**What moves OUT of Platform:** `FleetProbe` → InferenceOrgan (it probes inference backends, not hardware). `BackupProbe` → stays independent (storage concern, no StorageOrgan yet).

## Section 4: Router + Profiles + Adaptive Fan-out

### Profile-Based Routing

```rust
// Existing enum in domain/chat.rs, extended with Calibration (I3 fix)
pub enum InferenceProfile {
    Scoring,       // max_tokens=1024, thinking=off, temp=0.3
    Agent,         // max_tokens=8192, thinking=on, temp=default
    Summary,       // max_tokens=1024, thinking=off, temp=0.2
    Infer,         // max_tokens=4096, thinking=on, temp=default
    Calibration,   // max_tokens=512, thinking=off, temp=0.0, timeout=15s
}

// Required match arm additions in domain/chat.rs:
impl InferenceProfile {
    pub fn max_tokens(&self) -> Option<u32> {
        match self {
            // ... existing arms ...
            Self::Calibration => Some(512),
        }
    }
    pub fn disable_thinking(&self) -> bool {
        match self {
            // ... existing arms ...
            Self::Calibration => true,
        }
    }
    pub fn temperature(&self) -> Option<f64> {
        match self {
            // ... existing arms ...
            Self::Calibration => Some(0.0), // deterministic for reproducibility
        }
    }
}
```

### Routing Algorithm

```
1. Profile → required capabilities
   Scoring  → {Json, Scoring}
   Agent    → {Thinking, AgentReasoning}
   Summary  → {Json, FastResponse}
   Infer    → {Json} (minimum)

2. Filter clusters by required capabilities
   cluster.capabilities ⊇ required → eligible

3. Filter backends in cluster by health
   backend.health == Healthy → primary
   backend.health == Degraded → fallback (weight ×0.5)
   backend.health == Dead → skip

4. Select backend by cluster strategy
   RoundRobin → next healthy
   Failover → first healthy, skip to next on error
   LowestLatency → lowest measured.mean_latency_ms

5. Apply profile overrides to request
   max_tokens, temperature, disable_thinking from profile
   grammar schema if backend.capabilities.contains(GRAMMAR)
```

### Adaptive Fan-out (Judge Integration)

Today the Judge fans out to ALL Dogs on every `/judge` call. The organ enables adaptive fan-out based on confidence target:

```rust
pub struct FanOutPolicy {
    pub min_dogs: usize,    // always consult at least N (default: 2 for quorum)
    pub max_dogs: usize,    // never exceed N (default: all)
    pub confidence_target: f64,  // φ-convergence C(n) target
}
```

Phase 1: fan-out = all Dogs (current behavior, no regression).
Phase 2: adaptive fan-out where low-stakes queries use 2 Dogs (C(0)=0.618) and high-stakes/crystallization uses all.

`FanOutPolicy` is owned by `InferenceOrgan` and injected into the Judge at construction (M7 fix). The Judge uses it in `evaluate()` to decide how many Dogs to dispatch.

## Section 5: Unified Health System

### Health State Reconciliation (C2 fix)

Three health representations exist today. The organ consolidates them:

| Existing | Location | Disposition |
|---|---|---|
| `BackendStatus { Unknown, Healthy, Degraded, Critical, Recovering }` | `domain/inference.rs` | **Replaced by `DogState`**. Remove `BackendStatus` enum. Migrate callsites (3: `BackendPort::health()`, `main.rs` boot check, `FleetProbe`). |
| `CircuitState { Closed, Open, HalfOpen }` | `infra/circuit_breaker.rs` | **Internal to organ health.** CB becomes an implementation detail inside `organ/health.rs`. Not exported. `CircuitState` is never visible to Judge or Pipeline. |
| `HealthGate` trait | `domain/health_gate.rs` | **Stays as the domain contract.** The organ implements `HealthGate` per Dog, exposing `should_allow()` and `state()` to the Judge. Internally, `should_allow()` checks both CB state AND quality gates. |

Result: **one health state per Dog (`DogState`)**, exposed via `HealthGate`. CB is internal plumbing, `BackendStatus` is deleted.

### Three-Layer Health

Today: CircuitBreaker (availability) and FleetProbe (observability) are disconnected. The organ unifies them:

**Layer 1 — Per-call gate (structural):**
Split `record_failure()` into two independent counters:
- `record_availability_failure()` → timeout, 5xx, unreachable. Existing CB behavior (3 consecutive → Open).
- `record_parse_failure()` → HTTP 200 but invalid JSON, missing axioms. Window-rate gate: `json_valid_rate < 0.5 over last 10 calls` → Degraded.

**Layer 2 — Rolling baseline (statistical):**
Per-Dog, per-axiom online statistics (Expert Fix 7 — Welford's algorithm, not fixed window):

```rust
/// Welford's online algorithm — numerically stable, O(1) per update, no circular buffer needed
pub struct WelfordAccumulator {
    pub count: u64,
    pub mean: f64,
    pub m2: f64,  // sum of squares of differences from mean
}

impl WelfordAccumulator {
    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }
    pub fn stddev(&self) -> f64 {
        if self.count < 2 { return f64::MAX; }  // K14: unknown = assume worst
        (self.m2 / (self.count - 1) as f64).sqrt()
    }
}

pub struct DogStats {
    pub axioms: [WelfordAccumulator; 6],  // per-axiom online stats
    pub json_valid: WelfordAccumulator,    // tracks 0.0/1.0 per call → rate = mean
    pub last_updated: Instant,
}
```
Cost: 18 f64 + 6 u64 per Dog in `Arc<tokio::sync::RwLock<DogStats>>` (tokio RwLock, NOT std — safe across `.await`).

Why Welford over EWMA: Welford gives both mean AND stddev for drift detection (2σ). EWMA gives smoothed mean but no natural stddev. For Phase 2 calibration, both are needed. Welford also naturally handles the bootstrap problem: `stddev()` returns `MAX` until count ≥ 2, which means drift detection is automatically disabled during warmup (K14: unknown = assume degraded).

**Hook location (C3 fix):** `DogStats` is updated in `judge.rs` AFTER `phi_bound()` is applied (around line 252-257), NOT in `dogs/inference.rs::validate_scores()`. Reason: `validate_scores()` runs on raw scores before clamping — drift statistics must be computed on the values that actually enter consensus (phi-bounded). The organ exposes an `update_stats(dog_id, &PhiBoundedScores)` method called by the Judge after aggregation.

```
dogs/inference.rs::validate_scores()  → rejects garbage (zero flood, degenerate variance)
judge.rs line ~257: phi_bound()       → clamps to [0.05, 0.618]
judge.rs line ~260: organ.update_stats(dog_id, &bounded_scores)  ← HERE
judge.rs line ~301: trimmed_mean()    → consensus
```

Drift detection: if any axiom mean deviates >2σ from baseline → `DogStateChanged` event + `Degraded` state.

**Layer 3 — Calibration (compound):**
Golden stimulus set (5 crystals with known score ranges) + continuous tracking on real traffic.
- At boot: run all golden stimuli against all backends. Fail → `Degraded`, not `Dead` (K14: missing = assume degraded).
- Every 30 minutes: re-run golden set as background task. **K6 compliance: outer loop wrapped in `tokio::time::timeout(Duration::from_secs(180))`.** With 5 backends × 5 stimuli × 15s per call, worst case = 375s. Timeout at 180s means at most 12 probes complete before cutoff; remaining are skipped and logged (I7 fix).
- Continuous: update `DogStats` on every real call.

### Dog States

```rust
pub enum DogState {
    Healthy,                    // all checks pass
    Degraded {                  // participates with weight ×0.5
        reason: DegradedReason,
        since: Instant,
    },
    Unavailable {               // excluded from fan-out
        reason: String,
        since: Instant,
    },
}

pub enum DegradedReason {
    ParseFailureRate(f64),      // json_valid < threshold
    CalibrationDrift(String),   // axiom drift > 2σ
    CapabilityRevoked(Capability),
    HighLatency(u32),           // p95 > 2× cluster mean
}
```

The Judge receives `DogState` alongside each Dog via the `HealthGate` trait and applies weight:
- `Healthy` → weight 1.0
- `Degraded` → weight 0.5 in trimmed mean
- `Unavailable` → `should_allow()` returns false, skipped by Judge

## Section 6: Calibration System (Compound)

### Golden Stimulus Library

```rust
pub struct GoldenStimulus {
    pub id: String,
    pub domain: String,
    pub content: String,
    pub expected_verdict: VerdictKind,    // HOWL, WAG, GROWL, BARK
    pub expected_ranges: AxiomRanges,     // per-axiom [min, max]
}

pub struct AxiomRanges {
    pub fidelity: (f64, f64),
    pub phi: (f64, f64),
    pub verify: (f64, f64),
    pub culture: (f64, f64),
    pub burn: (f64, f64),
    pub sovereignty: (f64, f64),
}
```

Initial set from existing crystals:
1. Sicilian Defense (chess, expected HOWL, Q≈0.6)
2. Fool's Mate (chess, expected BARK, Q<0.15)
3. Scholar's Mate (chess, expected GROWL, Q≈0.3)
4. "The earth is flat" (general, expected BARK)
5. φ-convergence formula (meta, expected HOWL)

### Bootstrap Phase (I5 fix — cold start)

On first boot (no prior calibration data):

1. **Hardcoded seed ranges** ship with the kernel. Conservative: `fidelity: (0.05, 0.618)` for all axioms (the full φ-bounded range). These never reject a Dog — they only validate JSON parsability and structural correctness.
2. **First 20 healthy responses** per Dog refine the seed ranges. The organ tracks these in-memory and narrows the expected range to `(mean - 2σ, mean + 2σ)`.
3. **Calibration becomes strict** after 20 responses. Drift detection activates only after the baseline is established.
4. **Ranges persist to disk** as `~/.config/cynic/calibration.json` (optional, not SoT — rebuilt from live data if missing).

Fresh deployment: all Dogs start `Healthy`, calibration is in "learning mode" for the first ~20 calls, then switches to "enforcing mode."

### Calibration Flow

```
Boot:
  for each backend:
    for each golden stimulus:
      response = call(stimulus, InferenceProfile::Calibration)
      if !valid_json(response) → revoke Capability::Json
      if !in_range(scores, expected_ranges) → log warning
      if all_fail → DogState::Degraded("calibration failed at boot")

Runtime (every 30 min):
  for each healthy backend:
    pick 1 random golden stimulus
    response = call(stimulus, InferenceProfile::Calibration)
    update DogStats
    if drift_detected → emit CalibrationResult event

Continuous (every real call):
  update DogStats.axiom_means/stddevs
  update DogStats.json_valid_rate
  if json_valid_rate < 0.5 over 10 calls → DogState::Degraded
```

## Section 7: Discovery + Announce Protocol

### Phase 1 (Asymmetric)

Static discovery from config:

```toml
# ~/.config/cynic/fleet.toml (extended)
[node.cynic-core]
address = "<TAILSCALE_CORE>:3030"
transport = "tailscale"

[node.cynic-gpu]
address = "<TAILSCALE_GPU>"
transport = "tailscale"

[[node.cynic-gpu.backend]]
id = "qwen35-9b-gpu"
endpoint = ":8080/v1"
model = "qwen3.5-9b"
capabilities = ["json", "thinking", "scoring", "grammar"]
context_size = 32768
timeout_secs = 30

[[node.cynic-gpu.backend]]
id = "embed-gpu"
endpoint = ":8081/v1"
model = "qwen3-embedding-0.6b"
capabilities = ["embedding"]

[cluster.sovereign-scoring]
required = ["json", "scoring"]
strategy = "lowest_latency"
backends = ["qwen35-9b-gpu", "gemma-4b-core"]

[cluster.cloud-frontier]
required = ["json", "scoring"]
strategy = "round_robin"
backends = ["gemini-flash", "qwen-7b-hf"]
```

The organ loads `fleet.toml` at boot, constructs Nodes/Backends/Clusters, starts health probes.

### Phase 2 (Dynamic Announce)

When a remote node starts (e.g., S.'s machine):

1. Node runs `cynic-announce` (lightweight binary or systemd unit)
2. Registers via Tailscale Services (`tailscale serve`)
3. Kernel discovers via MagicDNS or periodic fleet scan
4. Kernel probes the new node → receives `NodeManifest`
5. Kernel registers backends, adds to eligible clusters
6. Calibration runs against new backends before routing real traffic

Withdrawal: heartbeat timeout (configurable, default 120s) → `BackendWithdrawn` event → remove from clusters.

## Section 8: Agent Consumption (Hermes, Claude Code)

### MCP Tool Evolution

| Tool | Change | Profile |
|---|---|---|
| `cynic_infer` | New optional `profile` param (default: `Infer`) | Agent, Infer, Summary |
| `cynic_judge` | Transparent — organ behind Dogs | Scoring (implicit) |
| `cynic_health` | Enriched: organ health, dog states, cluster status, calibration | N/A |
| `cynic_dogs` | NEW: per-dog state, capabilities, metrics, cluster binding | N/A |

### Hermes Specifics

- `cynic_infer(profile: "agent")` → organ routes to `Thinking`-capable cluster (Qwen 9B, thinking=on)
- `cynic_infer(profile: "infer")` → organ routes to fastest available
- Hermes declares NEED (profile), organ routes. Hermes never picks a cluster.

### Claude Code Specifics

- Same MCP interface
- `cynic_health` becomes diagnostic: Claude Code can check organ state without SSH
- `cynic_infer` enables sovereignty shift: delegate reasoning to local inference instead of Anthropic tokens

## Section 9: Events + Observability

### New KernelEvent Variants

```rust
DogStateChanged { dog_id, from: DogState, to: DogState, reason: String }
BackendDiscovered { node_id, backend_id, capabilities: CapabilitySet }
BackendWithdrawn { node_id, backend_id }
CalibrationResult { dog_id, passed: bool, drift: Option<CalibrationDrift> }
CapabilityRevoked { backend_id, capability: Capability, measured_rate: f64 }
```

Events flow to:
- `/events` SSE stream (external observability)
- Introspection (MAPE-K analysis → alerts → /health)
- Metrics (Prometheus gauges + counters — Expert Fix 8)
- Potential: self-crystals ("Qwen 9B JSON validity dropped to 43%")

### Prometheus Metrics (Expert Fix 8 — operable at 3am)

The existing `render_prometheus()` in `domain/metrics.rs` is extended. These gauges are non-negotiable for operability:

```
# Dog health state (gauge: 0=unavailable, 1=degraded, 2=healthy)
cynic_dog_state{dog_id="qwen35-9b-gpu"} 2

# JSON validity rate per backend (gauge: 0.0-1.0)
cynic_backend_json_valid_rate{backend_id="qwen35-9b-gpu"} 0.87

# Calibration drift per dog per axiom (gauge: sigma units)
cynic_calibration_drift_sigma{dog_id="qwen35-9b-gpu",axiom="fidelity"} 0.3

# Backend latency (histogram)
cynic_backend_latency_ms{backend_id="qwen35-9b-gpu"} 142

# Inference calls by profile (counter)
cynic_inference_calls_total{profile="scoring"} 1247
cynic_inference_calls_total{profile="agent"} 38
```

`curl /metrics | grep dog_state` must work immediately. SSE events are supplementary, not primary.

## Section 10: Debt Resolution

The organ fixes 10 of 14 identified debts:

| # | Debt | Resolution |
|---|---|---|
| D1 | main.rs god function (699 lines) | `InferenceOrgan::boot(config)` absorbs ~150 lines |
| D2 | tasks.rs:160 concrete SovereignSummarizer | Organ provides `Arc<dyn SummarizationPort>` |
| D3 | Health loop + FleetProbe disconnected | Organ unifies in `health.rs` |
| D4 | SovereignSummarizer constructed 2× | Organ owns one instance |
| D5 | BackendError::ModelNotLoaded never emitted | Calibration at boot verifies model |
| D7 | No /infer in REST | Organ exposes InferPort, REST wires it |
| D8 | InferPort bypasses InferenceProfile | Organ routes all through profiles |
| D9 | CircuitBreaker availability-only | Organ splits structural/availability |
| D10 | No per-Dog rolling stats | Organ calibration module |
| D14 | Silent .ok() in config.rs | Config loading moves to organ with proper error propagation |

Remaining debt (separate tickets):
- D6: K2 violation (raw reqwest in probes) — migrate probes to use PlatformPort HTTP adapter
- D11: K3 observe duplication (REST/MCP) — pipeline concern
- D12: K14 anomaly_detected defaults false — storage concern
- D13: K12 #[allow] without WHY — codebase-wide audit

## Section 11: Implementation Phases

### Phase 1 — Fix the Foundation

**Goal:** Fix the 6→2 Dog degradation. Organ boots, manages backends, unifies health. NO calibration in this phase.

**Scope (strict):**
- `organ/mod.rs` — InferenceOrgan facade: `boot()`, `shutdown()`, `dogs()`, `infer_port()`, `embed_port()`, `summarize_port()`
- `organ/registry.rs` — Node, Backend, Cluster, Dog registration from fleet.toml
- `organ/health.rs` — Unified CB (availability + parse-rate quality gate). `DogStats` with Welford online algorithm. NO golden stimuli yet.
- `organ/transport/inference.rs` — `InferenceTransport` trait (NOT unified `ServiceCall` enum — see Expert Fix 2)
- `organ/transport/embedding.rs` — `EmbeddingTransport` trait (separate)
- `organ/transport/llama_server.rs` — Grammar, /props, thinking control
- `organ/transport/cloud.rs` — Gemini, HF adapters
- Wire organ in `main.rs` — replace ~150 lines of manual construction
- `fleet.toml` with `RemediationConfig` (see Expert Fix 5)
- Fix D1, D2, D3, D4, D7, D8

**NOT in Phase 1:** Golden stimuli, calibration drift, adaptive fan-out, DiscoveryPort, PeerTransport.

### Phase 2 — Calibration + Quality Gates

**Goal:** Dogs stay healthy automatically. Quality-based circuit breaking. Calibration as compound (stimuli + tracking).

- `organ/calibration.rs` — Golden stimulus library, boot calibration (with bootstrap), periodic re-calibration
- `organ/health.rs` — Add score-distribution drift gate (EWMA, not fixed window — see Expert Fix 7)
- `DogState::Degraded` with weighted consensus in Judge
- `CapabilityRevoked` event on measured failure
- Cross-Dog agreement check on golden stimuli
- Fix D5, D9, D10

### Phase 3 — Network Discovery + Remote Backends

**Goal:** S.'s machine contributes GPU. Dynamic announce/withdraw.

- `organ/discovery.rs` — fleet.toml seed + Tailscale Services polling + heartbeat
- `organ/transport/mod.rs` — NodeTransport trait, multi-transport selection
- `NodeManifest` exchange between nodes
- Calibration gates remote backends before routing real traffic
- `BackendDiscovered`/`BackendWithdrawn` events

### Phase 4 — Peer Transport + Symmetric Mode

**Goal:** Each node has its own kernel. Verdicts and crystals propagate.

- `organ/transport/peer.rs` — GrpcPeer (tonic) replacing NullPeer
- Proto contracts for verdicts, crystals, coordination
- Trust model formalization
- PeerTransport integration with PlatformPort

## Non-Goals (Explicit)

- **Model-parallel inference** (Exo, Petals) — CYNIC does request-parallel, not layer-splitting.
- **LLM-as-judge for calibration** — circular at this scale.
- **RouteLLM cascading** (cheap→expensive) — Dogs are independent validators, not alternatives.
- **Semantic routing by embedding** — profile-based routing is sufficient.
- **DHT, Consul, etcd** — overkill for 2-5 nodes.

## Constraints (Expert Fix 4 — not open questions, hard constraints)

### Thinking budget is per-server, not per-request (PHASE 1 BLOCKER)

`--reasoning-budget` in llama.cpp is a startup flag. `disable_thinking` via `chat_template_kwargs` is per-request BUT only works on Qwen3/3.5. This means:

**Constraint:** A single llama-server instance serving Qwen 9B CANNOT safely serve both `InferenceProfile::Scoring` (thinking=off) and `InferenceProfile::Agent` (thinking=on) via `--reasoning-budget`. It CAN serve both via `chat_template_kwargs: {enable_thinking: false}` on Scoring calls — but ONLY for Qwen models.

**Phase 1 design response:**
1. `LlamaServerAdapter` sends `chat_template_kwargs: {enable_thinking: false}` for Scoring/Summary/Calibration profiles when `backend.declared.thinking == true`. This is Qwen-specific and the adapter knows it.
2. For non-Qwen models that don't support `chat_template_kwargs`: the backend must NOT declare `thinking` capability, OR the node must run two llama-server instances (one for scoring, one for agent).
3. `fleet.toml` documents this: a Backend entry is one llama-server instance with one thinking mode. Two profiles on the same model = two Backend entries on different ports.

This is NOT deferred. It's a concrete architectural constraint for Phase 1.

### Trust model (deferred to Phase 3+)

BFT (3f+1) + Tailscale WireGuard mutual auth. Formalize when implementing PeerTransport.

### Hot-swap model loading (deferred)

llama-server router mode supports `/models/load`/`/models/unload` but no graceful drain. Defer until model lifecycle management is needed.

## Research Sources

- Claude Code v2.1.88 leak analysis — QueryEngine centralized inference, model alias system, capability declaration, hook architecture
- ClawRouter (BlockRunAI) — 15-dimension weighted routing, <1ms, capability filtering, per-model cooldown
- OpenFang (RightNow-AI) — agent.toml capability whitelist, rate budget per agent
- RouteLLM — cost-quality cascading, single-scalar threshold
- Exo, Petals, llama.cpp RPC — model-parallel patterns (rejected: CYNIC is request-parallel)
- Tailscale Services (GA) — zero-config service discovery for small meshes
- LLM output drift monitoring (arXiv 2511.07585) — structural vs semantic drift
- ZenML 1200-deployment study — continuous model evaluation, golden datasets
- llama-server GBNF grammar — JSON schema enforcement via response_format
- llama.cpp --reasoning-budget — thinking token control (per-server, not per-request)

## Appendix A: Spec Review Resolution Log

| ID | Severity | Issue | Fix |
|---|---|---|---|
| C1 | CRITICAL | fleet.toml vs backends.toml naming | Added Section 0: migration plan, fleet.toml replaces backends.toml with fallback |
| C2 | CRITICAL | Three health states (BackendStatus, CircuitState, DogState) | Added reconciliation table in Section 5: BackendStatus deleted, CB internal, HealthGate stays |
| C3 | CRITICAL | DogStats hook at wrong location (validate_scores vs judge.rs) | Fixed in Section 5: update after phi_bound() in judge.rs ~line 260, with explicit call chain |
| I1 | IMPORTANT | bitflags/tonic not in Cargo.toml | Added to Section 0: bitflags Phase 1, tonic Phase 4 |
| I2 | IMPORTANT | ServiceChunk undefined, Stream import | Defined ServiceChunk enum, noted futures_util import path |
| I3 | IMPORTANT | InferenceProfile::Calibration match arms | Added explicit match implementations in Section 4 |
| I4 | IMPORTANT | DogBinding layer ownership | Added layer import direction table in Section 0 |
| I5 | IMPORTANT | Bootstrap phase for golden ranges | Added 4-step bootstrap in Section 6 (seed→refine→strict→persist) |
| I6 | IMPORTANT | Capability singular type missing | Added Capability enum alongside CapabilitySet in Section 1 |
| I7 | IMPORTANT | Calibration task K6 timeout | Added 180s outer timeout in Section 5 |
| M1 | MINOR | LowestLatency fallback when measured=None | Added K14 comment: fall back to RoundRobin |
| M2 | MINOR | Transport/NodeTransport name collision | Renamed to TransportKind |
| M5 | MINOR | announce/withdraw in transport trait | Extracted to separate DiscoveryPort trait |
| M6 | MINOR | Storage/Transcription variants undefined | Annotated as Phase 3+ with comments |
| M7 | MINOR | FanOutPolicy has no owner | Added: owned by InferenceOrgan, injected into Judge |

## Appendix B: Expert Panel Resolution Log (Rev 3)

Three reviewers (Senior Rust, AI/ML Infra, DevOps/SRE) all returned REVISE. 8 fixes applied:

| # | Expert | Issue | Fix applied |
|---|---|---|---|
| E1 | Rust | Phase 1 too large (calibration + organ) | Cut calibration to Phase 2. Phase 1 = registry + health + routing only |
| E2 | Rust | `ServiceCall` enum premature for Phase 1 | Replaced with separate `InferenceTransport` + `EmbeddingTransport` traits |
| E3 | ML | Capability = binary flag, should be rate | Replaced `CapabilitySet` bitflags with `DeclaredCapabilities` (bool) + `MeasuredCapabilities` (f64) + `CapabilityThreshold` (min rates) |
| E4 | ML | Thinking per-server is Phase 1 blocker, not open question | Moved from "Open Questions" to "Constraints" with concrete design response |
| E5 | DevOps | fleet.toml silently drops RemediationConfig | Added `RemediationConfig` to Backend struct, migrated from backends.toml |
| E6 | DevOps | Transport <5ms auto-selection will flap | Deferred multi-transport to Phase 3. Phase 1 = Tailscale only |
| E7 | Rust+ML | DogStats needs specified online algorithm, tokio::sync not std | Specified Welford's algorithm with code, tokio::sync::RwLock, K14-safe bootstrap |
| E8 | DevOps | Monitoring insufficient for alerting | Added 5 named Prometheus gauges/counters, `curl /metrics | grep dog_state` requirement |

Additional ML feedback noted for Phase 2 (not blocking Phase 1):
- Cross-Dog agreement check on golden stimuli
- Domain-stratified drift detection (chess vs general)
- Confidence intervals on scores → weighted trimmed_mean
- Prompt sensitivity (temp=0.0 calibration vs temp=0.3 operational)
