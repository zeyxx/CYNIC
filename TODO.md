# CYNIC — Strategic Roadmap

> **Tactical work lives in the mempool** (`POST /observe domain=mempool`).
> This file tracks phases and milestones only — the "what and why", not the "how".
> Session agenda is generated at session-init from live sources → `.claude/session-agenda.json`.
> Human edits this file. Cortex reads it. Never write tactical items here.

---

## Phase 2.1 — Calibration Loop
**Depends on**: Phase 2.0 K15 Closure (DONE — divergence 0.247, 45/45, commit `3973118`)
**Goal**: Retrain Dogs on outcome feedback if enrichment divergence > 0.10
**Falsify**: Dogs still disagree equally (max_disagreement unchanged) after calibration
**Success**: BURN and VERIFY scores improve on enriched domains

---

## Phase 3 — Convergence Loop
**Depends on**: Phase 2.1 Calibration
**Goal**: Wire convergence_trigger → observations → Dog retrain cycle
**Falsify**: Convergence observations produced but no consumer acts on them
**Success**: Deterministic + LLM Dogs show ρ > 0.70 agreement on repeat stimuli

---

## Phase 4 — Sovereign Distribution
**Depends on**: Phase 3 Convergence
**Goal**: Multi-node kernel federation, x402 pay-per-verdict endpoint
**Falsify**: Second node diverges from primary on identical stimulus
**Success**: Two sovereign nodes agree within φ⁻¹ on verdict scores

---

## Parallel — Governance
- **Session continuity** (in progress): mempool as task queue, generated agenda — this PR
- **Inter-session state**: TODO.md replaced by `.claude/session-agenda.json`
- **Python tier governance**: `make lint-python-tiers` to CI (131 dead modules)

---

## Completed Phases (Reference)

| Phase | What | Date | Evidence |
|---|---|---|---|
| Phase 1 | Sovereign conviction pipeline, ρ=0.776 | 2026-05-16 | PRs #204-207 |
| Phase 1.5 | Soma L1/L2, GPU contention, hermes crons | 2026-05-17 | PRs #121-211 |
| Phase 2.0 | K15 Closure — outcome measurement, divergence 0.247 | 2026-05-30 | commit `3973118` |
| Hackathon | Demo + submission | 2026-05-10 | commits 0a491066, 35ca26f0 |
