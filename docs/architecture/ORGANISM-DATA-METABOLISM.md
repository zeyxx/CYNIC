# CYNIC Organism — Data-Centric Architecture

> **Thesis:** Intelligence is an edge product of data metabolism. The organism's value is its datasets, not its inferences. Wisdom emerges when humans see patterns in logs that the system couldn't invent alone.

---

## Epistemic Status

- **Observed:** Claude Code sessions are logged (`.jsonl` files in `~/.claude/projects/`); kernel observations exist (SurrealDB); organ logs scatter across directories; agent tasks are tracked (agent_tasks table)
- **Deduced:** No single data aggregation point exists; Askesis is starving; pattern extraction is manual
- **Conjecture:** Data pipeline → Askesis → human reflection + Gemini learning = the closed loop that currently doesn't exist

---

## The Goldmine: Claude Code Session Logs

**Location:** `~/.claude/projects/-home-user-Bureau-CYNIC/` (and other session dirs)

**What's in there:**
- Every conversation turn (user message + Claude response)
- Every tool invocation (Read, Edit, Bash, Grep, etc.)
- Every decision point (Plan mode entries, refactorings, escalations)
- Every error and recovery (failed builds, git conflicts, test failures)
- Timestamps, context, reasoning chains

**Why it matters:**
- Kernel logs are "what the machine did"
- Session logs are "why the human chose what the machine should do"
- Combined: you have the full causality chain from intention → decision → execution → outcome

**Data structure (observed):**
```jsonl
{
  "timestamp": "2026-04-30T12:00:00Z",
  "turn": 42,
  "user_message": "ok, en attendant on peut travailler sur cynic node?",
  "claude_response": "go, je voulais que tu me montre le diagramme...",
  "tools_called": ["Read", "Edit", "Agent"],
  "decisions_made": ["refactor Phase B as Soma's command layer", "encode distributed topology as future"],
  "session_id": "b696d837-c825-416a-b8a8-10f1f89193ae"
}
```

This is **the source of strategic knowledge** that Askesis should synthesize.

---

## The Complete Data Pipeline

```
LAYER 1: LOG SOURCES
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

┌─────────────────────────────────────────────────────────┐
│ KERNEL METABOLISM (cynic-kernel)                        │
│ • /observe events (K15 observations)                    │
│ • Dog health snapshots (latency, error rate, model)     │
│ • Node heartbeats (hardware, models loaded)             │
│ • Command execution (Soma → node logs)                  │
│ • Judge updates (Dog roster changes)                    │
│ • Crystal formation (verdict aggregation)               │
│                                                          │
│ Storage: SurrealDB (tables: observations, events, ...)  │
│ Format: structured (schema-enforced)                    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ EXECUTION METABOLISM (cynic-node)                       │
│ • Hardware probes (GPU memory, CPU%, temperature)       │
│ • Model load/switch/unload commands                     │
│ • Process lifecycle (spawn, crash, respawn)             │
│ • Dog health checks (endpoint latency, /health probe)   │
│ • Failures (timeout, OOM, network partition)            │
│                                                          │
│ Storage: Local file (~/logs/node-{id}.jsonl)            │
│ Format: JSONL (append-only)                             │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ AGENTIC METABOLISM (organs: hermes-x, inference_lab)    │
│ • Task dispatch (what work was assigned)                │
│ • Tool invocations (which tools, arguments, results)    │
│ • Decision logs (why chose A over B)                    │
│ • Error recovery (fallback strategies)                  │
│ • Outcome measurements (did task succeed?)              │
│                                                          │
│ Storage: ~/.cynic/organs/{organ_id}/data.jsonl          │
│ Format: JSONL (domain-specific schema)                  │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ STRATEGIC METABOLISM (Claude Code sessions)             │
│ • Session context (what was the human trying to do?)    │
│ • Decision points (refactor? add feature? debug?)       │
│ • Tool usage patterns (which tools for which tasks?)    │
│ • Error handling (how did human respond to failures?)   │
│ • Reasoning chains (multi-turn problem solving)         │
│ • Code changes (what was changed and why)               │
│                                                          │
│ Storage: ~/.claude/projects/*/sessions.jsonl            │
│ Format: JSONL (turn-by-turn conversation)               │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ BEHAVIORAL METABOLISM (human UI/interaction)            │
│ • Click patterns (where does user focus?)               │
│ • Deliberation time (pause points, re-reads)            │
│ • Scrolling behavior (depth of engagement)              │
│ • Search queries (what are they looking for?)           │
│ • Configuration changes (what do they tune?)            │
│                                                          │
│ Storage: ~/logs/behavior.jsonl (if instrumented)        │
│ Format: JSONL (event stream)                            │
└─────────────────────────────────────────────────────────┘
```

---

## LAYER 2: AGGREGATION (Askesis Data Intake)

```
┌─────────────────────────────────────────────────────────┐
│ ASKESIS LAYER (Data Aggregator + Pattern Detector)      │
│                                                          │
│ Ingest from:                                            │
│  • SurrealDB (query /observations, /events)             │
│  • Local files (node logs, organ logs, session logs)    │
│  • Remote (Tailscale: node-N:/home/user/logs/)          │
│  • Stream (K15 /observe endpoint, SSE)                  │
│                                                          │
│ Output datasets:                                        │
│  • kernel_decisions.json (what Judge chose + why)       │
│  • node_executions.json (what nodes did, latency)       │
│  • agent_reasoning.json (task reasoning chains)         │
│  • human_strategy.json (session-level decisions)        │
│  • failures.json (all error events, recovery)           │
│  • hardware_capacity.json (GPU/CPU timeline)            │
│                                                          │
│ Askesis processes these into:                           │
│  • Pattern files (heuristics.md, dogs_agreement.json)   │
│  • Reflection (weekly-reflection.md)                    │
│  • Crystals (for kernel injection)                      │
│  • Metrics (compliance trend, pattern velocity)         │
└─────────────────────────────────────────────────────────┘
```

---

## LAYER 3: CONSUMPTION (Feedback Loops)

```
CONSUMERS OF DATASETS:

┌──────────────────────────────┐
│ KERNEL (domain logic)        │
│                              │
│ • Load heuristics.json       │
│ • Update Dogs (token_gates)  │
│ • Inject crystals            │
│ • React to failures.json     │
└──────────────────────────────┘

┌──────────────────────────────┐
│ SOMA ORCHESTRATOR            │
│                              │
│ • Read hardware_capacity     │
│ • Decide model allocation    │
│ • Observe failures           │
│ • Adjust strategy            │
└──────────────────────────────┘

┌──────────────────────────────┐
│ ORGANS (agents)              │
│                              │
│ • Read human_strategy        │
│ • Learn from failures.json   │
│ • Auto-evolve SKILL.md       │
│ • Report results back        │
└──────────────────────────────┘

┌──────────────────────────────┐
│ GEMINI (meta-learning)       │
│                              │
│ • Analyze reasoning chains   │
│ • Propose algorithm changes  │
│ • Synthesize wisdom          │
│ • Generate learning tasks    │
└──────────────────────────────┘

┌──────────────────────────────┐
│ HUMAN (via UI/reflection)    │
│                              │
│ • View weekly-reflection.md  │
│ • See pattern emergence      │
│ • Make strategic decisions   │
│ • Authorize system changes   │
└──────────────────────────────┘
```

---

## The Datasets (What Must Be Produced)

### Core Datasets (Phase 1)

| Dataset | Source | Schema | Consumer | Frequency |
|---------|--------|--------|----------|-----------|
| `kernel_decisions.jsonl` | /observe endpoint | {timestamp, domain, verdict, q_score, dogs_used, why} | Askesis, Gemini | Real-time |
| `node_executions.jsonl` | cynic-node logs | {timestamp, node_id, command, duration_ms, hardware, result} | Soma, Askesis | Real-time |
| `agent_tasks.jsonl` | agent_tasks table | {task_id, domain, status, reasoning, outcome, time_elapsed} | Organs, Askesis | Real-time |
| `failures.jsonl` | Kernel + node + agent logs | {timestamp, type, context, recovery, was_successful} | Soma, Askesis | Real-time |

### Aggregated Datasets (Phase 2)

| Dataset | Built by | Purpose | Consumers |
|---------|----------|---------|-----------|
| `heuristics.json` | Askesis + human | Token gates, wallet patterns, etc. | Kernel (load at boot) |
| `dogs_agreement.json` | Askesis | Which Dogs agree? Where diverge? | Kernel (quality metrics) |
| `hardware_capacity.json` | Askesis | GPU memory over time, CPU patterns | Soma (allocation decisions) |
| `human_strategy.json` | Askesis (from session logs) | What strategies work? Learning curves? | Organs, Gemini |
| `failure_modes.jsonl` | Askesis | Categories of failures, recovery success | Soma (fallback routing) |

### Wisdom Outputs (Phase 3)

| Output | Built by | Purpose |
|--------|----------|---------|
| `weekly-reflection.md` | Askesis | Pattern summary for human review |
| `gemini_learning_tasks.json` | Askesis + Gemini | "Try this: would it improve Dogs?" |
| `crystals_to_inject.json` | Askesis | High-confidence patterns → kernel |
| `metrics_dashboard.json` | Askesis | Compliance trend, pattern velocity |

---

## How Claude Code Sessions Feed In

**Location:** `~/.claude/projects/-home-user-Bureau-CYNIC/{session-id}.jsonl`

**Extraction (Askesis must parse):**

```rust
// Pseudocode: what Askesis extracts from each session
struct SessionLog {
    session_id: String,
    date: Date,
    turns: Vec<Turn>,
}

struct Turn {
    turn_num: usize,
    user_intent: String,       // "debug heartbeat issue", "refactor Dogs"
    claude_analysis: String,   // reasoning shown to user
    tools_used: Vec<String>,   // [Read, Grep, Edit, Bash]
    code_changed: bool,
    decision: String,          // "use RosterPort trait for Phase D"
    outcome: String,           // "refactor complete", "blocked on SurrealDB"
}
```

**What patterns emerge:**

- **Debugging cycle length:** How long does it take to go from error → root cause → fix?
- **Tool selection:** Which tools do humans use for which tasks? (Read → Grep → Edit = investigation pattern)
- **Decision confidence:** Do humans second-guess decisions? (multiple turns on same problem = uncertainty signal)
- **Blocked states:** What unblocks a session? (external data? feedback? new insight?)
- **Code change velocity:** Is velocity increasing or decreasing? (indicator of system health)

**Integration:**

```rust
// Askesis needs a LogStore impl for Claude Code sessions
pub struct ClaudeCodeLogStore {
    session_dir: PathBuf,  // ~/.claude/projects/-home-user-Bureau-CYNIC/
}

#[async_trait]
impl LogStore for ClaudeCodeLogStore {
    async fn read(&self, start: DateTime, end: DateTime) -> Result<Vec<LogEntry>> {
        // Parse all .jsonl files in session_dir
        // Filter by turn timestamp
        // Return as LogEntry structs
    }
    
    async fn write(&self, entry: LogEntry) -> Result<()> {
        // Not applicable (read-only from human perspective)
    }
}
```

---

## Data Flow Diagram (Complete)

```
SESSION LOGS            KERNEL              NODES              ORGANS
(strategy)              (decisions)         (execution)        (agents)
    │                       │                   │                  │
    ├─────────────────────┬─┴───────────┬───────┴───┬──────────────┤
    │                     │             │           │              │
    ▼                     ▼             ▼           ▼              ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │                                                                   │
  │                    ASKESIS DATA AGGREGATOR                       │
  │                                                                   │
  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
  │  │ LogStore:    │  │ LogStore:    │  │ LogStore:    │           │
  │  │ SurrealDB    │  │ Local Files  │  │ ClaudeCode   │           │
  │  │ (queries)    │  │ (tail)       │  │ (parse)      │           │
  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘           │
  │         │                 │                 │                   │
  │         └─────────────────┼─────────────────┘                   │
  │                           │                                     │
  │                    AuditEngine                                  │
  │              (pattern extraction)                               │
  │                           │                                     │
  │         ┌─────────────────┼─────────────────┐                  │
  │         │                 │                 │                  │
  │    ┌────▼─────┐   ┌──────▼──────┐   ┌─────▼──────┐            │
  │    │ Decision  │   │ Failure     │   │ Human      │            │
  │    │ patterns  │   │ modes       │   │ strategy   │            │
  │    └────┬─────┘   └──────┬──────┘   └─────┬──────┘            │
  │         │                │                │                    │
  │         └────────────────┼────────────────┘                    │
  │                          │                                     │
  │                   Reflection                                   │
  │              (weekly-reflection.md)                            │
  │                                                                   │
  └───────────────────────────┬───────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
          ┌────────┐    ┌──────────┐    ┌─────────┐
          │ KERNEL │    │ SOMA     │    │ HUMAN   │
          │ (load  │    │ (decide  │    │ (read & │
          │ gates) │    │ alloc)   │    │ reflect)│
          └────────┘    └──────────┘    └─────────┘
```

---

## Phase Implementation

### Phase 1 (Now → May 10)

**Goal:** Get one data pipeline working end-to-end

**Path:** Session logs → Askesis → human reflection

```bash
# Pseudocode
1. Askesis reads ~/.claude/projects/*/sessions.jsonl
2. Parses decision points
3. Outputs: "Most common failure pattern: GPU memory"
4. Human reads, validates
5. Human updates heuristics.json
6. Kernel loads heuristics.json at next boot
```

**No blocker.** Data already exists.

### Phase 2 (May 10 → June)

**Goal:** Wire all data sources

**Path:** Kernel observations + node logs + agent tasks → Askesis → datasets

**Blockers:**
- Node logs need a schema (must exist for cynic-node to emit)
- Askesis needs ClaudeCodeLogStore + SurrealDB reader (new LogStore impls)
- Failure taxonomy must be codified (what counts as "failure"?)

### Phase 3 (June+)

**Goal:** Feedback loops close

**Path:** Datasets → Kernel (heuristics), Soma (allocation), Organs (learning)

**Blocker:** Gemini learning loop (analyzing reasoning chains requires LLM)

---

## Why This Is Data-Centric

**Current assumption (wrong):**
- Kernel produces intelligence → humans trust it → system works

**Data-centric assumption (honest):**
- System produces datasets → humans see patterns → humans decide → system learns
- Intelligence is not in the kernel. Intelligence is in the human's reflection on datasets.

**The organism is successful when:**
- Datasets are rich and queryable
- Patterns emerge that nobody predicted
- Human reflection improves with each cycle
- System changes follow from human-validated patterns, not from confident inference

---

## Falsification

**Phase 1 is falsified if:**
- Session logs are parsed but not reflected (data is collected, not consumed)
- Askesis produces reflection but humans don't act on it (feedback loop broken)
- Datasets exist but aren't shared with consuming systems (pipeline incomplete)

**Phase 2 is falsified if:**
- Node logs aren't queryable alongside kernel logs (data is siloed)
- Failure patterns aren't aggregated (each failure treated as isolated)
- Hardware capacity isn't used to inform Soma decisions (data doesn't drive decisions)

**Phase 3 is falsified if:**
- Datasets don't change system behavior (data is decorative)
- Learning loop is one-way (humans read, don't write back)
- Gemini sees logs but can't propose changes (learning is passive)

---

## Next Steps

1. **Define log schemas** for cynic-node (hardware, commands, health)
2. **Implement LogStore** for Claude Code sessions (parse ~/.claude/projects)
3. **Wire Askesis intake** (SurrealDB reader + JSONL readers)
4. **Build one feedback loop** (session logs → reflection → human reads)
5. **Measure:** Does human behavior change after seeing Askesis output?
