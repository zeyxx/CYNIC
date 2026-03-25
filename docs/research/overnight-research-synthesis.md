# Overnight Research — Cross-Cutting Synthesis

*Generated 2026-03-21 by research agent*

## New Star: joemccann/x-bookmarks-pipeline

**Architecture:** Single Rust crate, async tokio, bounded worker pool. Pipeline: Fetch → Classify → Vision → Plan → Generate → Validate. Per-stage SQLite cache by `(bookmark_id, stage)`.

### 5 Patterns Stealable for CYNIC

1. **Per-stage cost capture** — Every LLM call records `(prompt_tokens, completion_tokens, usd_cost)` per stage. CYNIC tracks cost at verdict level only. Per-axiom cost breakdown → empirical basis for sovereign-first routing (P4).

2. **Per-stage idempotent cache** — If stage N fails, stages 0..N-1 preserved. CYNIC's `judge.evaluate()` is all-or-nothing. Partial Dog failures discard all results. This pattern makes judgment **resumable**.

3. **Bounded concurrency as config** — `MAX_WORKERS` env + per-category timeouts. CYNIC fires all Dogs simultaneously with no concurrency budget. A Dog worker pool enables sequential sovereign-first dispatch.

4. **Failure degradation table** — browser.rs documents every CDP failure as `(scenario, behavior)`. Design artifact pattern worth copying for Dog failure documentation.

5. **Daemon mode with new-only processing** — CCM re-processes the full window each cycle. x-bookmarks-pipeline only processes NEW items, maintaining a processed-set across cycles.

---

## Cross-Cutting Analysis (97 Stars)

### 1. Convergent Patterns (3+ repos)
- Provider abstraction + cost matrix (5 repos)
- SQLite/local-first dominant (no cloud DB)
- Per-stage independent caching (4+ repos)
- Semantic similarity as cache key = table stakes
- MCP has won as coordination layer

### 2. Rust Ecosystem Momentum
- ~13/97 starred repos are Rust
- Rust dominates systems-level agent infra (openfang, agent-browser, rtk)
- TypeScript dominates agent frameworks by star count
- x-bookmarks-pipeline validates CYNIC's exact stack (tokio + reqwest + serde + clap)
- **No pure-Rust production inference runtime exists** — still runs through llama.cpp C++ bindings

### 3. Sovereignty Trend
- ~11-20/97 repos explicitly sovereignty-focused
- unsloth (56K) + whisper.cpp (48K) prove self-hosting is mainstream
- **CYNIC's unique position:** has BOTH sovereign backends AND emerging routing intelligence — no other single project has both

### 4. Cost Compression
CYNIC has all 4 strategies partially implemented:
- Token compression (rtk)
- Sovereign inference
- Semantic caching
- Intelligent routing

**The compound value is the combination.** No other starred repo has all four. P4 (wiring BackendRouter) is the missing link.

### 5. Gaps No Starred Repo Addresses
- **(a)** Axiom-level judgment calibration corpus — no repo has multi-dimensional epistemic scoring with ground-truth validation
- **(b)** Consensus protocol for heterogeneous judges — ClawRouter routes to ONE model, x-bookmarks-pipeline uses one model per stage. CYNIC's multi-Dog Byzantine-tolerant consensus is **architecturally unique**
- **(c)** Temporal epistemic state tracking — no repo tracks how a judgment evolves as evidence accumulates

### 6. Top 3 Multiplicative Compounds

1. **x-bookmarks-pipeline × CYNIC** — Add `/judge` calls at classification stage. Pipeline brings browser auth + vision + Pine Script; CYNIC brings judgment quality. Neither alone is the product.

2. **ClawRouter × CYNIC BackendRouter** — Steal ClawRouter's tier model (41 models, cost × quality × latency matrix), port routing algo to Rust BackendRouter. Sovereign-first routing without building calibration from scratch.

3. **contextplus × CYNIC** — Configure as MCP server against cynic-kernel/src. Cold start penalty eliminated, code change observations feed back into crystal retrieval.

---

## Top 5 Actions

1. **Wire BackendRouter** with ClawRouter-style tier model (P4, 1 session)
2. **Per-stage verdict cost tracking** from x-bookmarks-pipeline pattern (~2 hours)
3. **Install contextplus MCP** against cynic-kernel/ (30 min, zero kernel changes)
4. **Per-stage idempotent caching** for multi-Dog judgment (1 session + SurrealDB schema)
5. **Define axiom calibration corpus** — 50-100 stimuli with ground-truth scores per axiom, start with chess positions
