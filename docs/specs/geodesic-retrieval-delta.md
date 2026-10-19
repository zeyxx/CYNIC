# Spec delta: geodesic retrieval + epistemic vigilance (external corpus)

## Source

Frozen external corpus: `vault/captures/external-pins-2026-10-18/` (outside repo,
11 files + `manifest.json`). 10 unique contents — `gemini-code-1792351298567.md`
and `gemini-code-1792351438062.md` are byte-identical (sha256 `e7910db1…`).
**15/15 arXiv IDs verified** against export.arxiv.org (2026-10-18, titles match).
Worked numbers in the corpus are pedagogical toys, not measurements.

Nothing from the corpus enters the living `cynic know` index as authority.
Only this spec's retained deltas become retrievable, and only after proof.

## Retained (mapped to pipeline stages)

| # | Lesson | Stage | Requirement |
|---|--------|-------|-------------|
| R1 | Geodesic/diffusion retrieval over a causal graph (personalized PageRank) beats naive cosine and blocks the U-shaped short-circuit (stale month-1 code ranking above live month-4 code) | Model/Projection | `cynic-traces` Model layer MUST be a causal graph (sessions/commits/decisions, invalidation-weighted edges); retrieval walks the graph (Dijkstra/diffusion), never raw cosine over flat chunks. Formalizes the tensor doctrine's anti-linear rule. |
| R2 | Multi-turn preference optimization with discount (DMPO, `2406.14868`) | Projection | DPO pairs MUST be trajectory-scoped with temporal discount γ, not isolated turns. Anti-leakage split by connected component already required; add γ to the projection contract. |
| R3 | Committee-variance epistemic trigger (mean energy + variance over 3–5 critics → HITL on disagreement) | Projection/ops | Admission surfaces SHOULD expose a committee-disagreement signal; high variance = fail-closed to the operator, never silent best-effort. Softmax alone is shift-invariant and insufficient. |
| R4 | Milestone-DAG history (SWE-Milestone/DeepCommit, `2603.13428`) | Model | Milestones (functionally cohesive, testable units) are the canonical history atom for evolution reasoning, not raw commits. α10 (strict DAG) extends: milestone nodes with dependency edges. |
| R5 | Survival/temporal weighting of operator signal (recent maintained commits outrank early abandoned ones) | Projection | Preference pairs carry a survival weight (still on main = maximal; replaced = counter-example). Formalizes the empirical temporal-regime rule. |
| R6 | Comparative PRM tuples (SURFPRM, `2609.32351`) | Projection | Process judges MUST be comparative (state, good action, rejected action, critique) by knockout, never absolute scores. Grounds the PRM-over-ORM rule. |

## Explicitly deferred (gated behind the canonical substrate — Gall's law)

- EBM relief + actor-critic + Bellman selection (needs R1 graph + validated pairs first).
- Activation steering / SAE probes on local models (needs a local model program, which does not exist).
- System-2→System-1 KL distillation + recursive fractal agents (needs the whole chain green first).
- Python tooling cited by the corpus (UMAP, PyTorch Geometric, NetworkX) is re-homed to Rust at port time per the supply-chain rule.

## Acceptance

- R1–R6 each land as a testable clause in the `cynic-traces` spec or a proof test, not as prose.
- No external-corpus sentence is quoted as authority in code, contracts, or the knowledge index.
- This spec is the ONLY in-repo artifact derived from the frozen corpus until R1–R6 are proven.
