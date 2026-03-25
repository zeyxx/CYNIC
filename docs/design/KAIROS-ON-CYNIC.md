# KAIROS on CYNIC — Design Document

*How a trading agent lives on an epistemic platform without building its own cathedral.*

## Status

**Draft.** Zero Rust code in this phase. Design first, prove foundations, then build.

## Evidence Base

This document is grounded in:
- Crystallize-truth analysis of CYNIC × KAIROS actual state (2026-03-22)
- Empirical research: 13 agent/platform projects (LMOS, AutoGPT, CrewAI, LangGraph, OpenFang, dimos, Temporal.io, OpenSandbox, AgentKeeper, Paperclip, Hermes-Agent, Ouroboros, ClawRouter)
- Counter-evidence: LMAX architecture, HFT latency research, hostile expert position on platform-mediated state
- Deep audit of both codebases (CYNIC kernel inventory, KAIROS subsystem map)

## The Core Tension

The consensus across 13 agent platforms: **the platform owns all persistence; agents are stateless workers.** This is correct for epistemics (judgment, learning, crystals) and wrong for execution (positions, orders, risk).

Trading violates the stateless-worker assumption in three ways:
1. **Non-idempotent operations** — sending an order twice ≠ sending it once
2. **Latency-critical correctness** — platform round-trip (50-500μs) exceeds arbitrage window
3. **Offline safety** — if the platform restarts, the agent must know its own positions to avoid uncapped exposure

Rule #28 stands: KAIROS does not manage its own database, its own learning loop, or its own storage schema. But Rule #28 does not mean zero local state — it means no independent infrastructure. Execution state is ephemeral and safety-critical, not a database.

**Falsifiable**: if KAIROS can safely continue operating during a 10-second CYNIC kernel restart without local position tracking, this tension is fabricated. Test: kill the kernel mid-trade-cycle, observe whether KAIROS can safely halt.

---

## 1. What KAIROS Keeps

These subsystems are domain-specific and irreplaceable. They run inside KAIROS, not CYNIC.

| Subsystem | What It Does | Why It Stays |
|---|---|---|
| **Ingestion adapters** (Pyth, Jupiter, Hyperliquid) | Fetch market data from exchange APIs | Domain-specific protocols, latency-sensitive, changes with exchange API versions |
| **Signal Conditioner** (Kalman filter) | Noise reduction, regime detection | Domain math, <1ms, no reason to network-mediate a Kalman update |
| **Monte Carlo engine** (GBM + jump-diffusion) | Win probability estimation | Pure math, stateless per call |
| **Mechanical Judge** (42 facets, 6 axioms) | System 1 — instant deterministic verdict | <1ms, must not wait for network. 32/42 facets are live, 10 are neutral stubs |
| **Execution logic** | Position sizing, risk rules, order routing | Safety-critical: must enforce limits locally even if CYNIC is unreachable |
| **Local execution state** | Open positions, orders in flight, real-time P&L | Ephemeral, safety-critical. Refreshed from CYNIC on reconnect (not authoritative — CYNIC is) |

### What KAIROS Deletes

| Subsystem | Lines | Why |
|---|---|---|
| `particle_filter.py` | ~500 | 0 callers. Dead code. Burn it. |
| `anthropic` dependency | — | 0 imports in source. Dead dependency. |
| `FearGreedIngester` (production wiring) | ~100 | Implemented, tested, never wired. Wire it or delete it. |
| `DexScreenerIngester` (commented out) | ~50 | Commented out = dead. Wire it or delete it. |
| `ChronosJudge` / `AionJudge` references | — | Referenced in comments, never implemented. Delete the references. |
| `FrequencyBridge` port | — | Port defined, 0 adapters. Delete. |
| `TemporalVector.judge_tier()` routing | — | 3 tiers declared, 1 implemented. Delete the routing, keep KairosJudge. |
| Own SurrealDB (`KairosDB`) | ~300 | Violation of Rule #28. Migrate to CYNIC agent state API. |

**Falsifiable**: after deletion, `python -m pytest` must still pass with 0 failures on all non-integration tests. If any test breaks, the code wasn't dead.

---

## 2. What KAIROS Delegates to CYNIC

| Function | CYNIC Endpoint | Contract | Latency Budget |
|---|---|---|---|
| **Epistemic judgment** | `POST /judge` | Send conditioned signal + 42 facets as context. Receive multi-Dog verdict. | Async — fire-and-forget, result stored as crystal observation. Does NOT block execution. |
| **Persist trade decision** | `POST /observe` | Domain: `trading`, content: structured decision JSON (signal, verdict, action, reasoning). | Async — <100ms, fire-and-forget. |
| **Read crystallized wisdom** | `GET /crystals?domain=trading` | Mature crystals (21+ obs, conf ≥ 0.618) for the trading domain. | On-startup + periodic refresh (60s). Cache locally. |
| **Report agent health** | `POST /coord/register` + heartbeat | Session registration, heartbeat every 60s. | <50ms per heartbeat. |
| **Claim/release coordination** | `POST /coord/claim` | Before modifying strategy parameters, claim the resource. | <50ms. |
| **Persist Q-Table** | `POST /agent/{id}/state` | **(NEW — does not exist yet)** Key-value per domain. KAIROS writes Q-Table entries as domain-tagged state. | Every 5min (learner tick), batch write. |
| **Retrieve risk limits** | `GET /agent/{id}/config` | **(NEW)** CYNIC-authoritative risk limits (max position, max drawdown, capital). KAIROS caches locally, refreshes every 60s. | On-startup + periodic. |

### The Delegation Contract

1. KAIROS **never blocks execution** on a CYNIC response. Every CYNIC call is fire-and-forget or cached.
2. KAIROS **refreshes** epistemic state (crystals, risk limits, Q-Table) on startup and periodically.
3. If CYNIC is unreachable, KAIROS **continues with stale cache** and logs the gap. It does NOT halt trading.
4. KAIROS **reports everything** to CYNIC: every signal, every judgment, every trade decision. CYNIC is the system of record.

**Falsifiable**: set `CYNIC_URL` to an unreachable host. KAIROS must still: (a) produce mechanical verdicts, (b) execute dry-run trades, (c) log locally. If it crashes or hangs, the delegation is wrong.

---

## 3. What CYNIC Must Add

Four services are missing. Each is independently testable.

### 3.1 Agent State Storage

**What**: Key-value store per agent per domain. `POST /agent/{id}/state/{key}`, `GET /agent/{id}/state/{key}`, `DELETE /agent/{id}/state/{key}`.

**Why**: KAIROS Q-Table, strategy parameters, and cached risk limits need platform-managed persistence without KAIROS running its own DB. Every agent-platform project stores agent state in the platform (CrewAI: ChromaDB+SQLite, LangGraph: checkpointer, Paperclip: PostgreSQL, Hermes: SQLite).

**Schema**: `agent_state` table in SurrealDB. Fields: `agent_id`, `domain`, `key`, `value` (JSON), `updated_at`. TTL: configurable per domain.

**Falsifiable**: write a Q-Table entry via API, restart the kernel, read it back. If it's gone, the storage doesn't work.

### 3.2 Agent Config / Risk Limits

**What**: `GET /agent/{id}/config` returns CYNIC-authoritative configuration for an agent. Includes risk limits that the agent MUST obey but CANNOT modify.

**Why**: Ouroboros proved that safety parameters must be platform-owned, not agent-reasoned. KAIROS must not be able to raise its own position limits. CYNIC is the authority. The agent caches and obeys.

**Schema**: `agent_config` table. Fields: `agent_id`, `config` (JSON), `updated_at`. Managed by human operator via API or direct DB.

**Falsifiable**: set max_position=3 in CYNIC config. Have KAIROS attempt to open a 4th position. If it succeeds, the boundary is broken.

### 3.3 Event Stream

**What**: `WebSocket /events` — typed event stream, filterable by agent and domain.

Event types:
```
AgentRegistered { agent_id, timestamp }
AgentHeartbeat  { agent_id, timestamp }
JudgmentStarted { stimulus_domain, agent_id, timestamp }
DogEvaluated    { dog_id, verdict_class, q_score, latency_ms, timestamp }
VerdictIssued   { verdict_id, q_score, verdict_class, domain, timestamp }
CrystalObserved { crystal_id, observations, confidence, state, timestamp }
CrystalPromoted { crystal_id, old_state, new_state, timestamp }
ObservationStored { domain, agent_id, timestamp }
```

**Why**: The operator is blind. Every platform researched (LMOS, LangGraph, Paperclip, OpenFang) provides platform-owned observability. CYNIC has CRUD endpoints but no event stream. The data exists in the kernel — it just needs to be broadcast.

**Falsifiable**: connect a WebSocket client, submit a `/judge` request, verify that JudgmentStarted + N × DogEvaluated + VerdictIssued events arrive in order with correct timestamps.

### 3.4 Crystal Domain Namespaces

**What**: Crystals tagged with `agent_id` in addition to `domain`. Retrieval filters by both. Agent A's crystals don't pollute Agent B's prompt context.

**Why**: Without namespaces, KAIROS trading crystals would appear in chess judgments (and vice versa). The `domain` field partially solves this, but multiple agents in the same domain (two trading agents with different strategies) would cross-contaminate.

**Schema change**: add `agent_id` field (optional) to `crystal` table. Retrieval: `WHERE domain = $domain AND (agent_id = $agent OR agent_id IS NONE)`. `agent_id IS NONE` = shared wisdom (cross-agent crystals promoted by confidence).

**Falsifiable**: create crystals for agent-A and agent-B in domain `trading`. Query crystals for agent-A. If agent-B crystals appear, namespacing is broken.

### Build Order (Rule #27 — compound connections)

```
3.1 Agent State Storage
 └─→ enables 3.2 (config stored in same subsystem)
 └─→ enables KAIROS Q-Table migration
 └─→ enables any future agent's state persistence

3.3 Event Stream
 └─→ enables frontend dashboard (S.'s zone)
 └─→ enables human observability (Section 6)
 └─→ enables automated monitoring/alerting
 └─→ feeds CCM observations (replaces hook-based observation)

3.4 Crystal Namespaces
 └─→ enables multi-agent without contamination
 └─→ enables KAIROS-specific crystal accumulation
```

Order: **3.1 → 3.2 → 3.3 → 3.4**. Each is independently deployable and testable.

---

## 4. Migration Plan

### Phase 0: Clean House (no CYNIC changes)

**In KAIROS only.** Delete dead code, fix port conformance.

| Step | What | Test |
|---|---|---|
| 0.1 | Delete `particle_filter.py`, `anthropic` dep, dead references | `pytest` passes, no import errors |
| 0.2 | Wire `KairosDB` to implement `StoragePort` protocol (or delete `StoragePort` if unused) | Ports match implementations |
| 0.3 | Fix `ralph_loop.py` to use fused score when CYNIC is available (currently ignores CYNIC result) | Assert: when CYNIC is reachable, fused score is used; when unreachable, local score is used |
| 0.4 | Remove `DexScreenerIngester` commented-out code, `FearGreedIngester` unwired code (or wire them) | `grep -r "DexScreener\|FearGreed" --include="*.py"` returns only active code or zero |

**Gate**: `pytest` all green. No behavior change. Pure cleanup.

### Phase 1: Wire CYNIC Judgment (KAIROS changes only)

CYNIC already has everything needed for this phase.

| Step | What | Test |
|---|---|---|
| 1.1 | `ralph_loop.py`: submit every conditioned signal to `POST /judge` (fire-and-forget, async) | `/verdicts?domain=trading` returns trading judgments |
| 1.2 | `ralph_loop.py`: call `POST /observe` with structured trade decision after every cycle | `/crystals?domain=trading` starts accumulating observations |
| 1.3 | On startup: `GET /crystals?domain=trading` → cache locally → inject into mechanical judge context | Mechanical judge prompt includes crystal text when crystals exist |
| 1.4 | Heartbeat: `POST /coord/register` on start, heartbeat every 60s | `/agents` shows KAIROS with active session |

**Gate**: run KAIROS for 1 hour. Verify: (a) trading observations appear in CYNIC, (b) crystal observations accumulate (forming state), (c) no execution regression (dry-run trades identical with/without CYNIC). This is Rule #26 — prove the foundation before building on it.

**Falsifiable**: compare trade decisions with CYNIC enabled vs disabled over 100 cycles. If decisions differ by >5%, the integration is influencing execution prematurely (before crystals mature).

### Phase 2: CYNIC Platform Services (Rust changes)

Build the four services from Section 3.

| Step | What | Depends On | Test |
|---|---|---|---|
| 2.1 | Agent State Storage API | Nothing | Integration test: write → restart → read |
| 2.2 | Agent Config API | 2.1 (same table pattern) | Write config, read from KAIROS, verify enforcement |
| 2.3 | Event Stream WebSocket | Nothing (parallel with 2.1) | WebSocket client receives events during `/judge` |
| 2.4 | Crystal Namespaces | Nothing (schema migration) | Agent-A crystals isolated from Agent-B |

**Gate**: `/build` passes. Each endpoint has integration test. `/test-chess` shows no regression.

### Phase 3: KAIROS Migration (KAIROS changes)

| Step | What | Depends On | Test |
|---|---|---|---|
| 3.1 | Migrate Q-Table persistence: `KairosDB.upsert_q_entry` → `POST /agent/kairos/state/q_table` | 2.1 | Q-Table survives KAIROS restart without own DB |
| 3.2 | Risk limits from CYNIC: `GET /agent/kairos/config` → local cache, refresh 60s | 2.2 | Position ceiling enforced even if KAIROS code is modified |
| 3.3 | Remove `KairosDB` entirely. Remove own SurrealDB dependency. | 3.1, 3.2 | `grep -r "surreal\|SurrealDB\|KairosDB" --include="*.py" kairos/` returns zero (except test mocks) |
| 3.4 | Connect to event stream for self-monitoring | 2.3 | KAIROS logs its own events from CYNIC's perspective |

**Gate**: KAIROS runs for 24 hours with zero own-DB dependency. All state persisted via CYNIC. Trade decisions unchanged.

**Falsifiable**: after Phase 3, `ls /home/user/Bureau/KAIROS/ -R | grep -i surreal` returns nothing. KAIROS `requirements.txt` / `pyproject.toml` has no SurrealDB dependency.

### Phase 4: Crystal Loop Activation

This phase is **gated by Phase 1 volume**. Only proceed when:
- 100+ trading observations exist in CYNIC
- At least 3 crystals have reached `forming` state

| Step | What | Test |
|---|---|---|
| 4.1 | Fix crystal accumulation: replace FNV-1a exact hash with embedding-based similarity clustering (cosine threshold 0.85) | Similar signals ("SOL pump Asian session" vs "SOL pump late Asian") merge into same crystal |
| 4.2 | Verify crystal injection into Dog prompts with trading content | `grep "CYNIC Memory" /tmp/dog_prompts.log` shows trading crystal text |
| 4.3 | Benchmark: `/test-chess` before and after (regression check) + new `/test-trading` benchmark | No chess regression, trading crystals improve signal quality |

**Falsifiable**: submit 25 similar (but not identical) trading signals. If fewer than 5 crystals reach 21 observations, the clustering threshold is wrong.

---

## 5. System 1 / System 2

### The Model

| Property | System 1 (KAIROS Mechanical) | System 2 (CYNIC Consensus) |
|---|---|---|
| **Speed** | <1ms | 18-30s (sovereign), 2-5s (API Dogs) |
| **Nature** | Deterministic, 42 facets, 6 axioms | Probabilistic, 5 Dogs, multi-model |
| **Decision authority** | Execution decisions (trade/hold/exit) | Learning updates (crystal accumulation) |
| **Runs when** | Every cycle (60s) | Async, fire-and-forget per signal |
| **Failure mode** | Never fails (pure math) | Fails gracefully (KAIROS continues) |
| **Influence on the other** | Facet scores enrich CYNIC Dog prompts | Crystals modify KAIROS's context over time |

### How They Interact

```
KAIROS cycle (60s):
  1. Fetch market data          ← domain (KAIROS)
  2. Condition signal (Kalman)  ← domain (KAIROS)
  3. Monte Carlo win prob       ← domain (KAIROS)
  4. System 1: mechanical judge ← <1ms, deterministic, uses cached crystals
  5. Execute (dry-run/live)     ← domain (KAIROS), gated by CYNIC risk limits
  6. Fire-and-forget to CYNIC:
     a. POST /judge (signal + facets as context)     ← async
     b. POST /observe (decision record)              ← async

CYNIC (async, background):
  7. Dogs evaluate with crystal context   ← 18-30s
  8. Verdict stored, crystal observed     ← accumulation
  9. Crystal matures (21+ obs)            ← delayed influence
  10. Next KAIROS startup/refresh:
      crystals injected into System 1    ← influence loop closes
```

**The influence is delayed, not real-time.** System 2 never blocks System 1. System 2 influences System 1 through crystal accumulation — a slow, weighted, evidence-based channel. This is more appropriate than real-time override for trading (18-30s latency is unacceptable for execution).

### The Talker-Reasoner Limitation (and our response)

The DeepMind Talker-Reasoner model (arXiv:2410.08328) has a structural flaw: it requires manual labeling of when to wait for the Reasoner. Our architecture avoids this by making the Reasoner (CYNIC) purely asynchronous and never blocking. The influence channel (crystals) has natural latency that self-labels: only patterns with 21+ concordant observations cross the threshold. This is automatic, not manually labeled.

For future consideration: Global Workspace Theory (GWT) provides a better theoretical model than dual-process — N processors at different speeds competing for attention, not just 2. CYNIC's Dogs are already N processors. Crystal promotion is a form of attention competition. This is a natural evolution, not a redesign.

**Falsifiable**: measure trade quality (P&L, Sharpe, drawdown) with System 2 enabled vs disabled over 1000 cycles. If System 2 adds no measurable improvement, it is overhead, not compound.

---

## 6. Observability

### What the operator sees today

Nothing useful. `curl /health` returns a status code. `curl /verdicts` returns JSON. No timeline, no narrative, no pulse.

### What the operator needs

A real-time narrative stream: "14:32 KAIROS woke → fetched SOL/USDC → conditioned: regime=trending, vol=high → System 1: WAG Q=0.48 → submitted to CYNIC → 14:32:18 deterministic-dog: WAG 0.42 → 14:32:19 gemini: HOWL 0.55 → 14:32:37 sovereign: timeout → crystal 'sol-asian-pump' grew (obs 14→15, forming) → verdict: WAG Q=0.51"

This is the pulse of the organism. Not JSON to parse. Lived experience.

### Implementation

**Backend** (CYNIC kernel — Section 3.3):
- WebSocket `/events` emitting typed events
- Events emitted at existing code points (no new business logic, just broadcast)
- Filterable by `?agent=kairos&domain=trading`

**Frontend** (cynic-ui — S.'s zone):
- Timeline view consuming WebSocket events
- Crystal growth visualization (observation count over time, state transitions)
- Agent health dashboard (heartbeat status, last activity, uptime)
- System 1 vs System 2 comparison (mechanical verdict vs CYNIC consensus per signal)

**Terminal** (for T. — immediate):
- `websocat ws://localhost:3030/events | jq` — raw event stream
- Future: `cynic watch --agent kairos` CLI command

### Three Levels of Observability

| Level | Audience | Medium | Data |
|---|---|---|---|
| **Pulse** | Operator glancing | Single-line status | "KAIROS: alive, 3 open positions, last signal 42s ago, System 2 lag: 12s" |
| **Timeline** | Operator investigating | Event stream (WebSocket) | Typed events with timestamps, filterable |
| **Forensics** | Post-incident analysis | `/verdicts` + `/crystals` + `/agents` | Full history, queryable by time range and domain |

**Falsifiable**: connect a WebSocket client, run KAIROS for 10 minutes. If the operator cannot answer "what did KAIROS do in the last 5 minutes?" from the event stream alone, the observability is insufficient.

---

## 7. Success Metrics

These are not "it compiles." These are falsifiable claims about system behavior.

### Phase 1 Success (KAIROS → CYNIC wiring)

| Metric | Target | How to Measure |
|---|---|---|
| Trading observations in CYNIC | 100+ after 24h of KAIROS running | `GET /crystals?domain=trading` count |
| Crystal accumulation | ≥5 crystals in `forming` state | `GET /crystals` filter by state |
| Execution regression | <5% decision difference vs standalone | Compare verdict distributions with/without CYNIC |
| KAIROS resilience | 0 crashes during 10s CYNIC outage | Kill kernel, observe KAIROS |
| Heartbeat uptime | >99% heartbeat success rate | `GET /agents` session history |

### Phase 2 Success (CYNIC platform services)

| Metric | Target | How to Measure |
|---|---|---|
| Agent state round-trip | Write → restart kernel → read = identical | Integration test |
| Risk limit enforcement | 0 violations when limit is set | Attempt to exceed limit programmatically |
| Event stream latency | <100ms from kernel event to WebSocket delivery | Timestamp comparison |
| Crystal namespace isolation | 0 cross-agent contamination | Query agent-A, assert 0 agent-B crystals |

### Phase 3 Success (KAIROS migration)

| Metric | Target | How to Measure |
|---|---|---|
| Zero own-DB dependency | `grep surreal pyproject.toml` = empty | Automated check |
| Q-Table persistence via CYNIC | Survives KAIROS restart | Write entries, restart, read back |
| Trade decision quality | No regression vs pre-migration baseline | 1000-cycle comparison |
| CYNIC downtime tolerance | KAIROS trades correctly for ≥5 minutes without CYNIC | Network partition test |

### Phase 4 Success (crystal loop activation)

| Metric | Target | How to Measure |
|---|---|---|
| Crystal maturation rate | ≥3 crystals reach `crystallized` (21+ obs) in 7 days | `GET /crystals` query |
| Crystal injection verified | Dog prompts contain trading crystal text | Log analysis |
| Feedback loop measurable | Δ Q-score ≥ +0.01 between week 1 and week 4 | `/verdicts` historical comparison |
| No chess regression | `/test-chess` scores stable (±0.02) | Before/after benchmark |

### The Ultimate Metric

**Can the human operator, watching the event stream for 5 minutes, explain what the organism is doing and why?**

If yes: CYNIC is an epistemic platform with a living agent.
If no: CYNIC is a JSON API with a cron job.

---

## Appendix A: Empirical Research Summary

### The Consensus (13 platforms)

| Platform | Agent owns state? | Platform owns persistence? | System 1/2? |
|---|---|---|---|
| LMOS (Eclipse) | No | Yes (Qdrant + K8s) | Router fast / LLM slow |
| AutoGPT | No (agent IS data) | Yes (PostgreSQL) | No |
| CrewAI | No | Yes (ChromaDB + SQLite) | Manager/Worker tiers |
| LangGraph | No | Yes (checkpointer) | Composable via graph |
| OpenFang | No | Yes (openfang-memory) | Hands vs interactive |
| dimos | No | Yes (RxPY streams) | Reactive / LLM |
| Temporal.io | No (stateless workers) | Yes (event history) | Activities / Workflows |
| OpenSandbox | Split (container state) | Yes (control plane) | No |
| AgentKeeper | Agent-owned SQLite | Per-agent, not shared | No |
| Paperclip | No | Yes (PostgreSQL) | No |
| Hermes-Agent | No | Yes (SQLite + lineage) | L0-L3 cooperation ladder |
| Ouroboros | Agent owns source code | Supervisor owns budget/state | No |
| ClawRouter | N/A (routing service) | N/A | Tier-based routing |

### The Counter-Argument (hostile expert)

The "platform owns everything" pattern was designed for enterprise chatbots and workflow automation. Trading violates three assumptions: latency budget (50-500μs round-trip kills opportunities), non-idempotent operations (double-send an order = double exposure), and offline safety (platform restart ≠ agent blindness to own positions).

**Resolution**: two-tier state ownership. Epistemics to the platform. Execution state to the agent. Neither owns everything.

### Key Patterns Adopted

| Pattern | Source | Applied How |
|---|---|---|
| Platform owns safety surface | Ouroboros, Paperclip | CYNIC owns risk limits, KAIROS obeys |
| Memory as distilled facts, not transcripts | AgentKeeper | Crystals = distilled facts with criticality |
| Routing is infrastructure | ClawRouter | CYNIC routes Dogs, KAIROS doesn't choose |
| Cooperation ladder (L0→L3) | Hermes-Agent | Start at L1 (crystal injection), evolve to L3 (Dog debate) |
| Ambient oversight via sidecar | OpenSandbox | Every KAIROS decision auto-observed by CYNIC |
| Checkpoint per tool call | Hermes-Agent | KAIROS reports every cycle, not just trades |
| Event sourcing for durability | LMAX, Temporal.io | CYNIC events are the system of record |

### What No Project Has (CYNIC's unique angle)

**Dynamic trust boundary.** Every researched platform has a static agent/platform split. None implements boundary negotiation where the platform observes agent behavior over time and adjusts autonomy based on demonstrated reliability. CYNIC's crystal loop is the seed: as crystals mature on an agent's decisions, the platform accumulates evidence of the agent's competence. This could enable progressive autonomy — a KAIROS that consistently produces HOWL verdicts earns larger position limits. No other project ships this.

---

## Appendix B: State Ownership Map

```
┌─────────────────────────────────────────────────────────┐
│                    CYNIC PLATFORM                        │
│                                                         │
│  Epistemic State (authoritative, persisted)             │
│  ├── Verdicts (judge pipeline output)                   │
│  ├── Crystals (accumulated wisdom, domain-tagged)       │
│  ├── Agent State K/V (Q-Table, strategy params)         │
│  ├── Agent Config (risk limits, capital — human-set)    │
│  ├── Observations (raw events, 30d TTL)                 │
│  ├── Session Summaries (LLM-compressed narratives)      │
│  └── Coordination (claims, sessions, audit trail)       │
│                                                         │
│  Services                                               │
│  ├── POST /judge (multi-Dog consensus)                  │
│  ├── POST /observe (event ingestion)                    │
│  ├── GET  /crystals (wisdom retrieval)                  │
│  ├── WS   /events (observability stream)    [NEW]       │
│  ├── */agent/{id}/state/* (K/V storage)     [NEW]       │
│  └── GET  /agent/{id}/config (risk limits)  [NEW]       │
│                                                         │
└──────────────────────┬──────────────────────────────────┘
                       │ async (fire-and-forget)
                       │ periodic refresh (60s)
                       │ never blocks execution
┌──────────────────────┴──────────────────────────────────┐
│                    KAIROS AGENT                          │
│                                                         │
│  Execution State (ephemeral, safety-critical)           │
│  ├── Open positions (local, refreshed from CYNIC)       │
│  ├── Orders in flight (local only)                      │
│  ├── Real-time P&L (computed locally from market data)  │
│  ├── Risk limit cache (from CYNIC, refreshed 60s)       │
│  └── Crystal cache (from CYNIC, refreshed on startup)   │
│                                                         │
│  Domain Logic (irreplaceable)                           │
│  ├── Ingestion (Pyth, Jupiter, Hyperliquid)             │
│  ├── Signal Conditioner (Kalman)                        │
│  ├── Monte Carlo (GBM + jump-diffusion)                 │
│  ├── Mechanical Judge (42 facets, System 1)             │
│  └── Execution Logic (position sizing, risk)            │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

*Crystallized 2026-03-22. Evidence: 13 platforms, 3 codebases, 1 hostile expert, 0 Rust code changed.*
