# Agent Taxonomy — CYNIC Organism

> Each agent type has a distinct lifecycle, coordination mechanism, and interface contract.
> This document is the bootstrap reference for any new agent joining the organism.

---

## The 4 Agent Types

### 1. Cortex (Development Agents)

**What:** LLM sessions that reason about code, architecture, and design. Episodic (session-bounded). The organism's prefrontal cortex.

**Instances (observed, May 2026):**

| Instance | Model | Tier | Natural scope |
|----------|-------|------|---------------|
| Haiku A | Claude Haiku | Mechanical | Single-file fixes, lint, formatting, health probes, git cleanup |
| Haiku B | Claude Haiku | Mechanical | Scripts, data processing, cron wiring, simple refactors |
| Opus | Claude Opus | Deep | Architecture, multi-module refactors, design decisions, judgment |
| Gemini | Gemini CLI | Autonomous | Hermes organic, philosophical synthesis, domain exploration |

**Lifecycle:**
```
human dispatch (first message)
    → branch (MC1: random suffix, immediate)
    → perceive (read CLAUDE.md, TODO.md DAG, probe /health)
    → execute (edit, test, commit)
    → emit (PR)
    → consensus (human merges)
```

**Reads:** CLAUDE.md, TODO.md (DAG format), kernel `/health`, mempool (`/observations?domain=mempool`)
**Writes:** Code (via branch+PR), mempool (discoveries: `POST /observe domain=mempool`)
**Never writes:** TODO.md (human-only). Other cortex's branch.
**Coordination:** Module-level ownership. Branch isolation. Human dispatch = scope.

**Tier routing (from cost.md):**

| Task type | Tier | Model |
|-----------|------|-------|
| Lint, format, single-file fix | Mechanical | Haiku |
| Multi-file refactor, exploration | Standard | Sonnet/Haiku |
| Architecture, cross-module synthesis | Deep | Opus |
| Philosophical, autonomous, organic | Autonomous | Gemini |

---

### 2. Organ (Autonomous Subsystems)

**What:** Long-running services that perceive, transform, and evolve. Persistent (systemd/cron). The organism's viscera.

**Instances:**

| Organ | Purpose | Runtime |
|-------|---------|---------|
| Hermes X (proxy) | Capture X.com data via mitmproxy | systemd service |
| Hermes X (ingest) | Process captures → dataset.jsonl | systemd service |
| Hermes (data organism) | Perceive → reflect → evolve cycle | systemd timer (hourly) |
| Hermes (search executor) | Autonomous browsing via CDP | systemd service |

**Lifecycle:**
```
trigger (cron / event / timer)
    → perceive (read data sources, behavior logs)
    → transform (score, classify, extract)
    → reflect (write reflection, compare to SKILL.md)
    → evolve (update SKILL.md weights, domain confidence)
    → emit observation (POST /observe)
```

**Reads:** Data files (dataset.jsonl, behavior_log.jsonl), SKILL.md, kernel state
**Writes:** Observations to kernel, data artifacts, reflections, SKILL.md evolution
**Coordination:** Soma orchestrator (resource allocation, GPU budget, conflict prevention)

---

### 3. Dog (Independent Validators)

**What:** Scoring functions called per judgment request. Stateless, reactive. The organism's immune cells.

**Instances:**

| Dog | Type | Latency | Sovereign |
|-----|------|---------|-----------|
| deterministic-dog | Heuristic | 0ms | Yes |
| qwen-7b-hf | LLM (HF Inference) | 3.7s | No |
| qwen35-9b-gpu | LLM (local GPU) | 13.2s | Yes |
| gemini-cli | LLM (CLI subprocess) | 23s | No |

**Lifecycle:**
```
kernel calls (stimulus + domain + crystal context)
    → score (6 axioms independently)
    → respond (AxiomScores + reasoning)
    → kernel aggregates (trimmed mean → Q-score → verdict)
```

**Reads:** Stimulus (content to judge), crystal context (prior judgments), domain prompt
**Writes:** Nothing persistent. Scores flow back to kernel.
**Coordination:** Circuit breakers (auto-open on failure), Dog roster (TTL-based), discovery loop

---

### 4. Meta-Agent (Subagents)

**What:** Short-lived agents spawned by a cortex for bounded subtasks. The organism's reflexes.

**Instances (dispatched on demand):**

| Agent | Trigger | Purpose |
|-------|---------|---------|
| organism-architect | Before structural decisions | PHI+BURN analysis |
| rust-guardian | Before kernel commits | FIDELITY+VERIFY review |
| sovereign-ops | Deploy/infra tasks | SOVEREIGNTY enforcement |
| token-watchlist | Cron / before demo | Feed real data to Dogs |
| dream-consolidator | Sessions >= 21 | Memory cleanup |

**Lifecycle:**
```
parent cortex dispatches (Agent tool, bounded prompt)
    → inherits parent scope (branch, module ownership)
    → executes subtask
    → returns result to parent
    → terminates (no persistence)
```

**Reads:** Whatever parent gives it (files, context)
**Writes:** Results to parent (not directly to repo unless in worktree)
**Coordination:** Inherited from parent cortex. No independent claims.

---

## Inter-Agent Communication

```
                    ┌─────────────────────────────────┐
                    │         HUMAN (curator)          │
                    │  dispatches cortex, merges PRs   │
                    │  curates mempool → TODO.md DAG   │
                    └────┬──────────┬──────────┬───────┘
                         │          │          │
              ┌──────────▼───┐ ┌────▼────┐ ┌───▼──────────┐
              │  Cortex (x3) │ │  Gemini │ │  Cortex (x1) │
              │  Haiku       │ │  Auto   │ │  Opus        │
              └──────┬───────┘ └────┬───���┘ └───┬──────────┘
                     │              │           │
                     │    ┌─────────▼───────┐   │
                     │    │  Mempool        │   │
                     ├───►│  (kernel        │◄──┤
         discoveries │    │  /observations) │   │ discoveries
                     │    └─────────┬───────┘   │
                     │              │           │
              ┌──────▼───────────────▼──────────▼──────┐
              │              KERNEL                     │
              │  /judge  /agent-tasks  /coord  /health │
              └──────┬───────────────────┬─────────────┘
                     │                   │
              ┌──────▼─────��┐     ┌──────▼──────┐
              │    Dogs     │     │   Organs    │
              │  (scoring)  │     │  (sensing)  │
              └─────────────┘     └─────────────┘
```

---

## Two Work Queues (Not One)

| System | Purpose | Lifecycle | Written by | Read by |
|--------|---------|-----------|------------|---------|
| **Mempool** (`/observations?domain=mempool`) | Discovery — "I noticed X" | Immutable. Items accumulate. Human curates. | Cortex, Organs | Human (curation), Cortex (awareness) |
| **TODO.md DAG** | Planning — "Do X, it enables Y" | Mutable by human only. Tracks + dependencies. | Human only | Cortex (scope), Human (overview) |

**The bridge:** Human reads mempool, promotes ripe items to TODO.md DAG. This is the curation step — judgment about what's worth doing, sequencing, and assignment.

**Why not one system?** Observations are facts (immutable). Tasks are intentions (mutable state machines: pending → claimed → done). Mixing them violates single-responsibility and makes the mempool unreliable as a factual record.

---

## Coordination Mechanisms per Type

| Mechanism | Cortex | Organ | Dog | Meta-Agent |
|-----------|--------|-------|-----|------------|
| Branch isolation | Yes (MC1) | No (service) | No (stateless) | Inherited |
| Module ownership | Yes (human dispatch) | Yes (MANIFEST) | No | Inherited |
| Mempool write | Yes (discoveries) | Yes (observations) | No | Via parent |
| TODO.md read | Yes (scope) | No | No | No |
| Soma resource gate | No | Yes (GPU/memory) | Indirect (circuit) | No |
| Circuit breaker | No | No | Yes (auto) | No |
| /coord/claim | Optional | No | No | No |

---

## Evolution Path

**Today (3-4 cortex):** Human dispatch + branch isolation + module ownership. Works.

**Tomorrow (5-8 cortex):** Need automated dispatch. TODO.md DAG encodes which tier each task needs → cortex self-selects by matching its tier. Human only curates the DAG, doesn't dispatch individually.

**Future (10+ agents):** Kernel becomes the scheduler. `/agent-tasks` extended with tier field. Cortex polls for unclaimed tasks matching their tier + domain. Human sets priorities, agents self-organize. Mempool feeds the queue automatically when confidence reaches threshold.

**Falsifiable milestones:**
- Today → Tomorrow: when human can't remember which cortex has which scope mid-session
- Tomorrow → Future: when >50% of mempool items could be auto-promoted (pattern recognition)
