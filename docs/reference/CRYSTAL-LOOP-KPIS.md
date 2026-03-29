# Crystal Loop — KPIs & Objectives

*2026-03-21 — Rigorously defined before any implementation*

## The Question

Does the crystal feedback loop actually work? Today: 0 mature crystals, 0 evidence of learning.

## 5 KPIs

### KPI 1: Crystal Maturation Rate
**What:** Number of crystals reaching `Crystallized` state per week.
**Baseline:** 0 (current — the loop is structurally inert)
**Target:** ≥ 5 per week with real data flowing
**Measurement:** `SELECT count() FROM crystal WHERE state = 'crystallized' AND updated_at > time::now() - 7d`
**Why it matters:** If we connect sources and nothing crystallizes, the architecture is wrong — not just the calibration.

### KPI 2: Verdict Quality Delta (Δ)
**What:** Q-Score difference WITH crystal injection vs WITHOUT.
**Baseline:** Unknown (never A/B tested — crystals have never been injected)
**Target:** Δ > 0.05 on same stimuli (5% improvement, statistically significant)
**Measurement:** Judge the same 30 stimuli with crystals ON (production) vs OFF (flag/endpoint). Compare distributions.
**Why it matters:** The ONLY KPI that proves crystals IMPROVE judgment, not just that they exist. Without this, everything else is decoration.

### KPI 3: Loop Closure Time
**What:** Time from observation/data ingestion to crystal influence on a verdict.
**Baseline:** ∞ (the loop has never closed)
**Target:** < 1 hour for high-signal data (memory files, commits). < 24h for transcripts.
**Measurement:** Timestamp chain: `observation.created_at → crystal.updated_at → verdict.created_at` where the verdict's crystal_context includes the crystal.
**Why it matters:** A loop that takes weeks to close is not learning — it's archiving.

### KPI 4: Crystal Regression Rate
**What:** Percentage of verdicts where injected crystals DECREASE Q-Score vs no-crystal baseline.
**Baseline:** 0% (no crystals injected yet)
**Target:** < 10% (some regression is expected from noise; >10% = crystal poison)
**Measurement:** For each verdict with crystal context: compare Q-Score to same stimulus without crystals. Count regressions.
**Why it matters:** A crystal that degrades judgment is worse than no crystal. This is the safety check. Crystal poison can compound — one bad crystal influences future crystals.

### KPI 5: Signal Yield
**What:** Percentage of ingested data that contributes to at least 1 Crystallized crystal.
**Baseline:** ~0% (workflow observations produce only Decaying crystals)
**Target:** > 20% for high-signal sources (memory files, commits). > 5% for low-signal (transcripts).
**Measurement:** Track provenance: which observation/data source contributed to which crystal. `source_count_contributing / total_source_count`.
**Why it matters:** If we ingest 1000 observations and 0 contribute to crystals, the ingestion pipeline is noise. Signal yield tells us whether each data source is worth the cost.

## Objective Graph (Target State)

```
Data Sources (7)           Filter           Crystal Loop           Output
─────────────              ──────           ────────────           ──────
Memory files ──────────►  [direct]  ──►
Git commits  ──────────►  [parse]   ──►
Docs         ──────────►  [direct]  ──►    observe_crystal()
Transcripts  ──────────►  [LLM]    ──►    → running mean          format_crystal_context()
Human feedback ────────►  [hook]    ──►    → state machine         → Dog prompts
Dog reasoning ─────────►  [extract] ──►    → Forming               → enriched verdicts
Workflow obs  ─────────►  [signal]  ──►      → Crystallized
                                             → Canonical
                                             → Decaying

Each pipe has its own filter. Each filter has its own signal yield.
The graph models relationships: commit CAUSES memory update CAUSES doc update.
```

## Measurement Infrastructure Required

1. **A/B test endpoint** — `/judge` with `?crystals=off` parameter to disable injection
2. **Provenance tracking** — each crystal stores which data sources contributed
3. **Temporal chain** — observation_id → crystal_id → verdict_id linking
4. **Weekly dashboard** — 5 KPIs automated (can be a simple script + /status enhancement)

## What We Don't Know Yet

- What the right crystallization threshold is (0.618 is structurally unreachable with raw Q-Scores, normalization is a band-aid, TrueSkill μ+σ is promising but untested with real data)
- Whether sovereign LLM (Gemma 3 4B on CPU) can extract signal from transcripts efficiently
- Whether crystal injection actually changes Dog behavior or gets ignored (the RAG context injection problem)
- What the computational cost of full transcript ingestion is
- Whether the graph model (sources as nodes, transformations as edges) adds value over simple per-source pipes
