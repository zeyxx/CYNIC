# CYNIC Ingestion Design — How to Feed the Organism

*2026-03-21 — Crystallized from 3-layer analysis + external research (7 patterns)*

## The Problem

CYNIC's crystal feedback loop has NEVER produced a mature crystal. 20 crystals exist, all Decaying, all "Read main.rs — 158x observed." Meanwhile, 326 MB of rich data (transcripts, memory, git, docs) passes entirely outside the system.

The loop is starving — not because the math is wrong, but because it receives almost nothing meaningful to eat.

## Truths (Crystallized)

| T# | Truth | Conf. | Impact |
|----|-------|-------|--------|
| T1 | The crystal loop is an untested hypothesis. 0 mature crystals, 0 evidence of improvement. Must be validated before any pipe is built. | 58% | SEED + TEST before anything else |
| T2 | Ingestion without filtering produces crystallized noise ("Read main.rs"). Filter BEFORE ingestion is mandatory. | 55% | Each source needs its own signal extractor |
| T3 | Signal density varies 100x across sources. Memory (90%, 452K) ≫ Commits (70%, 8.7KB) ≫ Transcripts (10%, 326MB). Cost is inversely proportional. | 52% | Connect high-density first, low-density last |
| T4 | KPI 2 (verdict quality delta WITH vs WITHOUT crystals) is existential. Without it, everything is vanity metrics. | 60% | A/B test endpoint (`/judge?crystals=off`) is prerequisite |
| T5 | The 7 sources form a causal graph (commit → memory → doc). Start with simple pipes + dedup. Build graph when provenance data justifies it. | 48% | Pipes first, graph later. Provenance tracking from day 1. |
| T6 | Computational asymmetry: memory/commits = seconds (structured text). Transcripts = hours (LLM on CPU). Cost drives implementation order. | 55% | Zero-LLM sources first. LLM sources when GPU is reliable. |

## 7 Patterns to Steal (from research)

| Pattern | Source | What CYNIC takes |
|---------|--------|-----------------|
| **Schema-as-noise-filter** | claude-mem | Output schema has no field for noise. If LLM can't fit it into `{facts, concepts, files}`, it's discarded. |
| **Common identifier** | OpenTelemetry | Stamp every event with `session_id`. Correlation across sources via JOIN, not schema unification. |
| **Tail-based sampling** | OTel | Don't decide what to keep at ingestion. Buffer, observe outcomes, keep events that led to HOWL verdicts. |
| **Shared embedding space** | CLIP/multi-modal AI | Sources don't need schema unification — they need projection to common vector space. |
| **Single-pass DAG** | Hercules git mining | One event stream, 7 analyzers. Each commit read once, transforms shared. |
| **Correction-weighted extraction** | MemGPT/LangMem | User corrections ("actually…", "no that's wrong") are 10x higher signal. Detect and weight them. |
| **Hierarchical summarization** | GraphRAG/RAPTOR | Raw events → session summaries → themes → crystals. Each level discards ~90% but preserves structure. |

**Meta-finding:** Every system uses the same trick — don't unify raw data, unify the representation. The fusion layer is always a common ID, shared embedding space, or structured schema. Raw data stays heterogeneous.

## 5 KPIs (Minimum Measurement)

1. **Crystal Maturation Rate** — crystals reaching Crystallized per week. Baseline: 0. Target: ≥5.
2. **Verdict Quality Delta** — Q-Score WITH crystals vs WITHOUT. Target: Δ > 0.05. **EXISTENTIAL.**
3. **Loop Closure Time** — observation → crystal influence on verdict. Baseline: ∞. Target: < 1h.
4. **Crystal Regression Rate** — verdicts where crystals DECREASE quality. Target: < 10%.
5. **Signal Yield** — % of ingested data contributing to ≥1 Crystallized crystal. Target: >20% for high-signal.

## Implementation Order

```
Step 0: SEED + TEST (10 min, blocks everything else)
  Insert 1 crystal manually → judge → verify injection → measure KPI 2
  If Δ ≤ 0: STOP. The loop doesn't work. Rethink.
  If Δ > 0: CONTINUE.

Step 1: MEMORY FILES (1 day, zero LLM)
  98 memory/*.md → parse markdown → 1 crystal per file
  Domain = memory type. Confidence = 1.0 (human-validated).
  Measure: KPI 1, KPI 5

Step 2: GIT COMMITS (1 day, zero LLM)
  122 commits → parse type(scope): description → observe_crystal
  Churn patterns → co-change coupling → workflow crystals
  Measure: KPI 1, KPI 3, KPI 5

Step 3: FORMAL A/B TEST (2 days)
  30 diverse stimuli (chess + code + trading)
  /judge?crystals=on vs /judge?crystals=off
  Measure: KPI 2 (Δ), KPI 4 (regression rate)
  GO/NO-GO decision on loop validity

Step 4: DOCS CRYSTALLIZED TRUTHS (1 day, zero LLM)
  12 docs/CYNIC-*.md → 1 crystal per truth statement
  Only if Step 3 validates the loop

Step 5: SESSION SUMMARIES ENRICHMENT (existing, improve)
  Already built — sovereign LLM summarizes sessions
  Improve: richer observation context in PostToolUse hook
  Needs: reliable sovereign LLM (S. GPU)

Step 6: TRANSCRIPT DIGESTION (future, needs GPU)
  391 transcripts → LLM-compressed observations
  Schema-as-noise-filter (claude-mem pattern)
  Correction-weighted extraction for user feedback
  Computational cost: hours on CPU, minutes on GPU
  Only when: Steps 1-5 validated, GPU reliable

Step 7: DOG REASONING RE-INGESTION (future, investigate)
  Currently 0 chars stored — verify why
  If Dogs DO produce reasoning: extract → observe_crystal
  If not: this gap doesn't exist
```

## The Score ≠ Confidence Problem

The crystallization threshold (0.618) is structurally unreachable because Q-Scores (max ~0.57) are used as confidence values. Two approaches explored:

**Normalization** (implemented, commit de66bd8): `score / φ⁻¹` maps [0, 0.618] → [0, 1.0]. Simple but conceptually a band-aid — score still equals confidence.

**TrueSkill-inspired μ+σ** (experimented, not implemented): Each crystal has μ (mean score) and σ (uncertainty). Confidence = μ - 2σ (conservative lower bound). σ decreases with observations. Correctly handles: consistent good (crystallizes), consistent bad (doesn't), oscillating (doesn't). Requires schema change (add σ field to Crystal).

**Decision needed:** Validate with Step 0 first. If normalization works empirically (KPI 2 positive), keep it. If not, implement μ+σ.

## Gap 8: Binary Version Drift (discovered during session)

The running kernel version (`/health → version`) can silently diverge from `git describe --dirty` on HEAD. Two sessions deploying independently = last-deployer-wins with no alert. The binary that was running during this session was 10+ commits behind HEAD.

This is the same observability debt: CYNIC doesn't monitor itself. The fix belongs in the health loop or the session-init hook: compare running version to git HEAD, alert if divergent.

## What This Document Doesn't Cover

- WebSocket /ws event bus design (see PARALLEL-SESSIONS-CRYSTALLIZED.md)
- `cynic run` launcher design (de-prioritized — sugar, not foundation)
- OpenClaw integration (blocked on loop validation)
- IPv6 multi-node architecture (see PARALLEL-SESSIONS-CRYSTALLIZED.md)
- The organic development workflow (ILC protocol — see CYNIC-DIAGNOSTIC doc)
