# Inference Organ Roadmap — 5-Session Design

> Crystallized 2026-04-12 from research + production data + cynic-judge + crystallize-truth.

## Context

CYNIC's inference pipeline has a systemic positivity bias. 47% tier match on benchmark, 15/16 misclassifications are 1 tier too high. Root cause is structural, not prompt-level.

## Crystallized Truths (φ-bounded)

| T# | Truth | Conf. | Design Impact |
|----|-------|-------|---------------|
| T1 | Qwen positivity bias is structural — 46% rubric sensitivity (BiGGen Bench) + 0.95-0.99 raw on Scholar's Mate | 58% | Replace Qwen Dogs, don't fix |
| T1a | Prometheus-2 7B fits CYNIC's rubric paradigm. Skywork-Critic doesn't (pairwise ≠ absolute scoring) | 52% | Prioritize Prometheus-2, budget prompt adapter |
| T2 | CoT-before-score improves calibration for non-thinking Dogs | 48% | Reorder prompt: reasoning → JSON |
| T3 | Benchmark GT is the hidden bottleneck — LLM-derived GT = calibrating to noise | 55% | Stockfish/Lichess GT before heavy Dog optimization |
| T4 | Per-Dog per-axis weighting underpowered at 31 stimuli | 50% | Expand to 100+ before weighting |
| T5 | Minority veto requires explicit abstention tracking | 45% | Add abstention flag to DogScore first |
| T6 | Prompt fixes are cheap + measurable but necessary-not-sufficient | 52% | Do first as baseline, not as the fix |
| T7 | S.'s machine is at home — physical access exists as fallback | — | Sovereignty concern downgraded |
| T8 | Latency ignored in calibration plan | 42% | Measure latency alongside accuracy everywhere |

## Per-Dog Diagnosis (Scholar's Mate, 2026-04-12)

| Dog | FID | PHI | VER | CUL | BURN | SOV | Raw range | Assessment |
|-----|-----|-----|-----|-----|------|-----|-----------|------------|
| deterministic | 0.31 | 0.62 | 0.41 | 0.31 | 0.60 | 0.40 | 0.31-0.62 | Correctly abstains, pulls down |
| qwen-7b-hf | 0.62* | 0.62* | 0.15 | 0.62* | 0.62* | 0.62* | raw 0.15-0.95 | Clamped at phi, only VERIFY ok |
| qwen35-9b-gpu | 0.62* | 0.62* | 0.62* | 0.62* | 0.62* | 0.62* | raw 0.90-0.99 | Worst. All axes clamped. |
| gemma-4b-core | 0.30 | 0.40 | 0.10 | 0.50 | 0.20 | 0.60 | 0.10-0.60 | Best. Understood substance vs form. |

## Session Map

```
S1 (Prompt Calibration + GT)
 │  produces: Stockfish GT, expanded benchmark (60+), prompt baseline
 │
S2 (Infra + Model Eval)
 │  consumes: benchmark + GT from S1
 │  produces: model candidates evaluated, fleet infra ready
 │
S3 (Model Swap)
 │  consumes: candidates from S2, benchmark from S1
 │  produces: new fleet, post-swap measurements
 │  GATE: tier match > 65% to proceed
 │
S4 (Organ SoC + Aggregation)
 │  consumes: all measurements S1-S3
 │  produces: organ architecture, per-Dog weighting, abstention tracking
 │
S5 (Deterministic-dog v2)
    consumes: abstention semantics from S4
    produces: domain-aware heuristics
```

## Session 1 — Prompt Calibration + GT Foundation

**Role:** Inference calibration engineer
**Protocol:** Scientific — one variable, before/after, measure accuracy + latency
**Deliverables:**
1. Stockfish GT for chess stimuli (break circular LLM-derived GT)
2. Prompt calibration (CoT-before-score, BARK few-shot, substance prefix)
3. Benchmark expansion to 60+ stimuli

## Session 2 — Fleet + Model Evaluation

**Role:** Infra + ML engineer
**Protocol:** Evaluate on expanded benchmark, latency budget per Dog
**Deliverables:**
1. Tailscale MCP: ts_deploy for daemon lifecycle
2. S.'s machine onboarded, llama-server installed
3. Prometheus-2 7B Q5, Gemma 3 12B QAT Q4 evaluated on benchmark

## Session 3 — Model Swap

**Role:** Fleet operator
**Protocol:** Replace → benchmark → decision gate
**Deliverables:**
1. Replace qwen35-9b-gpu with best candidate
2. Replace/keep qwen-7b-hf based on results
3. Full fleet benchmark with new Dogs

## Session 4 — Organ SoC + Aggregation

**Role:** Systems architect
**Protocol:** Architecture informed by S1-S3 empirical data
**Deliverables:**
1. Dog ≠ Backend ≠ Node separation
2. Per-Dog per-axis reliability weighting (requires 100+ benchmark stimuli)
3. Abstention tracking in DogScore
4. Minority veto (requires abstention)
5. DogStats persistence

## Session 5 — Deterministic-dog v2

**Role:** Heuristic engineer
**Protocol:** Domain-aware scoring with formal verification
**Deliverables:**
1. Domain-aware heuristic registry
2. Formal notation expansion
3. Abstention semantics formalization

## Research Sources

- Minority Veto Ensemble: arXiv 2510.11822 (Oct 2025)
- RULERS (Locked Rubrics): arXiv 2601.08654 (Jan 2026)
- Judge-Aware Ranking (IRT): arXiv 2601.21817 (Jan 2026)
- Prometheus-2: arXiv 2405.01535 (EMNLP 2024)
- Skywork-Critic: RewardBench #1 for <10B
- JudgeBoard (Qwen 46% corruption): AAAI 2025
- Phi-calibration: CYNIC internal (2026-04-06) — "clamp is honest, fix Dogs"
- Benchmark baseline: CYNIC internal (2026-04-11) — 47% tier, 6 gaps
