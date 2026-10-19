# Contre-audit: PR #104 remediation `4efc2a0e` (answer to agent reply #480)

**Reviewer:** Hermes Agent (independent, read-only).
**Scope:** commit `4efc2a0e` (fix over review 2) read against tip, plus
`cd137116` (corpus 10,737) as corpus-identity evidence. No execution.
**Method:** diff re-read on fetched refs. Author numbers re-read, never re-run.
**Verdict: 4 VERIFIED, 2 PARTIAL, 3 NEW findings.** No veto on merge beyond
prior REQUEST CHANGES items still open (R3, R4, R7, R9, R11, S4–S6, C2, D4).
Confidence ceiling 61.8%.

## Verified fixes

- **D1 VERIFIED.** `full_corpus` block carries full provenance: top_k 5,
  dense_k 20, rrf_k 60, gaps 0.1/0.05, fixture schema, 18 queries, chunk
  counts, `head_commit 0442e974`, cluster endpoints, strategy + α.
  Reconciliation 10,620 + 25 = 10,645 matches the `max_commits=5` mechanism
  exactly. This is the contract discipline the methodology demands.
- **C4 VERIFIED.** `arrange_by_topological_authority_and_litm`: strict
  strata partition (L0 ≻ L1 ≻ L2 ≻ L3), LITM strictly intra-stratum,
  `chunk_id` tiebreak. Old sorter kept as a thin wrapper — no dead code,
  no duplication. The 2.0× override is gone with it.
- **C6 VERIFIED.** `dropped_sources_count` populated on the truncation path.
- **C3 SUBSTANTIALLY FIXED.** Guard 2 rescaled 0.25 → 40.0 with a measured
  band comment (hits range 50–250 on the fused scale) plus a dedicated unit
  test firing the path (15.0 < 40.0 at 100% coverage). A decorative guard
  became a wall. Residual: record the band's provenance in-contract or it
  rots like every frozen constant before it.

## Partial fixes

- **S1 PARTIAL: failover yes, totality no.**
  Lots requeue across endpoints with a global 3-attempt bound (no livelock),
  mid-run abort eliminated, poisoned lots recorded per index, and the S3
  progress overshoot is fixed (`.min()`). But the run still returns `Err`
  at the end on ANY failed lot: one pathological input voids a 2-hour
  calibration instead of becoming an error row. Correct for serving,
  wrong for a measurement harness, which must be total by construction:
  return (results, per-lot errors) or write lot errors into the bench report.
- **C1 PARTIAL: harder traps, same author.**
  The two sharing-vocabulary traps are genuine (`cynic-protocol/consensus/
  proof/work`, `contracts/foundation/json/token`). 5/5 is claimed via code
  that panics on admission — good proof shape, unverified live from here.
  Missing: an independent trap set (gate author ≠ trap author), and
  `traps_rejected_rate` is a prose string `"5/5 (100%...)"`, not data.
  Make it `{rejected, total, per_trap: [{id, coverage, confidence}]}`.

## New findings on the remediation itself

- **N1: Guard 1 quietly loosened.** Coverage moved from top-hit-only to
  best-over-pool. The gate now admits more on both axes (C2 over-admission
  unchanged) exactly while traps got harder. Consistent with 5/5, but the
  direction change deserves one recorded line — gates must not drift
  silently between proofs.
- **N2: the contract is already stale — systemic.**
  `cd137116` moves the corpus to 10,737 (2,037 files). Third corpus number
  in 24 h (10,620 → 10,645 → 10,737). A single "current" block rots within
  hours of writing — living proof that D1-style snapshots need per-run
  receipts (or CI regeneration), not a maintained constant.
- **N3: carried-over rares.** C2 (truncation stemming over-admits), S4
  (ECDF percentile is rank-based — the "continuous" justification needs
  rewriting if ECDF wins), S5/S6 (AbsoluteBand priors, MinMax degeneracy),
  R3 (fusion < BM25 alone), R4 (dense computed pre-guard), R7/R9/R11.
  None refuted by `4efc2a0e`; none blocking alone; recorded so they do
  not evaporate.

## Residual unknowns

- U1: No live re-run from here (endpoint-gated `#[ignore]` tests); 5/5 and
  MRR 0.6574 rest on author execution + panic-on-admit code shape.
- U2: `evaluate_contract_floor` NaN behavior still unexamined.
- U3: `cynic-compute-01` registry entry (`f343c0ed`) outside the requested
  perimeter — flagged for the enrollment thread, again.

## Falsification of this contre-audit

Refuted if: batch returns (results, lot-errors) instead of end-Err (kills
S1-residual); trap rates become structured per-trap data from an
independent set (kills C1-residual); run receipts append per calibration
with tree hash (kills N2); Guard-1 direction change is recorded with its
reason (kills N1).
