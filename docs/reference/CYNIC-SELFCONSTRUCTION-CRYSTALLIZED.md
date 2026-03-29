# CYNIC Self-Construction — Crystallized Truth

*Crystallized 2026-03-24. Two passes. Pass 1: architecture + levels. Pass 2: knowledge ingestion (invalidated Pass 1 Step 0).*

## Problem Statement

Can CYNIC help build itself via Claude Code, worktrees, and intelligent crystal injection? What's the actual architecture, what's assumption, and what compounds?

## The Measurement That Changed Everything

Before building anything, we measured the crystal corpus:

```
100 crystals. 2 domains. 83 sessions. 253 MB of raw conversation.
- 93 "workflow" crystals = co-occurrences ("file X + file Y co-edited")
- 7 "chess" crystals = actual knowledge (opening names)
- 80+ memory files (.md) = real curated knowledge, OUTSIDE the kernel
- Top 5 injected at session start = ALL co-occurrence noise
```

**The crystal system produces noise.** Not because the algorithm is wrong — CCM does exactly what it's designed to do. Because the OBSERVATIONS are thin: `{tool: "Edit", target: "main.rs", status: "ok"}` contains zero semantic content. No pipeline can extract knowledge from this.

**The memory file system produces value.** 80+ curated files with real decisions, patterns, learnings. But they're flat markdown outside the kernel — no semantic search, no temporal decay, no Dog evaluation, no lifecycle.

**Three memory systems, none connected:**

| System | Written by | Read by | Content | In kernel? |
|---|---|---|---|---|
| Memory files (.md) | Claude + Human | Claude (session start) | Real knowledge | No |
| Crystals (SurrealDB) | CCM automatic | Pipeline (injection) | Activity noise | Yes |
| Sessions (JSONL) | Claude Code | Nobody | Everything (253 MB) | No |

## Architecture Decision

**CYNIC is a judgment LAYER, not a workflow CONTROLLER.**

Orchestration lives in skills (markdown). The kernel provides endpoints. Claude Code decides.

## The Reframe: Channel, Not Pipeline

Research into production memory systems (Mem0, Mastra, Letta, Aura) revealed four patterns:

| Pattern | System | Write-time LLM | How it works |
|---|---|---|---|
| **A: Extract per write** | Mem0 | Yes (2+ calls/add) | LLM extracts facts → dedup against vector store → ADD/UPDATE/DELETE |
| **B: Batch threshold** | Mastra | Yes (at 30K token threshold) | Accumulate → Observer summarizes → Reflector condenses |
| **C: Agent self-edit** | Letta/MemGPT | Agent IS the LLM | Agent decides what to remember via tool calls |
| **D: Statistical** | Aura, CYNIC CCM | No | Co-occurrences, frequency, sub-ms. No knowledge. |

**Key insight from Letta:** The agent already HAS the knowledge in its context. It doesn't need extraction — it needs a way to EMIT what it learned. CYNIC's case is different from Mem0/Mastra: the knowledge source IS the agent itself.

**The problem is not "build a pipeline" — it's "build a channel."**

Claude Code hooks can only see tool results (not reasoning). Only the agent has access to WHY decisions were made. Therefore:
- Pattern A (extract per write): impossible — observations don't contain knowledge
- Pattern B (batch threshold): impossible — summarizing thin data produces thin summaries
- Pattern C (agent self-edit): **the only viable path** — agent emits knowledge directly
- Pattern D (statistical): keep for workflow patterns, not for knowledge

## What Must Change in the Kernel

### 1. `cynic_learn` MCP tool (NEW — the channel)

```
cynic_learn({
    domain: "architecture",
    content: "Dogs can't judge code diffs — tested, Q-scores don't separate good from bad",
    confidence: 0.55,
    source: "session-2026-03-24",    // audit trail
    reasoning: "Tested 5 good + 5 bad diffs, no Q-score separation"  // optional WHY
})
```

This creates a crystal observation with `crystal_type: "knowledge"`. Enters the same lifecycle as activity crystals but through a richer path.

### 2. `crystal_type` field (SCHEMA — distinguish noise from knowledge)

```
crystal_type: "co-occurrence" | "workflow" | "knowledge"
```

Same lifecycle (Forming → Crystallized → Canonical → Decaying → Dissolved). Different entry thresholds. Different retrieval priority.

### 3. Differentiated crystallization threshold

| Type | Threshold to Crystallize | Rationale |
|---|---|---|
| co-occurrence | 21 observations (Fib F8) | Statistical patterns need many data points |
| workflow | 21 observations (Fib F8) | Same |
| knowledge | 3 confirmations (Fib F3) | Explicit learnings from independent sessions |

Knowledge crystals maintain φ-bounded confidence (max 0.618) and temporal decay. The lifecycle handles quality: unconfirmed learnings decay, confirmed ones crystallize.

### 4. `/distill` skill → calls `cynic_learn` (CONNECT — not separate systems)

Currently `/distill` writes memory files. Updated: `/distill` writes memory files AND calls `cynic_learn` for each learning. The kernel and the filesystem stay in sync. Progressive transition: when knowledge crystals are rich enough, session-init injects them alongside (not instead of) memory files.

### 5. Memory file bootstrap (ONE-SHOT — seed the corpus)

Ingest the 80+ existing memory files as knowledge crystals with `state: Forming, confidence: 0.5`. The lifecycle handles the rest — future sessions that confirm a memory file's content will crystallize it. Those that contradict will let it decay.

## Three Independent Levels (Revised)

### Level 0 — Knowledge Channel (THE SEED)

**What:** Add `cynic_learn` MCP tool + `crystal_type` field + differentiated threshold.

**Why it's the seed:** Without knowledge in the crystal system, Levels 1 and 2 compound noise. With knowledge, every subsequent improvement (smart retrieval, pulse, self-construction) operates on real signal.

**Effort:** ~2-3h. 1 MCP tool (~50 lines) + schema field + CCM threshold change.

**KPI:** After 10 sessions using `/distill` → `cynic_learn`, count knowledge vs activity crystals. Success = knowledge crystals contain actual learnings, not activity metadata.

### Level 1 — Smart Crystal Retrieval

**What:** Replace `GET /crystals?limit=5` with `/crystals/relevant?query=...` (PageIndex pattern).

**Prerequisite:** Level 0 must produce knowledge crystals worth retrieving. Measure corpus first.

**Architecture:** Background domain index (refresh every 5 min) + sovereign ranking at query time. Filter by `crystal_type = "knowledge"` for session injection.

### Level 2 — Mid-Session Feedback (cynic_pulse)

**What:** MCP tool returning delta crystals + anomalies since last call.

**Prerequisite:** Event bus must have internal consumers.

### Level 3 — Worktree Self-Construction

**What:** Skill orchestrates worktrees, CYNIC judges diffs, human approves merges.

**Prerequisites:** Code quality benchmark (Dogs separate good/bad diffs) + Levels 0-1 working.

## Truth Table (Revised)

| T# | Truth | φ⁻¹ | Design Impact |
|---|---|---|---|
| **T0** | **The crystal corpus is structurally noise because observations are thin.** `{tool, target, status}` contains zero semantic content. No retrieval algorithm fixes this. | 61% | The FIRST fix is richer input (knowledge channel), not smarter output (retrieval). Invalidates previous Step 0. |
| T0a | Hooks cannot access Claude's reasoning — only tool results. Knowledge extraction MUST come from the agent, not the hook pipeline. | 61% | Pattern C (agent self-edit) is the only viable path. Patterns A and B fail on thin data. |
| **T1** | **CYNIC needs a channel, not a pipeline.** The agent already has the knowledge — it needs to EMIT it, not have it EXTRACTED. `cynic_learn` is a ~50 line MCP tool, not a rewrite. | 58% | Add `cynic_learn` MCP tool. Don't build extraction pipeline. Don't parse session JSONLs. |
| T1a | Memory files and crystals are parallel systems that should converge. Memory files are the seed; crystals are the scalable store with lifecycle + search. | 55% | Bootstrap: ingest memory files → Forming crystals. Ongoing: `/distill` feeds both systems. |
| **T2** | **Activity patterns and knowledge are fundamentally different types.** One is statistical (co-occurrences), the other is semantic (learnings). One is not a degraded version of the other. | 58% | `crystal_type` field. Same lifecycle, different entry paths, different retrieval priority. |
| T2a | Knowledge crystals need faster crystallization (3 confirmations vs 21) because explicit learnings happen once per session, not 100x. | 52% | Fib F3 threshold for knowledge, F8 for activity. φ-bounded confidence + decay = poison protection. |
| **T3** | **CYNIC is a judgment layer, not a workflow controller.** Orchestration in skills, endpoints in kernel, decisions by Claude Code. | 58% | No worktree orchestration in kernel. Skills consume kernel services. |
| **T4** | **Each level proves independent value.** Level 0 (channel) stands alone. Don't pitch as "self-construction loop." | 58% | Each level has own KPI. No level depends on another for value. |
| **T5** | **Human is the quality signal at merge points.** Autonomous to explore, human to approve. | 55% | Worktree orchestrator proposes, never merges alone. |
| **T6** | **Opportunity cost: fix audit RCs alongside, not instead.** The kernel BARKs itself (Q=0.22). But the knowledge channel is small (~2-3h) and compounds everything. RC fixes and Level 0 can proceed in parallel. | 55% | Level 0 + RC fixes in parallel, not sequential. |
| **T7** | **The Supermemory insight (agentic > vector) is validated but misapplied by them.** 1 sovereign call, not 18. Agent emits knowledge (Pattern C), sovereign ranks at retrieval (Level 1). | 55% | Combine Pattern C (agent emits) + PageIndex (sovereign retrieves). Total: 1 LLM write + 1 LLM read. |

## Compound Execution Order (Revised)

```
STEP 0: cynic_learn MCP tool + crystal_type + threshold (2-3h)
    ↓ the crystal system can now receive knowledge
STEP 1: Update /distill skill to call cynic_learn (30 min)
    ↓ every distilled session feeds the kernel
STEP 2: Bootstrap memory files → Forming crystals (1h, one-shot)
    ↓ 80+ learnings seeded into the crystal system
STEP 3: Fix audit RCs (RC1, RC5, RC3) — in parallel with Steps 0-2
    ↓ reliable kernel
STEP 4: Measure knowledge crystal corpus (after 10+ sessions)
    ↓ determines if Level 1 (smart retrieval) is justified
STEP 5: Level 1 — /crystals/relevant with crystal_type filter
    ↓ sessions start with relevant knowledge, not noise
STEP 6: Code quality benchmark (5 good + 5 bad diffs)
    ↓ proves or disproves Dog separation on code
STEP 7: Level 2-3 — pulse + worktree orchestration (IF benchmarks pass)
```

## Deep Research Findings (3 parallel agents)

### Finding: `/observe` → crystals is separated BY DESIGN

`ccm.rs:aggregate_observations` explicitly states:
> "Workflow patterns are ANALYTICS, not epistemic memory. They are NOT fed into the crystal system because: (1) frequency ratios can NEVER reach crystallization threshold, (2) content is tool usage logs, not knowledge, (3) mixing operational telemetry with judgment crystals degrades prompt injection quality."

Crystals are created ONLY by the judge pipeline (Dog evaluation → Q-score → observe_crystal). `cynic_learn` would be the FIRST pathway from observation → crystal for knowledge.

### Finding: Academic consensus is "push LLM to write time"

| System | Write LLM calls | Read LLM calls | Contradiction |
|---|---|---|---|
| Mem0 vector | 2 | 0 | LLM judges ADD/UPDATE/DELETE |
| Graphiti/Zep | 4-5 | 0 | Temporal soft-delete with t_invalid |
| A-MEM | 3 | 0 | In-place mutation + graph propagation |
| Chronos | 1 | 1 | Temporal ordering at read time |
| Mastra OM | 1 (compress) | 0 | None (prose compression) |
| Letta/MemGPT | 0 (agent decides) | 0 | Agent's responsibility |
| **CYNIC current** | **0** | **0** | **None** |

CYNIC is at the bottom: 0 write, 0 read = nothing extracted, nothing retrieved.

Pattern C (Letta) is the cheapest: the agent already pays 1 LLM call/turn. Memory management layers on top for free. The CASS Memory pattern (Reflector → Validator → Curator) maps to CYNIC: **Agent extracts → Dogs validate → CCM deduplicates deterministically.**

### Finding: SessionSummary already exists — unwired

`SessionSummary` struct, `format_summarization_prompt()`, `SovereignSummarizer`, `format_session_context()` — all exist in the kernel. They are simply not connected to any trigger. Wiring `session-stop.sh → POST /summarize-session` is ~30 lines of kernel code + 1 hook line.

### Finding: Grammar-constrained generation = reliable 4B extraction

llama.cpp `--grammar` parameter achieves 100% JSON compliance regardless of model size. Gemma 4B + grammar constraint = structured extraction without format errors. The quality concern is semantic (does it extract the RIGHT things), not structural (is the JSON valid).

### Finding: Contradiction handling is an unsolved Layer 1+ problem

Three strategies in the literature:
- **Mem0**: LLM adjudicates ADD/UPDATE/DELETE at write time (expensive, 2 calls)
- **Graphiti/Zep**: Temporal soft-delete with `t_invalid` timestamp (preserves history, cheap)
- **Letta**: Agent's responsibility via `core_memory_replace` (fragile, contradictions accumulate)

For CYNIC Level 0: no contradiction handling. The lifecycle + decay is the first defense. For Level 1+: `invalidated_by: Option<CrystalId>` — temporal soft-delete à la Graphiti.

## Research Sources

### Memory System Architectures (primary)
- [Mem0 paper (arXiv)](https://arxiv.org/html/2504.19413v1) — Two-phase: extraction + update with LLM dedup
- [Mem0 DeepWiki](https://deepwiki.com/mem0ai/mem0/1-overview) — Architecture, graph memory, prompts
- [Mastra Observational Memory research](https://mastra.ai/research/observational-memory) — 94.87% LongMemEval, three-tier compression
- [Mastra docs](https://mastra.ai/docs/memory/observational-memory) — 30K token threshold, Observer + Reflector
- [Letta/MemGPT docs](https://docs.letta.com/concepts/memgpt/) — Agent self-editing, core/archival/recall memory
- [Letta memory management](https://docs.letta.com/advanced/memory-management/) — Tool calls: memory_insert, memory_replace
- [Mem0 vs Letta comparison](https://vectorize.io/articles/mem0-vs-letta) — Passive vs agentic tradeoffs
- [Aura Memory](https://dev.to/teolex2020/i-built-a-cognitive-layer-for-ai-agents-that-learns-without-llm-calls-33no) — Rust, no-LLM, SDR indexing, sub-ms
- [claude-mem](https://github.com/thedotmack/claude-mem) — PostToolUse → SDK Agent → compress → SQLite + ChromaDB
- [CASS Memory System](https://github.com/Dicklesworthstone/cass_memory_system/blob/main/PLAN_FOR_CASS_MEMORY_SYSTEM.md) — Reflector → Validator → Curator (zero-LLM dedup)
- [Graphiti/Zep paper (arXiv 2501.13956)](https://arxiv.org/html/2501.13956v1) — 4-5 write LLM calls, temporal soft-delete, RRF retrieval
- [A-MEM paper (arXiv 2502.12110)](https://arxiv.org/abs/2502.12110) — Zettelkasten-inspired, 3 write LLM calls, in-place mutation

### Supermemory ASMR Analysis
- [Chronos paper (arXiv 2603.16862)](https://arxiv.org/html/2603.16862) — 95.6%, dual-calendar, temporal normalization
- [LongMemEval](https://xiaowu0162.github.io/long-mem-eval/) — 500 questions, 5 abilities, ICLR 2025
- [Supermemory blog](https://blog.supermemory.ai/we-broke-the-frontier-in-agent-memory-introducing-99-sota-memory-system/) — "social experiment", "parody"
- [HN critique](https://news.ycombinator.com/item?id=46426762) — pass@8 methodology

### Small LLM Capabilities
- [LLMs under 7B (2026)](https://mljourney.com/best-open-source-llms-under-7b-parameters-run-locally-in-2026/) — Qwen3-4B matches 120B on 7/8 benchmarks
- [Best SLMs 2026](https://www.bentoml.com/blog/the-best-open-source-small-language-models) — 4B extraction benchmarks
- [Grammar-constrained generation](https://medium.com/@emrekaratas-ai/structured-output-generation-in-llms-json-schema-and-grammar-based-decoding-6a5c58b698a6) — 100% JSON compliance with GBNF

### Session Transcript Research
- [Claude Code session format](https://databunny.medium.com/inside-claude-code-the-session-file-format-and-how-to-inspect-it-b9998e66d56b)
- [Claude Code session continuation](https://blog.fsck.com/releases/2026/02/22/claude-code-session-continuation/)
- [GSD context monitor](https://github.com/gsd-build/get-shit-done/tree/main/hooks)
- [Temporal Semantic Memory (arXiv 2601.07468)](https://arxiv.org/html/2601.07468v1) — dialogue/semantic time split

## What Was Systematically Ignored

1. **Cost of transition** — during migration, knowledge is fragmented across memory files AND crystals. Mitigated: memory files keep working, crystals ADD, never REPLACE.
2. **Agent discipline** — will Claude Code actually call `/distill` → `cynic_learn`? Currently voluntary. Enforcement requires hook or skill discipline, not kernel logic.
3. **4B LLM quality for retrieval ranking** — Gemma 4B doing PageIndex-style reasoning is assumed adequate. Unverified for crystal domains.
4. **Contradiction handling** — deferred to Level 1+. Level 0 uses lifecycle decay only. Graphiti's `t_invalid` pattern is the researched solution for later.
5. **Session summarization path** — `SessionSummary` exists unwired. Could be a quick win alongside `cynic_learn` but is a DIFFERENT concern (context compression vs knowledge crystallization).

## Falsification Conditions

- **T0 falsified if:** enriching observations (not via agent) produces useful crystals (e.g., if context field in current observations contains enough signal)
- **T0 reinforced by:** `ccm.rs` explicitly states observations don't feed crystals. The code agrees with the truth.
- **T1 falsified if:** `cynic_learn` is called regularly but crystals remain noisy (the channel works but the content is bad)
- **T2 falsified if:** a unified threshold works equally well for both types (no differentiation needed)
- **T2a falsified if:** knowledge crystals with F3 threshold produce poison that degrades session quality
- **Vision falsified if:** after 20 sessions with `cynic_learn`, knowledge crystals add no measurable value over memory files alone
