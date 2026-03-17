<!-- Auto-invocation via ~/.claude/commands/cynic-skills/engineering-stack-design/ — no frontmatter here to avoid duplication -->

# Engineering Stack Design

## Overview

**Inventory before priority. Always.**

The failure mode: jumping to "we'll use hexagonal + SOLID + TDD" without mapping the full decision space. Premature convergence on familiar patterns while missing critical ones.

Three phases, never collapsed:
- **Phase 1 — INVENTORY**: exhaustive enumeration across 11 domains. No judgment. No filtering.
- **Phase 2 — CRYSTALLIZATION**: applicability filter → dependency map → emergent patterns.
- **Phase 3 — BUILD SEQUENCE**: dependency graph → realistic development phases.

Complete Phase 1 fully before starting Phase 2. Complete Phase 2 before deriving Phase 3.

**The output document answers HOW. The design document answers WHAT. They must be orthogonal.**
If your "How" section describes what a component does rather than how to implement it, it belongs in the design doc, not here.

---

## Phase 1 — Inventory

Enumerate ALL potentially relevant items from the 11 domains below.
Do not skip domains. Do not filter while enumerating.

### D1 — Architecture Patterns (structural)
How the system is shaped at the macro level.

| Pattern | Core problem it solves |
|---|---|
| Hexagonal (Ports & Adapters) | External dependencies swappable without touching domain |
| Event-Driven Architecture | Components decoupled via events, not direct calls |
| CQRS | Read and write models separated for clarity and scale |
| Event Sourcing | State derived from append-only event log |
| Pipeline | Data flows through sequential transformation stages |
| Reactive / Actor Model | Async message-passing, backpressure-native |
| Layered / Clean Architecture | Dependency rule: outer layers depend on inner, never reverse |
| Modular Monolith | Single deployable, strong module boundaries |
| Microservices | Independent deployables, network boundary between modules |
| Saga / Choreography | Distributed transactions via event chains |
| Client-Server | Separation of concerns between requester and provider |
| Peer-to-Peer (P2P) | No central authority, nodes are equal |
| Serverless / FaaS | No server management, event-triggered execution |

### D2 — Design Patterns (tactical)
How components interact internally.

| Pattern | Core problem it solves |
|---|---|
| Repository | Domain doesn't know where data lives |
| Factory / Abstract Factory | Object creation decoupled from usage |
| Strategy | Algorithms swappable at runtime |
| Observer / Pub-Sub | Producers don't know their consumers |
| Adapter / Facade | External interface mismatch hidden from domain |
| Decorator | Behavior added without modifying base |
| Circuit Breaker | Fail fast, prevent cascade on remote failures |
| Bulkhead | Isolate failures to one resource pool |
| Outbox | Reliable event publication with local transaction |
| Unit of Work | Group operations into atomic commits |
| Domain Event | State change broadcast within domain boundary |
| Sidecar / Ambassador | Cross-cutting concerns extracted from main process |
| Retry with Exponential Backoff | Transient failures handled without thundering herd |
| Saga (orchestration vs choreography) | Distributed transactions with compensation on failure |

### D3 — Development Methodology (process)
How code is written and validated.

| Method | Core problem it solves |
|---|---|
| TDD | Design before implementation, tests as spec |
| DDD | Model reflects domain language, not tech language |
| BDD | Acceptance criteria readable by domain experts |
| Property-Based Testing | Edge cases found by generators, not by hand |
| Contract Testing | Interface boundaries verified independently |
| SOLID | Maintainability under change |
| YAGNI / DRY | Avoid complexity that isn't needed yet |
| Clean Code | Code reads as prose, not puzzles |
| Trunk-Based Development | Integration friction minimized |
| Mutation Testing | Tests actually catch bugs, not just run green |
| Chaos Engineering | Verify resilience by injecting failures |

### D4 — Engineering Principles (operational)
How the running system behaves under stress and failure.

| Principle | Core problem it solves |
|---|---|
| Observability (logs/metrics/traces) | You can understand what the system is doing |
| Idempotency | Retry-safe operations, no double-effects |
| Resilience (retry, timeout, fallback) | Transient failures don't cascade |
| Backpressure | Fast producers don't overwhelm slow consumers |
| Graceful Degradation | Partial failure → reduced service, not total failure |
| Fault Tolerance | System continues despite component failure |
| Eventual Consistency | Strong consistency sacrificed for availability |
| Immutability | No hidden mutation, safe concurrency |
| Structured Concurrency | Async tasks scoped, never leaked |
| Audit Trail | Every state change traceable and replayable |
| Rate Limiting | Protect services from overload / abuse |
| Health Checks + Heartbeats | Detect dead/degraded nodes automatically |
| Distributed Tracing | Follow requests across service boundaries |

### D5 — Networking & API Patterns
How services communicate over the network.

| Pattern | Core problem it solves |
|---|---|
| REST | Resource-oriented, stateless, cacheable API |
| gRPC / Protobuf | Typed, efficient, streaming-capable RPC |
| GraphQL | Client-driven queries, no over/under-fetching |
| WebSockets | Bidirectional real-time communication |
| API Gateway | Single entry point, cross-cutting concerns (auth, rate limit, routing) |
| Service Discovery | Services find each other without hardcoded addresses |
| Load Balancing | Distribute traffic across instances (round-robin, least-connections, consistent hash) |
| Reverse Proxy | Hide backend topology, terminate TLS, cache |
| DNS Resolution | Name → address mapping, TTL-based caching |
| Consistent Hashing | Distribute data/load with minimal redistribution on node change |
| Gossip Protocol | Decentralized state propagation across nodes |
| Consensus (Raft/Paxos) | Agreement across distributed nodes despite failures |

### D6 — Data Patterns
How data is stored, moved, and queried.

| Pattern | Core problem it solves |
|---|---|
| ACID Transactions | Atomicity, consistency, isolation, durability |
| Database Sharding | Horizontal scaling beyond single-node capacity |
| Data Replication (leader/follower, multi-leader) | Availability + read scaling |
| Database Indexing | Query performance on large datasets |
| Bloom Filters | Probabilistic membership test, avoid unnecessary lookups |
| Change Data Capture (CDC) | React to data changes without polling |
| Message Queues (bounded) | Decouple producers/consumers, buffer bursts |
| Caching Strategies (read-through, write-through, write-behind) | Reduce latency and backend load |
| Cache Eviction (LRU, LFU, TTL) | Bounded memory with intelligent replacement |
| SQL vs NoSQL vs Graph vs Document | Data model fits query patterns |
| Time-Series Storage | Efficient storage/query of timestamped metrics |
| Write-Ahead Log (WAL) | Crash recovery with durability guarantee |
| Batch vs Stream Processing | Throughput vs latency tradeoff for data pipelines |

### D7 — Security Patterns
How the system protects itself and its users.

| Pattern | Core problem it solves |
|---|---|
| Zero Trust | Never trust, always verify — even internal traffic |
| Authentication (mTLS, JWT, API keys) | Prove identity of caller |
| Authorization (RBAC, ABAC, capability-based) | Enforce what caller can do |
| Encryption at rest + in transit | Data protected from unauthorized access |
| Secret Management (vault, env injection) | Credentials never in code or logs |
| Input Validation / Sanitization | Prevent injection (SQL, command, XSS) |
| Principle of Least Privilege | Components get minimum required permissions |
| Audit Logging | Every security-relevant action recorded |
| Dependency Scanning | Known vulnerabilities in third-party code detected |
| Secure Defaults | Safe configuration out of the box, no "root/root" |

### D8 — System Design Tradeoffs
Explicit decisions that must be made — not patterns to adopt, but axes to position on.

| Tradeoff | Options | Decision criteria |
|---|---|---|
| Vertical vs Horizontal Scaling | Scale up (bigger machine) vs scale out (more machines) | Cost, complexity, failure domain |
| Strong vs Eventual Consistency | CAP theorem position | Read-after-write requirements |
| Push vs Pull Architecture | Events pushed to consumers vs consumers poll | Latency vs resource efficiency |
| Stateful vs Stateless Services | State in service vs state in external store | Scalability vs simplicity |
| Synchronous vs Asynchronous | Request-response vs fire-and-forget | Latency requirements vs throughput |
| Monorepo vs Polyrepo | All code in one repo vs separate repos | Team size, deployment coupling |
| Shared DB vs DB-per-service | Services share storage vs own their data | Consistency vs independence |
| Build vs Buy vs Open Source | Custom, SaaS, or OSS | Control, cost, sovereignty |
| Optimistic vs Pessimistic Concurrency | Detect conflicts vs prevent them | Contention level, retry cost |
| Thin vs Thick Client | Logic on server vs client | Offline capability, latency |

### D9 — Language / Runtime Specific
Adapt this domain to the actual stack. **Document the stack first, then enumerate.**

*Example: Python + asyncio*
| Decision | Options | Stakes |
|---|---|---|
| Async model | asyncio TaskGroup vs raw create_task | Leak prevention |
| Type system | Protocol (structural) vs ABC (nominal) | Adapter flexibility |
| Data modeling | dataclass vs pydantic vs attrs | Validation, serialization |
| DI approach | Manual vs framework (inject, dependency-injector) | Complexity vs control |
| Type strictness | mypy strict vs pyright basic | Bug surface vs velocity |
| Logging | structlog vs stdlib logging | Structured output |
| Package mgmt | uv vs pip vs poetry | Reproducibility |

*Example: Rust + tokio*
| Decision | Options | Stakes |
|---|---|---|
| Async runtime | tokio vs async-std vs smol | Ecosystem, performance |
| Trait objects | dyn Trait vs generics vs enum dispatch | Flexibility vs performance |
| Error handling | anyhow vs thiserror vs custom | Ergonomics vs precision |
| Serialization | serde + bincode vs protobuf vs flatbuffers | Performance vs interop |
| Concurrency | Arc<RwLock> vs channels vs actors | Safety vs ergonomics |
| Logging | tracing vs log + env_logger | Structured, span-based vs simple |
| Build | cargo workspace vs single crate | Module isolation vs simplicity |

### D10 — Domain-Specific Patterns
Patterns native to the problem domain. Enumerate the domain's known patterns even if unfamiliar — research during Phase 2.

*Example: Algorithmic trading systems*
| Pattern | Core problem it solves |
|---|---|
| Tick-to-trade latency management | Decision loop fast enough for market conditions |
| Signal generation pipeline | Raw data → conditioned signal → decision |
| Risk circuit breaker | Position limits enforced before execution |
| P&L attribution | Which signal generated which profit/loss |
| Order flow integrity | No duplicate orders, idempotent execution |
| Warm-up protocol | System behavior until calibrated |
| Regime detection | Strategy adapted to market conditions |
| Audit trail (trading) | Regulatory + forensic reconstruction of every decision |

### D11 — Fractal Meta-Principle
**After enumerating D1–D10, identify the meta-principle.**

The meta-principle is the pattern that appears at every scale of the system — in the file, the module, the layer, the architecture, the documentation structure, the development phases. It is not a pattern among others: it is the generator of structure.

Ask: *Which principle, if applied consistently at every scale, would produce most of the patterns I just enumerated?*

If one principle generates 80%+ of the others when applied recursively → it is the meta-principle.
Document it separately. It governs the output document's own structure.

*Example: Separation of Concerns applied at every scale generates: Hexagonal (architecture scale), Pipeline (data scale), Bulkhead (concurrency scale), Structured Concurrency (task scope scale), Zero Trust (security scale), TDD (test/code separation), document separation (design doc vs engineering stack).*

---

## Phase 2 — Crystallization

**REQUIRED SUB-SKILL:** Use `crystallize-truth` for this phase.

Crystallize-truth governs two things here:
1. **Layer 0 (empirical)**: verify technical claims about each pattern before adopting. "Hexagonal adds overhead" — measured or assumed? Apply to every pattern where you have a prior.
2. **Layers 1–3**: apply to tensions between adopted patterns. Every tension is a potential crystallize-truth session.

For each enumerated item, answer:

```
Applicable?     yes / partial / no / defer
Tension with:   [list conflicting items]
Enables:        [list items this unlocks]
Cost of adopt:  [concrete estimate]
Cost of skip:   [concrete risk]
Verdict:        ADOPT / ADAPT / REJECT / DEFER
```

**Filtering rules:**
- **ADOPT**: cost-of-skip > cost-of-adopt, no unresolvable tension
- **ADAPT**: partially applicable — document explicitly what's excluded
- **REJECT**: document reason (prevents future re-opening)
- **DEFER**: cost unknown → define empirical condition that will resolve it

### Emergent Patterns (after filtering)

After the dependency graph is built, look for patterns that emerge from *combinations* of adopted items — patterns that were not in the original D1–D10 inventory but arise from their interaction.

These are often the most architecturally significant insights. They cannot be discovered before Phase 2 because they require knowing which patterns were adopted and how they relate.

```
For each combination of 2+ adopted patterns, ask:
"Does adopting both create a new constraint or capability not present in either alone?"
If yes → document as Emergent Pattern.
```

*Example: adopting Hexagonal + Structured Concurrency + FrequencyBridge simultaneously creates:
"Every task group boundary IS a frequency tier boundary IS a port boundary."
This emergent pattern is not in any inventory — it arises from the three adopted together.*

---

## Dependency Mapping

After filtering, map dependencies between ADOPTED items:

```
Which patterns ENABLE others?     → these come first
Which can be decided in parallel? → these are independent
Which have circular tension?      → these need synthesis
```

The dependency graph IS the build order. Critical path = implementation sequence.

---

## Phase 3 — Build Sequence

Collapse the dependency graph into realistic development phases.

**Rules:**
- Patterns at Level 0 (no dependencies) → Phase 0 (before any domain code)
- Patterns that are expensive to retrofit → promote to earliest phase
- Phases must be observable: each phase ends with a demonstrable behavior, not just working code
- A phase that produces no observable output is not a phase — merge it with the next

Ask for each phase:
1. What can you observe at the end of this phase that you couldn't before?
2. What does the system do if this phase is skipped and you try to build the next?
3. Is this phase genuinely independent of the next, or are they coupled?

*Realistic means: account for the fact that retrofitting costs 3–5× implementing correctly from the start. Level 0 patterns (hexagonal ports, concurrency model, test pipeline) pay off across all future phases — invest in them first.*

---

## Output — Engineering Stack Document

**Structure of the output document:**

```
1. Meta-principle (fractal — governs the document's own structure)
2. Dependency graph → development phases
3. Emergent patterns
4. Per-cluster implementation guides (WHAT → reference design doc; HOW → this document)
5. Rejected / Deferred
```

One entry per ADOPTED pattern:

```
### [Pattern Name]
Design doc ref: [§X — what this component IS]
Why:       [one sentence — what failure does this prevent?]
How:       [concrete implementation — file locations, class names, interfaces, test strategy]
Phase:     [P0 / P1 / P2 / P3 / P4 — when this is built]
Scope:     [what is explicitly NOT included]
Tension:   [known conflicts and resolution]
```

**The "How" field must answer: given the design doc's description of this component, how do I write the code?**
- File locations and module names
- Interface signatures
- Wiring points (where adapters are constructed)
- Test strategy specific to this pattern
- Rules that prevent known violations

If the "How" field describes what the component does rather than how to implement it → move it to the design doc.

One section for REJECTED / DEFERRED:
```
### Rejected
[Pattern]: [reason]. Revisit if [condition].

### Deferred
[Pattern]: unknown cost. Resolve when [observable condition].
```

---

## Pattern Deep Dives — Implementation Knowledge

These are the patterns most commonly adopted AND most commonly misimplemented. When you ADOPT one from Phase 2, apply its deep implementation rules — not just its name.

---

### Hexagonal Architecture (Ports & Adapters)

**The dependency rule (inviolable):**
```
Adapters ──depend on──▶ Ports (traits/interfaces) ──live in──▶ Domain Core
Domain Core depends on NOTHING external. No HTTP, no DB, no filesystem, no framework.
```

**Three zones — know what goes where:**

| Zone | Contains | Depends on | Example |
|---|---|---|---|
| **Domain Core** | Business logic, value objects, port traits, domain events, pure functions | Nothing external — only stdlib types | `InferenceRequest`, `BackendStatus`, `route()` logic |
| **Driving Adapters** (inbound) | Translators from external input → domain calls | Domain Core (calls ports) | gRPC service, REST controller, CLI handler |
| **Driven Adapters** (outbound) | Implementations of port traits that talk to external systems | Domain Core (implements ports) | `LlamaCppBackend`, `SurrealDbAdapter`, `SysfsDetector` |

**Driving vs Driven — the critical distinction:**
- **Driving** = something CALLS your domain (user, API client, cron job). The adapter translates external protocol → domain method call.
- **Driven** = your domain NEEDS something external (database, HTTP API, filesystem). The adapter implements a domain-defined trait.
- **Test:** If you remove the adapter, does domain code still compile? If yes → correctly separated. If no → dependency leak.

**Port design rules:**
- A port is a **trait/interface defined in the domain** that describes what the domain NEEDS, in domain language
- Ports use domain types, never adapter types (`InferenceRequest`, not `HttpRequest`)
- One port per capability axis (Interface Segregation): `StoragePort` and `EventBus` are separate, even if both backed by the same DB
- Port methods are async only when the domain semantically requires async (I/O). Pure domain logic is sync.
- **Fat port smell:** If a port has >7 methods, it's probably multiple ports merged. Split by consumer.

**Composition root (the wiring point):**
- ONE file (typically `main.rs`, `main.py`, `app.ts`) that:
  1. Creates concrete adapter instances
  2. Injects them into domain services via constructor
  3. Is THE ONLY FILE that imports concrete adapter types
- Every other file imports only traits/interfaces
- **Test:** `grep` for concrete adapter type names. They should appear ONLY in the composition root and their own files.

**Port contract testing (how hexagonal enables testing):**
```
Define test suite for each port:
  inference_port_contract(port: &dyn InferencePort) {
    // These tests run against MockBackend AND LlamaCppBackend
    assert capability is reported
    assert health returns valid state
    assert infer returns result or well-formed error
  }
```
- Mock adapters are NOT test scaffolding — they are first-class implementations that MUST pass the same contract tests
- If MockBackend passes but RealBackend fails, the contract test found a real bug

**Common violations (detect and fix):**
| Violation | Symptom | Fix |
|---|---|---|
| Domain imports adapter | `use reqwest::Client` in domain file | Define a port trait. Move HTTP call to adapter. |
| Adapter types in domain | `fn process(req: HttpRequest)` | Map to domain types at adapter boundary |
| `#[cfg]` in domain | `#[cfg(target_os = "linux")]` in business logic | Platform adapters behind a trait, selected in composition root |
| God composition root | main.rs has business logic | main.rs only wires. Logic goes to domain. |
| Missing port | Domain directly calls `fs::read()` or `env::var()` | Wrap in a port if it varies by environment. If truly cross-platform and pure, a function is fine. |
| Shared adapter | One adapter implements 3 ports | Each port gets its own adapter. Share internal client if needed. |

**When NOT to use a port (avoid over-engineering):**
- Cross-platform stdlib operations (`Path::new()`, `env::var()`) — no adapter needed
- Pure functions with no I/O — just call them
- Cold paths that change once per year — a function is fine
- **Rule of thumb from T6:** Hexagonal rigor proportional to rate-of-change. Hot paths (inference) get full ports. Cold paths (config persistence) get good functions.

---

### Circuit Breaker

**State machine (3 states, 4 transitions):**
```
         ┌──────────────────────────────────────────┐
         │                                          │
         ▼                                          │
      CLOSED ──(N consecutive failures)──▶ OPEN     │
      (normal)                            (reject)  │
         ▲                                  │       │
         │                                  │       │
         │                            (cooldown     │
         │                             expires)     │
         │                                  │       │
         │                                  ▼       │
         └────(probe succeeds)──── HALF-OPEN        │
                                  (one probe)       │
                                      │             │
                                      └─(probe fails)┘
```

**Per-resource, not global:** Each backend/service/connection gets its own circuit breaker instance. A failed DB doesn't open the circuit on inference.

**Parameters:**
| Parameter | Typical default | Meaning |
|---|---|---|
| `failure_threshold` | 3-5 | Consecutive failures before opening |
| `cooldown_ms` | 10_000-60_000 | Time in OPEN before probing |
| `probe_timeout_ms` | 5_000 | Timeout for the HALF-OPEN probe request |

**Implementation pattern:**
```
fn call(request):
  match self.state:
    CLOSED:
      result = try_call(request)
      if failure: self.failures += 1
        if self.failures >= threshold: transition(OPEN)
      else: self.failures = 0
      return result
    OPEN:
      if now() - self.opened_at > cooldown:
        transition(HALF_OPEN)
        return call(request)  // retry as probe
      else:
        return Err(CircuitOpen)
    HALF_OPEN:
      result = try_call(request)
      if success: transition(CLOSED)
      else: transition(OPEN)
      return result
```

**Replaces:** per-request health checks. Instead of checking health on EVERY request (O(N×M)), circuit breaker tracks state automatically.

---

### Event-Driven / Pub-Sub

**Bounded channels (backpressure rule):**
- NEVER use unbounded channels in production. They are memory leaks waiting for a burst.
- Bounded channel size = expected burst × processing time. Start small (64-256), measure, adjust.

**Fanout pattern:**
- One publisher, N subscribers each get their OWN copy of the event
- Subscriber failure doesn't affect other subscribers
- Slow subscriber gets backpressure (bounded channel fills), not dropped messages

**Event design:**
- Events are PAST TENSE facts: `BackendConnected`, `InferenceCompleted`, `HealthChanged`
- Events are immutable value objects with timestamp
- Events carry enough data to be self-contained (subscriber shouldn't need to query back)

---

### Service Discovery

**Static vs Dynamic:**
| Approach | When | How |
|---|---|---|
| **Static config** | Known topology, few nodes | Env vars, config file |
| **DNS-based** | Cloud/K8s environments | SRV records, headless services |
| **Registry-based** | Dynamic scaling | Consul, etcd, ZooKeeper |
| **Probe-based** | Edge/local network | Scan known ports, mDNS |

**For small clusters:** Probe-based discovery (scan known ports on known hosts) is simple and sufficient. No central registry needed.

---

### Observability (Logs / Metrics / Traces)

**The three pillars — each serves a different question:**
| Pillar | Answers | Format |
|---|---|---|
| **Logs** | "What happened?" | Structured JSON, not strings |
| **Metrics** | "How much / how fast / how often?" | Counters, gauges, histograms |
| **Traces** | "Where did the request go?" | Span trees with timing |

**Structured logging rule:** Every log entry is a machine-parseable record. Never `println!("error: {}", msg)`. Always `tracing::error!(backend_id = %id, latency_ms = latency, "inference failed")`.

**Metric types:**
- **Counter** — monotonically increasing (total requests, total errors)
- **Gauge** — goes up and down (active connections, memory usage)
- **Histogram** — distribution (latency percentiles p50/p95/p99)

---

### Graceful Degradation + Health States

**3-state health (never boolean):**
| State | Meaning | Action |
|---|---|---|
| **HEALTHY** | All dependencies reachable, all features available | Normal operation |
| **DEGRADED** | Some dependencies down, core features work | Report what's missing, continue with reduced capability |
| **CRITICAL** | Core dependencies down, cannot serve primary function | Refuse work, report loudly, keep probing for recovery |

**Boot with whatever is available.** Don't panic on missing optional dependencies. Start in DEGRADED, report what's missing, recover automatically when it comes back.

---

## Common Failures

| Failure | Symptom | Fix |
|---|---|---|
| Phase collapse | Filtering while enumerating | Stop. Complete inventory first. |
| Domain skip | "D10 doesn't apply here" | It always applies. Enumerate then reject. |
| D11 skip | Meta-principle not identified | Ask: what generates most of the other patterns? |
| Premature convergence | First familiar pattern adopted without inventory | Inventory is not optional |
| Missing tensions | Two conflicting patterns both adopted | Explicitly resolve or reject one |
| No dependency map | Stack is a list, not a sequence | Build the graph before the document |
| Missing emergent patterns | Only patterns from inventory documented | Look for combinations that create new constraints |
| Vague "how" | "Use Repository pattern" | "how" must name files, classes, interfaces |
| **Document is design doc copy** | "How" describes what components do | Move to design doc. Describe implementation only. |
| No build phases | Stack is complete but unbuildable | Phase 3 is not optional — derive from dependency graph |
| Phases not observable | "Phase 2 — set up internals" | Each phase must produce a demonstrable behavior |
| **Hexagonal in name only** | "We use ports" but domain imports `reqwest` | Run the test: does domain compile without adapters? |
| **Circuit breaker missing** | Health checked on every request, or not at all | Add per-resource circuit breaker with 3-state machine |
| **Unbounded channels** | Memory grows under load, eventually OOM | Replace with bounded channels + backpressure strategy |
| **Boolean health** | `is_healthy: bool` misses degraded state | 3-state enum: HEALTHY / DEGRADED / CRITICAL |