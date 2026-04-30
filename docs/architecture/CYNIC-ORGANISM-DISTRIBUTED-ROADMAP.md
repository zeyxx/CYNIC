# CYNIC Organism — Distributed Topology Roadmap

> **Goal:** Design Phase B → Phase D so the organism naturally evolves from centralized kernel to distributed peer network without full rewrites.

---

## Epistemic Status

- **Observed:** Current kernel assumes centralized authority (`registered_dogs` HashMap, `ArcSwap<Judge>`, SurrealDB on single node)
- **Deduced:** Refactoring is staged trait extraction, not architectural rewrite
- **Inferred:** Organic growth possible if we map all authority layers now
- **Conjecture:** Askesis layer (consensus infra) will be the bottleneck, not individual trait refactors

---

## The Authority Layers (What Must Stay Centralized vs. What Can Gossip)

```
LAYER          | What                     | Current Model     | Phase D Model
===============|==========================|===================|========================
LOCAL          | HW probing, processes    | node-local        | ✓ (no change needed)
BEHAVIORAL     | Calibration, scoring     | kernel authority  | ✓ (stays kernel)
STATE          | Roster, contracts        | kernel HashMap    | → RosterPort trait
OBSERVATIONS   | K15 events, crystals     | SurrealDB→kernel  | → ObservationPort
CIRCUIT BREAK  | Dog health gates         | ArcSwap<Judge>    | → ConsensusJudge
ORDERING       | Task queue, schedules    | kernel queue      | → DistributedQueue
CONSENSUS      | "Which Dogs are valid?"  | kernel decides    | → Quorum protocol
```

**The key insight:** Only behavioral authority (calibration, scoring) MUST stay on kernel. Everything else can be distributed with the right abstraction.

---

## Organic Blockers: What Must Exist Before Phase D Works

| Blocker | Location | Severity | Unblocks |
|---------|----------|----------|----------|
| **RosterPort trait** | domain/roster.rs (new) | Medium | Node-to-kernel Dog registration works in both topologies |
| **ObservationPort trait** | domain/observations.rs (new) | Medium | K15 observations gossip instead of being kernel-stored |
| **SurrealDB replication** | storage/surrealdb.rs | High | Crystals survive kernel death, quorum can decide validity |
| **Askesis consensus layer** | askesis/ (NEW subsystem) | CRITICAL | All distributed decisions need a quorum backbone |
| **Gossip protocol** | askesis/gossip (NEW) | CRITICAL | RosterPort + ObservationPort need transport |
| **Consensus Judge** | domain/judge.rs refactor | High | Judge swaps must respect network ordering, not just local ArcSwap |
| **Distributed task queue** | agent_tasks/ refactor | Medium | Agent dispatch survives kernel death |

**Critical path:** Askesis infra is the bottleneck. Traits can be designed now (Phase B), but they can't be tested/validated in Phase D without Askesis.

---

## Phase B → Phase D Growth Path (Staged Extractions)

### Phase B (Now): Design Transport-Agnostic Node

**Encode these design intentions (as comments/DESIGN doc, not code):**

```rust
// cynic-node: future-proof skeleton
// The command loop is intentionally transport-agnostic:
//   async fn poll_for_commands() -> Vec<Command> {
//       // Phase C: GET /pending_commands from kernel (HTTP client)
//       // Phase D: query local Soma (which gossips with peers)
//       soma.fetch_pending_commands().await
//   }
// This allows the same binary to work in both topologies.
// No code change needed; only Soma implementation swaps at boot.
```

**No blocker here.** Phase B skeleton is clean and future-proof.

---

### Phase C (May 11 - June): Centralized Soma (Kernel Orchestrates)

**Extract trait, provide single implementation:**

```rust
// domain/roster.rs (NEW)
#[async_trait]
pub trait RosterPort: Send + Sync {
    async fn register(&self, dog_id: &str, ttl_secs: u64) -> Result<(), RosterError>;
    async fn heartbeat(&self, dog_id: &str) -> Result<u64, RosterError>;
    async fn deregister(&self, dog_id: &str) -> Result<(), RosterError>;
    async fn expire_stale(&self) -> Vec<String>;
}

// infrastructure/roster/in_process.rs
pub struct InProcessRoster {
    dogs: Arc<RwLock<HashMap<String, RegisteredDog>>>,
}
impl RosterPort for InProcessRoster { ... }

// AppState change:
// - OLD: registered_dogs: Arc<RwLock<HashMap<String, RegisteredDog>>>
// + NEW: roster: Arc<dyn RosterPort>
```

**Blocker check:**
- ✓ No SurrealDB needed (in-process state)
- ✓ No consensus needed (kernel is SSOT)
- ✓ No Askesis needed (single point of authority)
- ✗ One design question: roster changes must trigger Judge ArcSwap — do we need a notification channel?

**Design decision:** Add `broadcast::Sender<RosterEvent>` to `InProcessRoster`. When a Dog is registered/deregistered, the event fires. The TTL janitor and Judge updater subscribe. This keeps state and behavioral logic loosely coupled.

**Cost:** ~200 lines of refactoring (trait + one implementation).

---

### Phase D (Post-hackathon, July+): Distributed Gossip (Askesis Foundation)

**This phase is BLOCKED until Askesis exists.** Askesis provides:

```rust
// askesis/ — new consensus subsystem
pub struct GossipRoster {
    // Replicated state machine using CRDT or Raft
    roster: Arc<DynAckowledgedStateMap<String, RegisteredDog>>,
    quorum: Arc<Quorum>,
}

impl RosterPort for GossipRoster {
    async fn register(&self, dog_id: &str, ttl_secs: u64) -> Result<(), RosterError> {
        // Propose to quorum: "add this dog"
        // Wait for N/2+1 ACKs before returning
        self.quorum.propose(RegisterOp { dog_id, ttl_secs }).await?;
        // Gossip protocol delivers to all peers
        Ok(())
    }
    // ... etc
}
```

**Prerequisites for GossipRoster to work:**
1. **Askesis consensus protocol** (Raft, PBFT, or CRDT depending on safety needs)
2. **Heartbeat gossip** (instead of point-to-point to kernel)
3. **Bootstrap and rejoin** (nodes discover peers, sync state on startup)
4. **Fault tolerance** (handles node crashes, network partitions)

**Additional refactors in Phase D:**

```rust
// domain/observations.rs — extract ObservationPort
#[async_trait]
pub trait ObservationPort: Send + Sync {
    async fn emit(&self, obs: Observation) -> Result<()>;
    async fn query(&self, domain: &str) -> Result<Vec<Observation>>;
}

// ObservationPort implementations:
// - Phase C: KernelObservation (queries SurrealDB on kernel)
// - Phase D: GossipObservation (CRDT-based replication across nodes)

// domain/judge.rs — extract ConsensusJudge
pub struct ConsensusJudge {
    local_judge: ArcSwap<Judge>,
    roster: Arc<dyn RosterPort>,  // subscribe to changes
    quorum: Arc<Quorum>,           // propose judge swaps
}
// When roster gossips a new Dog, all nodes see it, all update Judge in sync
```

**Cost:** ~1000 lines (Askesis backbone + 3 trait impls + Judge refactor).

---

## Blocking Dependency: Askesis

**Askesis is the foundational layer that organically enables distributed topology.** Without it:

- RosterPort can have `InProcessRoster` (Phase C) ✓
- RosterPort cannot have `GossipRoster` (Phase D) ✗
- Judge swaps cannot be ordered across network ✗
- Crystals cannot survive kernel death ✗
- Observations cannot be replicated ✗

**What Askesis must provide:**

| Component | Purpose |
|-----------|---------|
| **Quorum protocol** | Consensus on state changes (N/2+1 agreement) |
| **Gossip transport** | Broadcast to all nodes, handle packet loss |
| **CRDT/Raft engine** | Replicated state machine (roster, observations, contracts) |
| **Bootstrap** | Nodes discover each other, sync state on join |
| **Failure detection** | Detect dead nodes, exclude from quorum |

**Can Askesis be built organically (without redesigning the kernel)?**

Yes, if designed as a **standalone subsystem that the kernel optionally uses:**

```rust
// ASKESIS LAYER (independent)
pub mod askesis {
    pub mod consensus;      // quorum protocol
    pub mod gossip;         // transport
    pub mod crdt;           // state replication
    pub mod bootstrap;      // peer discovery
}

// KERNEL (uses Askesis optionally)
// At boot: if cfg!(feature = "distributed") {
//     roster = Arc::new(GossipRoster::new(askesis::quorum, askesis::gossip))
// } else {
//     roster = Arc::new(InProcessRoster::new())
// }
```

This is the **sovereign design**: the kernel doesn't mandate Askesis. Askesis is a feature that can be enabled or disabled at compile time. In Phase C, the kernel runs without it. In Phase D, nodes use it for gossip.

---

## Whole-Organism View: All Components That Need Refactoring

### Must Extract (Bottlenecks Distributed Growth)

| Component | Trait | Phase | Blocker | Notes |
|-----------|-------|-------|---------|-------|
| Roster (Dogs registered) | `RosterPort` | C | None | Single impl in Phase C |
| Observations | `ObservationPort` | D | Askesis | Gossip replication needed |
| Judge updates | `ConsensusJudge` | D | Askesis | Ordering guarantees needed |
| Task dispatch | `TaskQueuePort` | D | Askesis | Distributed queue needed |
| System contract | `ContractPort` | D | Askesis | Consensus on canonical config |

### Can Stay Centralized (No Refactor Needed)

| Component | Reason |
|-----------|--------|
| Calibration challenge | Requires live Dog access (node-agnostic, kernel still runs it) |
| Verdict scoring | Dogs are local; scoring is local; results gossip via observations |
| Circuit breaker | Can be per-node (local state) until Askesis, then consensus per phase |

### Optional Refactors (Nice-to-Have, Doesn't Block Phase D)

| Component | Benefit | Cost |
|-----------|---------|------|
| Logger as a trait | Centralize observability, or gossip logs | ~100 lines |
| Health check as trait | Per-node checks, or quorum health | ~50 lines |
| Config loader as trait | Static on kernel, or gossip config changes | ~80 lines |

---

## Design Debt to Avoid (Encode Now)

### DO: Comment Future Refactors

```rust
// TODO(Phase D): Extract as RosterPort
//   - Phase C: InProcessRoster (kernel HashMap)
//   - Phase D: GossipRoster (Askesis CRDT)
//   See: docs/architecture/CYNIC-ORGANISM-DISTRIBUTED-ROADMAP.md
let registered_dogs = Arc::new(RwLock::new(HashMap::new()));
```

### DON'T: Hardcode Assumptions

❌ **Bad:** `if is_kernel() { use kernel_roster } else { error! }`
✓ **Good:** `let roster: Arc<dyn RosterPort> = ...` (injected at boot)

### DON'T: Mix Consensus Logic Into Handlers

❌ **Bad:** `POST /dogs/{id}/heartbeat` checks quorum directly
✓ **Good:** `POST /dogs/{id}/heartbeat` calls `roster.heartbeat()` (abstracted)

---

## Falsification Plan

**Phase B is falsified if:**
- cynic-node's command loop is hardcoded to poll kernel (defeats transport-agnostic design)
- Soma is described as "must run on kernel" without noting the future distributed variant

**Phase C is falsified if:**
- RosterPort trait is too tightly coupled to ArcSwap or SurrealDB (can't swap implementations)
- Judge updates don't get roster change notifications (sync will drift in Phase D)

**Phase D is falsified if:**
- Askesis consensus protocol is never built (Soma remains kernel-dependent forever)
- GossipRoster is attempted without quorum (consistency breaks under network partition)

---

## Summary: Organic Growth Strategy

| Phase | What | Blocker | Next |
|-------|------|---------|------|
| **B (May)** | cynic-node skeleton, transport-agnostic | None | Design RosterPort |
| **C (May-Jun)** | RosterPort trait, InProcessRoster impl | None | Build Askesis |
| **D (Jul+)** | GossipRoster, distributed topology | **Askesis** | Enable sovereignty |

**The organism doesn't need Askesis to function in Phase C.** It needs it to be *sovereign* in Phase D. Design for it now; build it when it hurts.

**Organism health checks:**
- Phase B: ✓ Transport-agnostic node (ready)
- Phase C: ✓ RosterPort abstraction (ready to design)
- Phase D: ✗ Askesis (blocker, future work)

This is honest design: the centralized kernel works immediately (Phase C), but the path to distributed sovereignty is visible and staged (Phase D via Askesis).
