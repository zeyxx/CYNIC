# Overnight Research: Agent Architecture Patterns
**Date:** 2026-03-21
**Scope:** 8 repos analyzed for patterns applicable to CYNIC multi-agent coordination

---

## 1. paperclipai/paperclip (30K stars)
**Stack:** Node.js + PostgreSQL
**Concept:** "Org chart for AI agents" — control plane for zero-human companies

### Coordination Patterns
Paperclip's core innovation is treating the org chart as the coordination primitive. Agents have a **reporting line**, and cross-team work follows an explicit escalation protocol:

1. When an agent receives a task outside its competence, it cannot cancel it — it must **reassign to its manager** with an explanation.
2. Every cross-team task tracks a **delegation depth integer** (how many hops from the original requester).
3. **Billing codes** propagate through delegation chains — Agent A's request to Agent B records all of B's token spend against A's billing code. Attribution is structural, not inferred.

The "task checkout" is **atomic** at the database level — no double-work, no race conditions on task claiming. This is listed explicitly as a solved problem.

### Memory / State Management
- Task context persists across heartbeats — agents resume the same task context rather than restarting cold.
- Two context delivery modes: **fat payload** (Paperclip bundles full context into heartbeat invocation, for stateless agents) vs **thin ping** (wake-up only, agent fetches what it needs via API, for sophisticated agents managing their own state).
- Company configs are **fully exportable** with two modes: template (structure only) and snapshot (full state including current tasks and progress).

### Task Decomposition
Hierarchical: Company Initiatives → Projects → Milestones → Tasks → Subtasks. CEO proposes strategic breakdown; Board approves before execution begins. Each task carries **full goal ancestry** so any agent can see the "why" not just the "what."

### Consensus / Governance
No voting — governance is **hierarchical approval gates**:
- Board approves: new agent hires, CEO's initial strategic breakdown
- Board can pause/resume any agent, override any decision, override any budget at any level
- Config changes are versioned; bad changes can be rolled back

### Rust Patterns for CYNIC
Not Rust, but the **atomic checkout + billing code propagation** pattern is directly applicable to CYNIC's coord system. The TOCTOU fix in CYNIC's `/coord/claim` endpoint solves exactly the same problem as Paperclip's atomic task checkout.

**Key steal:** The two-mode context delivery (fat vs thin) maps precisely to CYNIC's `/judge` endpoint — lightweight callers can get the full verdict payload, sophisticated callers (MCP clients) can fetch incrementally. This is already implicit in CYNIC's design but worth making explicit as a first-class contract.

---

## 2. NousResearch/hermes-agent (9.5K stars)
**Stack:** Python 3.11+, multiple backends
**Concept:** Self-improving agent with a closed learning loop

### Coordination Patterns
Hermes spawns **isolated subagents** for parallel workstreams. The key coordination primitive is the **learning artifact**: after completing a complex task, the agent autonomously generates a skill (SKILL.md) from the experience. These skills are then available to future runs.

The subagent model is isolation-first: each subagent gets its own context, executes, returns results + learnings. No shared mutable state between subagents.

ACP (Agent Client Protocol) is supported — Hermes can run as a coding agent inside IDEs (VS Code, Zed, JetBrains), receiving tasks from external orchestrators. This makes it a composable worker, not just a standalone agent.

### Memory / State Management
Three-tier memory architecture:
1. **Episodic memory** — FTS5 (full-text search) on session history with LLM summarization for cross-session recall
2. **Semantic memory (user model)** — Honcho dialectic user modeling builds a deepening model of who the user is across sessions
3. **Procedural memory (skills)** — agent-curated SKILL.md files, compatible with the agentskills.io open standard

Memory is provider-agnostic: `~/.hermes/.env` stores facts independently of any LLM provider.

**Critical pattern:** "Periodic nudges" — the agent nudges itself to persist knowledge. This is an internal feedback loop that prevents memory from going stale. It is not a human trigger — the agent decides when to consolidate.

### Task Decomposition
Python RPC scripts that call tools, collapsing multi-step pipelines into zero-context-cost turns. The skill system creates a protocol buffer for task knowledge — complex procedures become reusable primitives that don't consume context window.

### Consensus / Voting
No consensus mechanism. Single-agent with learning. The closest parallel: the FTS5 session search with LLM summarization functions as a form of "consensus across time" — the agent reconciles its own past reasoning before acting.

### Rust Patterns for CYNIC
The **FTS5 session search + LLM summarization** pattern is directly applicable to CYNIC's crystal system. CYNIC already has the crystal endpoint (`/crystals`) but the recall mechanism is purely recency-based. Hermes's approach — full-text index on past verdicts with LLM-guided retrieval — would let Dogs look up how similar claims were judged in the past before scoring, improving consistency.

**Key steal:** Skill self-improvement during use. CYNIC's Dogs produce verdicts but don't learn from them. A feedback path from verdict outcomes back into Dog scoring weights would create the same closed loop.

---

## 3. RightNow-AI/openfang (15K stars)
**Stack:** Rust, 14 crates, 137K LOC, 1767+ tests
**Concept:** Agent OS — single binary, pre-built autonomous "Hands"

### Coordination Patterns
The event bus is the coordination backbone:

```rust
// openfang-kernel/src/event_bus.rs
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    agent_channels: DashMap<AgentId, broadcast::Sender<Event>>,
    history: Arc<RwLock<VecDeque<Event>>>,  // 1000-event ring buffer
}
```

Events route to:
- `EventTarget::Agent(id)` — direct
- `EventTarget::Broadcast` — all agents
- `EventTarget::Pattern(pattern)` — regex/glob routing (phase 1 broadcasts to all; subscribers filter)
- `EventTarget::System` — kernel-internal

This is a clean separation: the bus doesn't know about agent semantics, just routing topology.

The **A2A (Agent-to-Agent) protocol** enables cross-instance coordination: `AgentCard` discovery, task delegation with lifecycle tracking (`a2a_task_store`), and status polling. External A2A agents are tracked in `a2a_external_agents: Mutex<Vec<(String, AgentCard)>>`.

### Memory / State Management
`MemorySubstrate` abstraction over SQLite + vector embeddings. The `embedding_driver` is `Option<Arc<dyn EmbeddingDriver>>` — graceful fallback to text search when no embedding model is available. This is the correct approach for CYNIC's sovereign-first philosophy.

The **taint tracking system** is architecturally notable:

```rust
pub enum TaintLabel {
    ExternalNetwork,
    UserInput,
    Pii,
    Secret,
    UntrustedAgent,  // ← critical for multi-agent trust
}

pub struct TaintedValue {
    pub value: String,
    pub labels: HashSet<TaintLabel>,
    pub source: String,
}
```

Values carry provenance labels that propagate through computation. `TaintSink` declares which labels it blocks. This prevents UntrustedAgent outputs from flowing into privileged operations without explicit declassification.

**Security depth:** 16 independent security systems. Relevant to CYNIC:
- WASM dual-metered sandbox (fuel metering + epoch interruption) — CYNIC's WASM gate idea
- Merkle hash-chain audit trail — every action cryptographically linked to previous
- Loop guard with SHA256 tool-call deduplication and circuit breaker
- GCRA (Generic Cell Rate Algorithm) rate limiter — cost-aware, per-IP

### Task Decomposition
**Hands** are the decomposition primitive — pre-built 500+ word expert playbooks bundled with tools, settings, and domain expertise SKILL.md files. Each Hand is a multi-phase operational procedure, not a single prompt. Example: Researcher Hand has CRAAP credibility evaluation (Currency, Relevance, Authority, Accuracy, Purpose).

The WorkflowEngine supports DAG execution with step agents — each step can specify which agent handles it. This is more expressive than CYNIC's current single-Dog-per-verdict model.

### Consensus / Voting
No explicit voting. The `ModelRouter` handles intelligent routing with task complexity scoring and automatic fallback. This is routing intelligence, not consensus.

The `CapabilityManager` + RBAC system enforces what each agent can access — closer to capability-based security than consensus.

### Rust Patterns for CYNIC

**Direct steals:**

1. **EventBus architecture** — CYNIC's coord system currently uses REST polling. A broadcast channel with `DashMap<AgentId, broadcast::Sender<Event>>` and a 1000-event ring buffer would eliminate polling entirely and give Dogs real-time event streams.

2. **TaintedValue propagation** — CYNIC judges claims from external sources. Tagging claim content as `ExternalNetwork` and Dog responses as `UntrustedAgent`, then requiring explicit declassification before crystallization, would make the trust boundary machine-enforced rather than convention-based.

3. **Merkle hash-chain audit trail** — CYNIC's verdict log is append-only in SurrealDB but not cryptographically linked. Adding a hash-chain would make the audit trail tamper-evident, which matters for the epistemic immune system's credibility.

4. **GCRA rate limiter** — CYNIC's current rate limiter is a sliding window counter. GCRA is more precise and handles burst traffic more gracefully. The `openfang_runtime::gcra` implementation (inferred from README) is worth porting.

---

## 4. nullclaw/nullclaw (6.6K stars)
**Stack:** Zig, 678KB binary, <2ms startup, 5640+ tests
**Concept:** Zero-dependency autonomous AI assistant infrastructure

### Coordination Patterns
The **event bus** architecture in Zig:

```zig
// src/bus.zig
pub const InboundMessage = struct {
    channel: []const u8,   // "telegram", "discord", "webhook", "system"
    sender_id: []const u8,
    chat_id: []const u8,
    content: []const u8,
    session_key: []const u8,  // "channel:chatID" for session lookup
    media: []const []const u8,
    metadata_json: ?[]const u8,
};
```

Two blocking queues on a ring buffer with `Mutex+Condition`: inbound (channels → agent) and outbound (agent → channels). The `session_key` is a composite `"channel:chatID"` — this is the coordination primitive for multi-channel session continuity.

The architecture is **vtable-driven**:

```
src/providers/root.zig  (Provider vtable)
src/channels/root.zig   (Channel vtable)
src/tools/root.zig      (Tool vtable)
src/memory/root.zig     (Memory vtable)
src/observability.zig   (Observer vtable)
src/runtime.zig         (RuntimeAdapter vtable)
src/peripherals.zig     (Peripheral vtable)
```

Every extension point is an explicit `ptr: *anyopaque` + `vtable: *const VTable`. Callers must own the implementing struct — the protocol enforces this at the type level.

### Memory / State Management
SQLite + markdown backends with FTS5 vector search. The `filterToolSpecsForTurn` function implements dynamic tool loading based on keyword matching in the user message:

```zig
// Three modes:
.always   // tool always loaded (regardless of message content)
.dynamic  // tool loaded only when message contains keywords
.never    // tool excluded
```

This is context-aware tool injection: the agent doesn't carry a fixed tool set but dynamically assembles the right tools per turn based on message content. CYNIC's axiom routing does something similar (routing claims to relevant Dogs) but at the claim level, not the tool level.

### Task Decomposition
The agent loop has `DEFAULT_MAX_TOOL_ITERATIONS: u32 = 25` and `DEFAULT_MAX_HISTORY: u32 = 50`. The `compaction.zig` module handles history trimming before hitting the limit.

The `QueueMode` enum reveals sophisticated concurrent request handling:
```zig
const QueueMode = enum { off, serial, latest, debounce };
const QueueDrop = enum { summarize, oldest, newest };
```

When requests queue (busy agent), the system can summarize dropped messages rather than silently discarding them. This is a graceful degradation pattern — information isn't lost, just compressed.

### Consensus / Voting
No voting. The `SecurityPolicy.autonomy` field has modes: `full`, `supervised`, `allowlist`, `deny`. The `ExecSecurity` level determines whether the agent can self-authorize tool use.

### Rust Patterns for CYNIC

**Direct steals:**

1. **Dynamic tool injection via keyword matching** — CYNIC could apply this to axiom routing: if a claim mentions financial data, inject VERIFY and BURN Dogs; if it mentions a creative work, weight FIDELITY and CULTURE. This is currently static.

2. **QueueMode.debounce + QueueDrop.summarize** — When multiple judge requests arrive simultaneously, CYNIC's current model creates separate verdicts. A debounce window that summarizes concurrent requests into a single verdict batch would reduce Dog load significantly during traffic spikes.

3. **Vtable-driven extension architecture** — CYNIC's current Dog trait is a good start. The nullclaw pattern makes the extension contract maximally explicit: `ptr: *anyopaque` with a const vtable, no virtual dispatch hidden in Rust trait objects. The `observability.zig` Observer vtable is worth examining — it's how nullclaw instruments the agent loop without coupling metrics to business logic.

---

## 5. code-yeongyu/oh-my-openagent (42K stars)
**Stack:** TypeScript plugin for OpenCode
**Concept:** Multi-model orchestration harness with specialized agents

### Coordination Patterns
Three-layer orchestration with clear role separation:

```
Planning Layer:   Prometheus (planner) + Metis (gap analyzer) + Momus (reviewer)
Execution Layer:  Atlas (conductor/orchestrator)
Worker Layer:     Sisyphus-Junior + Oracle + Explore + Librarian + Frontend
```

**Prometheus interview protocol:** Before any plan is written, Prometheus interviews the user until four criteria are met: core objective defined, scope boundaries established, no critical ambiguities, technical approach decided, test strategy confirmed. This is a formal pre-commitment protocol — the agent refuses to plan until requirements are clear.

**Momus validation criteria (for high-accuracy mode):**
- 100% of file references verified
- ≥80% of tasks have clear reference sources
- ≥90% of tasks have concrete acceptance criteria
- Zero tasks require assumptions about business logic
- No maximum retry limit — Momus loops until approved

**Atlas conductor pattern:** Atlas reads the plan, delegates tasks to worker agents via category routing, accumulates "wisdom" (learnings) from each completed task, and uses that wisdom to inform subsequent delegations. It does not execute — it coordinates. "Doesn't play instruments, ensures perfect harmony."

### Memory / State Management
`boulder.json` tracks execution state across sessions:
```
{
  active_plan: path to current plan,
  session_ids: [all sessions that worked on this plan],
  started_at: timestamp,
  plan_name: human-readable identifier
}
```

When `/start-work` is called in a new session, it reads `boulder.json` and resumes exactly where the previous session left off. Sessions are implementation details; the plan execution is the persistent unit.

Hierarchical `AGENTS.md` files per directory give agents scoped context without bloating the context window. `/init-deep` generates these automatically.

### Task Decomposition
Category-based routing (not model-based):
```
visual-engineering  → UI/UX/design work
deep                → autonomous research + execution
quick               → single-file changes, typos
ultrabrain          → hard logic, architecture decisions
```

The category maps to a model at configuration time. Agents declare "what kind of work" not "which model." This is a stable interface — model selection is implementation detail.

The **hash-anchored edit tool (Hashline)** solves the harness problem: every line the agent reads comes back tagged `11#VK| function hello()`. Edits reference the tag. If the file changed since the last read, the hash won't match and the edit is rejected. Success rate improvement: 6.7% → 68.3% (Grok Code Fast 1).

### Consensus / Voting
The Momus loop is the closest analog to voting — a specialized reviewer independently evaluates the plan and can veto it (REJECTED) forcing revision. It is adversarial review, not averaging.

**Metis as gap analyst:** Metis specifically looks for "hidden intentions," "AI-slop patterns (over-engineering, scope creep)," and "missing acceptance criteria" — it functions as a negative space analyst, finding what Prometheus didn't say.

### Rust Patterns for CYNIC

**Direct steals:**

1. **Adversarial pre-commit review (Momus pattern)** — CYNIC's crystallization threshold (`φ⁻¹ = 0.618`) is the quantitative gate. The Momus pattern suggests a **qualitative gate**: a specialized reviewer that checks for axiom coverage gaps before crystallization. A "CrystalReviewer" Dog that only activates when confidence > 0.5 and looks for: Is VERIFY satisfied? Is SOVEREIGNTY not bypassed? Are all axioms covered?

2. **Category routing before model selection** — CYNIC's Dog routing is currently by model identity (`sovereign`, `gemini`, `huggingface`). Switching to category routing (`deep-reasoning`, `fast-heuristic`, `domain-expert`) with model mapping at config time would decouple Dog identities from infrastructure. A new GPU doesn't require changing routing logic.

3. **Prometheus interview protocol** — CYNIC's `/judge` endpoint currently accepts claims without a pre-commitment protocol. For high-stakes judgments (crystallization-eligible), requiring a structured intake form (claim type, domain, source quality, verification method) would reduce ambiguity before Dogs score.

4. **boulder.json session continuity** — CYNIC's session state (coord claim/release) is currently in-memory. Persisting a lightweight session state to disk would allow agent sessions to survive CYNIC kernel restarts without losing coordination context.

---

## 6. dimensionalOS/dimos (1.8K stars)
**Stack:** Python, LangGraph, LCM (Lightweight Communications and Marshalling)
**Concept:** Agentic OS for physical robots — quadrupeds, humanoids, drones

### Coordination Patterns
**Module system with typed streams:**

```python
class RobotConnection(Module):
    cmd_vel: In[Twist]       # ← subscribes to velocity commands
    color_image: Out[Image]  # ← publishes camera frames
```

Modules connect via `autoconnect()` which matches streams by `(name, type)`. When names conflict, transports can be overridden:

```python
blueprint.transports({("color_image", Image): LCMTransport("/color_image", Image)})
```

This is a **reactive dataflow graph** with explicit type-safe edges. Agents subscribe to perception streams and publish motor commands — no shared state, no explicit locking.

The skill discovery protocol: at startup, the agent uses RPC to discover all `@skill`-annotated methods across all deployed modules. Skills become LangChain tools automatically. This is **capability advertisement**, not static registration.

### Memory / State Management
**Spatio-temporal RAG (Spatial Memory):** The system maintains a dynamic map of object locations with temporal decay — "the cup was last seen at (x, y) 3 minutes ago." This is context-aware retrieval where the context is physical space and time.

The `spatial_memory` stream is published by the navigation module and subscribed to by the agent. The agent queries it via the `where_am_i()` and `navigate_with_text(query)` skills — natural language over spatial memory.

### Task Decomposition
LangGraph state machine: agent receives human input, decides whether to call skills or respond directly, executes skills via RPC, receives results, loops. Skills are RPC calls that may take time (physically moving the robot) — the agent waits for completion before proceeding.

### Consensus / Voting
No consensus. Single agent per robot. Multi-robot coordination would be inter-module messaging.

### Rust Patterns for CYNIC

The dimos architecture is less directly applicable (Python, robotics domain) but has two transferable ideas:

1. **Capability advertisement via RPC discovery** — CYNIC's Dogs are statically registered. A dynamic discovery protocol where Dogs advertise their capabilities (`axioms_scored: [FIDELITY, VERIFY]`, `domains: [chess, code, science]`, `max_latency_ms: 3000`) would let the kernel intelligently route claims to the most capable Dog rather than broadcasting to all.

2. **Typed stream edges (autoconnect pattern)** — CYNIC's internal data flow (claim → Dog → verdict → aggregator → crystal) is currently implicit. Making it an explicit typed dataflow graph (analogous to dimos's `In[T]`/`Out[T]` module system) would make the pipeline inspectable, testable in isolation, and replaceable one stage at a time.

---

## 7. Thinklanceai/agentkeeper (115 stars)
**Stack:** Python, SQLite
**Concept:** Cognitive persistence layer — cross-provider memory continuity

### Coordination Patterns
Single-agent focus. The Cognitive Reconstruction Engine (CRE) sits between the agent and any LLM provider as a middleware layer. No multi-agent coordination.

### Memory / State Management
The core architecture is notable despite the small scale:

```
Agent
  ↓
AgentKeeper (CRE)     ← provider-agnostic cognitive layer
  ↓       ↓       ↓
OpenAI  Claude  Gemini  Ollama
```

**Critical fact prioritization:** when the token budget is constrained, critical facts are preserved first. The `remember(content, critical=True)` API makes priority explicit at ingestion time rather than inferring it later.

**Cross-model recovery benchmark:** 95% critical fact recovery (19/20) when migrating GPT-4 → Claude with a 2000-token budget for 100 stored facts (20 critical). The SQLite persistence is provider-agnostic — facts survive complete provider switches.

The planned **semantic memory (embeddings)** and **multi-agent memory sharing** on the roadmap indicate the architectural gap this fills: no existing framework handles the provider-switch case for stateful agents.

### Rust Patterns for CYNIC

**Direct steal:** CYNIC's VerdictCache uses recency-based eviction. The `critical=True` flag pattern suggests a **priority tier in the cache**: verdicts that contributed to crystals should be evicted last, regardless of recency. High-value verdicts (those that crossed the crystallization threshold) deserve a protected tier.

This maps to a concrete change: `VerdictCache` gains a `protect(verdict_id)` method called during crystallization. Protected verdicts are evicted only when the cache is at absolute capacity and only in LRU order within the protected tier.

---

## 8. calesthio/Crucix (5.7K stars)
**Stack:** Node.js, ESM, single Express dependency
**Concept:** Personal intelligence terminal — 27 OSINT sources, parallel sweep, LLM synthesis

### Coordination Patterns
**Parallel execution with `Promise.allSettled()`** — all 27 sources query simultaneously. Failed sources return structured errors; the sweep continues. No source is a blocking dependency.

**Delta engine** — cross-sweep change detection with configurable thresholds and semantic deduplication. Every 15 minutes:
1. Query all sources in parallel
2. Compute delta from previous run (new signals, escalations, de-escalations with severity)
3. Evaluate alerts via LLM or rule-based fallback
4. Push via SSE to browsers

The alert tier system (FLASH / PRIORITY / ROUTINE) with semantic dedup is an implicit consensus mechanism: multiple sources flagging the same event converge to a single higher-priority alert rather than duplicate notifications.

### Memory / State Management
Two-tier delta memory:
- **Hot memory:** last 3 runs, atomic writes (`hot.json`)
- **Cold storage:** daily archives (`cold/YYYY-MM-DD.json`)

Atomic writes prevent partial reads. The separation between hot (operational) and cold (archival) memory mirrors database WAL patterns.

The `briefing.mjs` master orchestrator follows a strict source isolation pattern: each source module is standalone (`node apis/sources/gdelt.mjs` runs independently). This makes individual sources testable and replaceable without touching the orchestrator.

### Rust Patterns for CYNIC

**Direct steals:**

1. **Multi-tier alert system with semantic dedup** — CYNIC's Dog voting currently produces a single weighted score. A tiered output (HOWL → high confidence rapid response, WAG → standard verdict, GROWL/BARK → flag for human review) is already in CYNIC's spec. The Crucix pattern shows how to implement the tier promotion via cross-source convergence: if multiple Dogs independently flag the same claim at high severity, the verdict tier elevates.

2. **Hot/cold memory separation with atomic writes** — CYNIC's SurrealDB stores all verdicts uniformly. A hot tier (last N verdicts, in-memory or fast SQLite) separate from cold storage would improve VerdictCache performance. Writes to hot are atomic; cold archival is asynchronous.

3. **Source isolation (standalone testability)** — CYNIC's Dogs are not currently testable in isolation without the full kernel. The Crucix pattern of making each source a standalone module that exports a single `briefing()` function maps to making each Dog a standalone binary (or at least a standalone integration test) that can be invoked with a single claim payload.

---

## Synthesis: Patterns for CYNIC Multi-Agent

### Pattern 1 — Atomic Coordination with Attribution
**From:** Paperclip (atomic checkout + billing codes)
**For CYNIC:** The coord claim/release system already solves the TOCTOU problem. What's missing is attribution propagation: when a Dog is dispatched to judge a claim, the dispatch event should carry the coord session ID so that Dog responses are linked to the originating session. Currently, Dog responses are aggregated but the attribution chain (which session triggered which Dog run) is not preserved in the verdict.

### Pattern 2 — Closed Learning Loop via Verdict Feedback
**From:** Hermes (autonomous skill generation after complex tasks)
**For CYNIC:** Dogs produce verdicts but don't learn. The minimal viable loop: after a verdict is crystallized, compute a feedback signal (did the crystal survive? was it later contradicted?) and update Dog calibration weights. This requires a crystal lifecycle event (`crystal_confirmed`, `crystal_contradicted`) that triggers recalibration. Even without a training loop, storing verdict→outcome pairs creates the dataset for future calibration.

### Pattern 3 — Information Flow Taint Tracking
**From:** OpenFang (`TaintedValue` with `TaintLabel.UntrustedAgent`)
**For CYNIC:** Claims enter CYNIC from external sources. Dog responses are generated by models. Neither is trusted by default. A `TaintLabel` system where:
- Incoming claims carry `ExternalNetwork` taint
- Dog responses carry `UntrustedAgent` taint
- Crystal content requires explicit declassification (human review or quorum)

This would make CYNIC's trust boundary machine-enforced. The `VerdictCache` would carry taint labels; the crystallization path would enforce a declassification gate.

### Pattern 4 — Adversarial Pre-Crystallization Review
**From:** oh-my-openagent (Momus reviewer with REJECTED/OKAY loop)
**For CYNIC:** A "CrystalAudit" pass before writing a crystal. Triggered when confidence crosses the crystallization threshold (`φ⁻¹ = 0.618`). The audit checks:
- Is at least one Dog's VERIFY score above threshold?
- Is SOVEREIGNTY score not below GROWL?
- Are at least N of the 6 axioms covered by Dogs that actually scored them (not just defaulted)?
- Is the claim source taint declassified?

If any check fails: verdict is flagged, not crystallized, returned to judge queue with a higher-priority rerun request.

### Pattern 5 — Dynamic Tool Injection via Claim Classification
**From:** nullclaw (keyword-based dynamic tool loading per turn)
**For CYNIC:** Dog selection is currently static (all Dogs are dispatched for all claims, with fast-path exemptions). A claim classifier that maps claim domain to Dog subset would reduce load:
- Financial claim → BURN + VERIFY Dogs prioritized
- Creative/philosophical claim → CULTURE + FIDELITY Dogs prioritized
- Technical/code claim → VERIFY + PHI Dogs prioritized
- All claims → SOVEREIGNTY always included (it's the geometric mean axiom)

This is not pre-existing routing — it's **dynamic Dog injection** where the kernel assembles the right panel of judges based on the claim's detected domain. nullclaw implements this with keyword matching in `filterToolSpecsForTurn`; CYNIC could use deterministic-dog's heuristics to classify before routing.

### Pattern 6 — Merkle Hash-Chain Verdict Audit
**From:** OpenFang (cryptographically linked audit trail)
**For CYNIC:** CYNIC's verdicts in SurrealDB are append-only but not cryptographically linked. Adding a hash chain (each verdict includes `prev_verdict_hash: sha256(previous_verdict)`) would:
- Make the verdict log tamper-evident
- Enable external parties to verify the epistemic record without trusting CYNIC's DB
- Provide a cryptographic root for the crystal system ("this crystal is derived from verdicts with root hash X")

This is two lines per verdict at write time and one check per audit. Implementation cost is minimal; epistemic credibility gain is significant for the hackathon demo story.

### Pattern 7 — Category Routing Before Model Selection
**From:** oh-my-openagent (category → model mapping at config time)
**For CYNIC:** CYNIC's Dog identities currently couple model and role (e.g. `sovereign` = Qwen 3.5 9B on S. GPU). If S. machine goes down, the `sovereign` Dog disappears entirely. Decoupling:

```toml
[dogs]
[dogs.deep-reasoning]
role = "deep-reasoning"
model = "sovereign"   # ← can be changed without changing routing logic
min_latency_ok = false

[dogs.fast-heuristic]
role = "fast-heuristic"
model = "deterministic"
```

Dogs declare their **role capability**, the kernel routes by role, the config maps role to model. Adding a new GPU changes only the config, not the routing logic.

### Pattern 8 — Priority-Tiered Verdict Cache
**From:** AgentKeeper (`critical=True` flag at ingestion)
**For CYNIC:** The VerdictCache O(1) eviction fix (already committed) evicts by recency. But verdicts that contributed to crystals are high-value and should be evicted last. Adding a `protect(verdict_id)` call during crystallization creates a two-tier cache: protected verdicts survive until absolute capacity; regular verdicts evict normally.

---

## Prioritized Implementation Order for CYNIC

| Priority | Pattern | Effort | Impact |
|---|---|---|---|
| P1 | Merkle hash-chain audit (Pattern 6) | Low — 2 lines/verdict + migration | High — epistemic credibility for demo |
| P1 | Category routing (Pattern 7) | Medium — config schema change + routing logic | High — infrastructure resilience |
| P2 | Dynamic Dog injection (Pattern 5) | Medium — claim classifier + routing table | Medium — cost reduction, quality improvement |
| P2 | Priority-tiered VerdictCache (Pattern 8) | Low — `protect()` method + eviction logic | Medium — cache hit rate for crystals |
| P3 | Adversarial pre-crystallization review (Pattern 4) | High — new audit agent + coordination | High — crystal quality assurance |
| P3 | Taint tracking (Pattern 3) | High — type-level changes across verdict pipeline | Medium — trust enforcement |
| P4 | Closed learning loop (Pattern 2) | Very High — crystal lifecycle events + calibration | High — long-term, not hackathon |
| P4 | Attribution chain (Pattern 1) | Low — coord session ID in dispatch | Low — observability improvement |

---

## Repository Quality Assessment

| Repo | Stars | Language | Relevance to CYNIC | Signal Quality |
|---|---|---|---|---|
| openfang | 15K | Rust | Very High | Excellent — 137K LOC real production code, 16 security systems, direct Rust patterns |
| oh-my-openagent | 42K | TypeScript | High | Excellent — Momus/Prometheus pattern is novel, hash-anchored edits are production-validated |
| nullclaw | 6.6K | Zig | High | Excellent — AGENTS.md is the most rigorous engineering protocol of the 8 repos |
| paperclip | 30K | TypeScript | Medium | Good — org chart model is useful for CYNIC fleet orchestration |
| hermes | 9.5K | Python | Medium | Good — learning loop architecture is the right direction for CYNIC |
| crucix | 5.7K | Node.js | Medium | Good — multi-source delta engine maps to multi-Dog verdict aggregation |
| dimos | 1.8K | Python | Low-Medium | Niche — typed stream edges and capability advertisement are transferable |
| agentkeeper | 115 | Python | Low | Minimal — small scope, but priority-tiered cache insight is genuine |

