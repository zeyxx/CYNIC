# Review: PR #104 — data-centric bottom-up alignment

**Reviewer:** Hermes Agent (independent read-only audit, hostile-prism brief).
**Scope audited:** `origin/agent/hermes/rag-dense-margin` vs `origin/main`
(4 commits: `941be451`, `ecf3865d`, `e8e1bbd7`, `ad6efaff`).
**Method:** diff read on fetched refs, contract re-read, governance simulation on
PR tip (2285 tracked paths: 0 forbidden roots, 0 state leaks outside the declared
`infra/surface-registry/` skip). No code executed, no benches re-run, no live
endpoint touched. Every measurement below is the author's, re-read — never re-run.
**Overall verdict: REQUEST CHANGES** (narrow, gate-level: R1, R2, R6, B2).
Tranches 3 and 4 approve as-is. Confidence ceiling 61.8% throughout.

## Tranche 1 — bilateral guard + B1 Q4 (`941be451`): APPROVE WITH FIXES, Q 74.4 WAG

What holds: `dense_margin` is well-defined (0 without evidence,
`scorer.rs`), unit tests are clean contracts, throughput is internally coherent
(1000 chunks / 43.1 s = 23.2 chunks/s), and the contract records losing numbers
too (`full_corpus: null`, `sentinel_rqc4_paraphrase_dense: 0.0`). Rare honesty.

- **R1 (major): threshold provenance missing.** The contract records the MRR
  outcomes (`mrr_hybrid_bilateral_guard: 0.491`) but not the inputs that
  produced them: `bm25_guard_gap`, `dense_guard_gap`, K, `dense_k`, `limit`,
  fixture revision. A number without its config is not reproducible and violates
  the PR's own Chapter 0/Ch7 rules. Record the full config beside every measured
  block in `contracts/embedding-selection.json`.
- **R2 (major): adoption on means alone.** Unilateral 0.481 → bilateral 0.491 is
  adopted without the paired Wilcoxon test the methodology mandates (§7.2).
  Either attach the p-value over discriminating pairs or downgrade bilateral
  from adopted to candidate.
- **R3 (structural question): fusion underperforms sparse alone.**
  `mrr_bm25: 0.574` beats every hybrid variant (best 0.491) on the measured
  subset. The guard's proven win is "stop breaking BM25", not "fusion helps".
  The failure-mode matrix (spec §4.2) must name the query class where dense is
  load-bearing (RQ4_POWERS is one exhibit: 0.333 → 1.000) or fusion should be
  scoped to dense-sovereign queries only.
- **R4 (minor): dense always computed.** `top_k_dense` now runs before the
  guard, so the BM25 short-circuit still pays dense encoding. Reconcile with
  the Chapter 1 latency envelope or gate the call behind the sparse margin.
- **R5 (minor): sentinel at 0.0 without verdict.**
  `sentinel_rqc4_paraphrase_dense: 0.0` — dense fully blind on the paraphrase
  sentinel. Record one line: expected trap behaviour or dense blind spot.
- Repo switch `ggml-org` → `unsloth` (`embeddinggemma-2-UD-Q4_K_XL.gguf`,
  168 MB) accepted: `model_file` pinned, "same weights" reads as same base
  model. Pin the base-model digest too if cheap.
- Framing answer (`guard_gap: None`): verified no side effects —
  `dense_confident` defaults to false, identical to the old unilateral path.

| F 78 | Φ 80 | V 60 | C 82 | B 62 | S 92 | K 72 | **Q 74.4 WAG** |

## Tranche 2 — normative spec (`ecf3865d`): REQUEST CHANGES, Q 66.7 WAG

Strong principles, real falsifiers, correct small-n statistics in §WHY.
But the table freezes what the axioms order to measure:

- **B2 (blocking): self-declared normative status.** Header still claims
  `Normative & Operational` with no constituent point-by-point validation, and
  schema `cynic.spec.data_centric_rag.v1` is registered nowhere (contracts,
  infra, registry all empty). Downgrade to `Proposal` until admission.
  (Correction to my own earlier note: `contracts/embedding-selection.json`
  exists on main — the authority line is valid; only the schema is dangling.)
- **M1: `RSS > 3.0 GB → abort`.** Absolute number, no host. Contradicts Axiom 1
  (`RAM libre − 1 Go`). Which one rules?
- **M2: `ρ < 0.95 → reject`.** Frozen Spearman constant with no provenance
  (which model, which 1000 pairs). §3.2 orders measure-before-judge; the table
  judges before measuring.
- **M3: BM25 drift `±0.02`.** On a frozen deterministic tree the expected
  drift is exactly 0.00. The tolerance legalizes small pollution instead of
  forbidding it. Either 0 or a justification.
- **M4: trap `> 0.0% → gate failure`.** One missed trap on small N fails
  everything — brittle and anti-small-n in spirit. Missing: the judge protocol
  (who calls "hallucination"? without bidirectional swaps and κ, it is
  reviewer arbitrariness).
- **m1–m4:** `N ≥ 30` (§invariants) vs `N ≥ 40` (§7.1); `< 10 ms @ N ≤ 50k`
  without host/dim; `subset-1000` undefined (1000 what?); `μ_cos > 0.7`
  without origin measurement.
- **Coherence gap:** §2.2 normatively mandates Rust AST chunking
  (`syn`/`tree-sitter`) that the implementation does not have (see R6).
  A normative spec must not declare unimplemented behaviour.

| F 70 | Φ 55 | V 75 | C 80 | B 50 | S 85 | K 60 | **Q 66.7 WAG** |

## Tranche 3 — 100% ingestion audit (`e8e1bbd7`): APPROVE*, Q 80.1 HOWL*

Genuine proof harness: bounds, emptiness, BLAKE3 determinism, disk-slice
fidelity, volume ratchet. The `*`: scope, not quality.

- **R6 (major, scope): "100%" covers Markdown + JSON contracts only.**
  No `.rs` source is ingested (`lib.rs`: `ext == "md"`, `ext == "json"`).
  The engine is code-blind while the spec normatively mandates Rust AST
  chunking. Either implement §2.2 or downgrade spec + PR language to
  docs-and-contracts retrieval.
- **R7 (minor): silent skips.** Both walks use `if let Ok(read)` — an
  unreadable or non-UTF8 file vanishes from the corpus with no count.
  Add a skip counter to the manifest.
- **R8 (minor): panic before report.**
  `lines[line_start - 1..line_end]` panics before the violation is recorded
  in `invalid_line_bounds`, so the falsification path is dead for the worst
  case. Check-then-slice (`.get()`).
- **R9 (minor): id drifts from spec.** Test derives
  `BLAKE3(path:start:content)`; spec §2.3 promises
  `hash(path, start, end, content_digest)`. Same path+start+content with a
  different end collides by construction. Align one to the other.
- **R10 (minor): `>= 10000` ratchet fights pruning.** The pruning policy
  wants the corpus to shrink on debt removal; a growth-only floor will
  eventually veto a legitimate purge. Two-sided band or drop.
- **R11 (minor): JSON span finder assumes 2-space/tab indent.**
  Other indents and minified JSON fall back to dummy `(1, total)` silently,
  and nested-string false positives are unhandled. Count fallbacks per file.

| F 88 | Φ 70 | V 85 | C 80 | B 75 | S 90 | K 75 | **Q 80.1 HOWL*** |

## Tranche 4 — chunk RRF + grading (`ad6efaff`): APPROVE, Q 85.6 HOWL

Minimal, correct, self-aware diff. Keying on `chunk_id` ends file-level
smearing; the sort tiebreak `(path, span)` carries an explicit comment about
`HashMap` randomness — the framing question on determinism is answered in
code. `grade_chunk` (exact id > span key > path fallback) preserves
retro-compatibility with the existing fixture.

- **R12 (minor): magic grade.** `find_gold`/`find_dense_gold` hardcode
  `== 2` and the 0/1/2 scale is undocumented in code. Name a `GOLD_GRADE`
  constant and point at the fixture doc.

| F 85 | Φ 88 | V 80 | C 85 | B 85 | S 92 | K 85 | **Q 85.6 HOWL** |

## Residual unknowns (explicit)

- U1: No bench re-run here; all numbers are author-reported, re-read only.
  The 0.481 → 0.491 delta could be noise — that is exactly why R2 demands
  the paired test rather than asserting it.
- U2: Fixture size behind `subset-1000` (queries? chunks?) unconfirmed —
  blocks any power reading of the MRR table.
- U3: Whether `evaluate_contract_floor` can yield NaN (sort uses
  `unwrap_or(Equal)`); unexamined, low risk.
- U4: My own earlier B1 (missing contract) was stale-checkout error,
  withdrawn above. Skepticism applied to self first.

## Falsification of this review

This review is refuted if: the contract gains full threshold configs and a
paired p-value for bilateral (kills R1/R2); the fixture documents
`subset-1000` and the dense-sovereign query class (kills R3/U2); the spec
drops to Proposal with unfrozen gates (kills B2/M1–M4); or ingestion covers
`.rs` via the mandated AST path (kills R6).
