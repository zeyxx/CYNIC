# CYNIC Resource Management — 5-Phase Plan

**Date**: 2026-10-03
**Status**: Approved, Phase 1 in progress
**Principle**: Big bang extraction to reveal causal chains, then organic detailing.

---

## Phase 1: Extraction big bang (IN PROGRESS)

**Goal**: Extract ~15k LOC from `cynic-host` (86k) and ~3.5k LOC from `cynic-observer` (8.3k) into dedicated crates.

**Strategy**: Move all files at once, compile, iterate. Reveal all causal chains.

### Extraction map

| Source | Destination | LOC |
|---|---|---|
| `cynic-host/src/agent_runtime/` | `cynic-agent-execution` (new) | ~2k |
| `cynic-host/src/agent_access/` | `cynic-agent-execution` | ~1.5k |
| `cynic-host/src/agent_host/` | `cynic-agent-execution` | ~800 |
| `cynic-host/src/mcp_runtime/` | `cynic-mcp` | ~500 |
| `cynic-host/src/source_sync/` | `cynic-vcs` | ~1k |
| `cynic-host/src/repo_access.rs` | `cynic-vcs` | ~650 |
| `cynic-host/src/credential_hardening/` | `cynic-secrets` | ~300 |
| `cynic-host/src/storage_capabilities/` | `cynic-storage` | ~500 |
| `cynic-host/src/storage_identity/` | `cynic-storage` | ~1k |
| `cynic-host/src/framework_bootstrap/` | `cynic-lifecycle` | ~1.2k |
| `cynic-host/src/runtime_layout.rs` | `cynic-lifecycle` | ~880 |
| `cynic-host/src/runtime_harness.rs` | `cynic-lifecycle` | ~690 |
| `cynic-host/src/audit_engine/` | `cynic-forensics` (new) | ~1.5k |
| `cynic-host/src/network_actuation.rs` | `cynic-network` | ~920 |
| `cynic-host/src/transport.rs` | `cynic-network` | ~840 |
| `cynic-host/src/runtime_metrics.rs` | `cynic-observer` | ~820 |
| `cynic-host/src/registry/mod.rs` | `cynic-discovery-host` | ~730 |
| `cynic-host/src/domain/packages.rs` | `cynic-pkg` | ~670 |
| `cynic-host/src/domain/services.rs` | `cynic-host-probes` | ~630 |
| `cynic-observer/src/solana_archive/` | `cynic-zeroclaw-solana` | ~2.5k |
| `cynic-observer/src/consensus.rs` | `cynic-consensus` | ~290 |
| `cynic-observer/src/federation.rs` | `cynic-federation` (new) | ~250 |
| `cynic-observer/src/buffer.rs` | `cynic-protocol` | ~260 |

**Total**: ~18.5k LOC extracted (21% of cynic-host, 42% of cynic-observer)

### New crates

- `cynic-agent-execution` (L2): agent runtime, access, host observation
- `cynic-forensics` (L2): crash detection, correlation, patterns
- `cynic-federation` (L2): cross-host/fleet observation sharing

### Steps

1. Create new crates with `cargo init`
2. `git mv` files to destinations
3. Add dependencies to `Cargo.toml`
4. Fix imports
5. Compile and iterate
6. Delete originals from `cynic-host`/`cynic-observer`

### Risks

| Risk | Mitigation |
|---|---|
| Break compilation | Compile after each file move |
| Lose git history | Use `git mv` not `mv` |
| Miss dependencies | `cargo check` reveals missing imports |
| Regression | Existing tests must pass after each extraction |

---

## Phase 2: Forensics module (PENDING)

**Goal**: Unify crash detection in `cynic-forensics`, correlating all sources.

### Sources

| Source | Content | Frequency |
|---|---|---|
| `dmesg` | OOM kills, soft lockups, kernel panics, hung tasks | Real-time |
| `/var/log/syslog` | System errors, service failures | Real-time |
| `/var/log/kmsg` | Kernel messages | Real-time |
| cgroups | memory.max violations, cpu.max throttling | Per cgroup |
| systemd | Service failures, restart loops | Per service |
| `/proc/pressure` | Memory/CPU pressure stalls | Continuous |

### Types

```rust
pub struct ForensicEvent {
    pub kind: ForensicEventKind,  // OomKill | SoftLockup | KernelPanic | ...
    pub at: Instant,
    pub evidence: String,
    pub severity: Severity,       // Critical | High | Medium | Low
    pub source: ForensicSource,   // Dmesg | Syslog | Kmsg | Cgroup | Systemd | Pressure
}

pub struct ForensicPattern {
    pub events: Vec<ForensicEvent>,
    pub root_cause: String,
    pub recommendation: String,
    pub confidence: f32,
}
```

### Interface

```rust
pub trait ForensicObserver {
    fn observe(&self) -> Vec<ForensicEvent>;
    fn observe_source(&self, source: &ForensicSource) -> Vec<ForensicEvent>;
    fn correlate(&self, events: &[ForensicEvent]) -> Vec<ForensicPattern>;
}
```

### Correlation patterns

- OOM kill → service failure → restart
- Soft lockup → kernel panic → reboot
- Pressure stall → OOM kill

---

## Phase 3: Sharing protocol (PENDING)

**Goal**: Share observations between hosts/fleets, adapting to real needs without bottlenecks.

### Principles

| Principle | Implication |
|---|---|
| No bottleneck | No single central coordinator |
| Adapt to needs | Adaptive protocol, not rigid |
| Operate by failure | Fail fast, learn faster |
| 99.9% uptime | Redundancy, graceful degradation |

### Layers

| Layer | Scope | Protocol |
|---|---|---|
| Local | 1 host | Direct read of `infra/resource-limits.json` |
| Fleet | M hosts, same network | Gossip + LastWriterWins |
| Federated | N fleets, different regions | Vector clocks + cross-fleet patterns |

### Types

```rust
pub struct SharedObservation {
    pub host_id: String,
    pub observation: ResourceObservation,
    pub vector_clock: VectorClock,
    pub emitted_at: Instant,
}

pub struct VectorClock {
    pub clocks: BTreeMap<String, u64>,
}

pub enum SharingMode {
    Local,
    Fleet,
    Federated,
}
```

---

## Phase 4: Anti-oscillation (PENDING)

**Goal**: Prevent limit oscillations by combining hysteresis, cooldown, and reinforcement learning.

### Components

| Component | Mechanism | Parameters |
|---|---|---|
| Hysteresis | Dual threshold (upper: 90%, lower: 70%) | State: Relaxed ↔ Tightened |
| Cooldown | Max adjustments per hour | 3/hour, min 1h between |
| Reinforcement Learning | ε-greedy Q-learning | α=0.1, γ=0.9, ε=0.1 |

### Types

```rust
pub struct Hysteresis {
    pub upper_threshold: f32,
    pub lower_threshold: f32,
    pub state: HysteresisState,  // Relaxed | Tightened
}

pub struct Cooldown {
    pub min_duration: Duration,
    pub max_adjustments_per_hour: u32,
    pub recent_adjustments: VecDeque<Instant>,
}

pub struct ReinforcementLearning {
    pub alpha: f32,
    pub gamma: f32,
    pub epsilon: f32,
    pub q_table: HashMap<State, HashMap<Action, f32>>,
}
```

---

## Phase 5: LLM in the loop (PENDING)

**Goal**: Specialized LLM agent that decides limits based on observations.

### Architecture

```
Input:  ResourceObservation + ForensicEvents + ResourceLimits + ResourcePatterns
Prompt: "Host state: memory 85%, CPU 92%, recent OOM. Propose adjustment."
Output: ResourceAdjustment { memory_delta, cpu_delta, io_delta, net_delta, confidence, justification }
Fallback: confidence < 0.7 → deterministic rules
```

### Types

```rust
pub struct ResourceAdjustment {
    pub memory_delta: i64,
    pub cpu_delta: f32,
    pub io_delta: i64,
    pub net_delta: i64,
    pub confidence: f32,
    pub justification: String,
}

pub trait ResourceAgent {
    fn propose_adjustment(
        &self,
        observation: &ResourceObservation,
        events: &[ForensicEvent],
        limits: &ResourceLimit,
    ) -> ResourceAdjustment;
    
    fn learn(&mut self, events: &[ForensicEvent]);
}
```

---

## Execution order

```
Phase 1 (extraction) ──→ Phase 2 (forensics) ──→ Phase 3 (sharing)
                                                        │
                                                        ▼
                                              Phase 4 (anti-oscillation)
                                                        │
                                                        ▼
                                              Phase 5 (LLM agent)
```

**Dependencies**:
- Phase 2 depends on Phase 1 (forensics crate created during extraction)
- Phase 3 depends on Phase 1 (federation crate created during extraction)
- Phase 4 depends on Phase 2 (needs forensic events)
- Phase 5 depends on Phases 2+4 (needs events + anti-oscillation)

---

## Success criteria

| Phase | Criteria |
|---|---|
| 1 | All crates compile, tests pass, cynic-host < 71k LOC |
| 2 | All crash sources detected, patterns correlated |
| 3 | Observations shared across hosts without bottleneck |
| 4 | No oscillations, stable limits |
| 5 | LLM proposes adjustments with confidence > 0.7 |
