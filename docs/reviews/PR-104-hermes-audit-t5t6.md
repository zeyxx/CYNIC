# Review 2: PR #104 Tranches 5–6 — ML engineer, Data scientist, Cynical prisms

**Reviewer:** Hermes Agent (independent, read-only, hostile prisms).
**Scope audited:** `origin/agent/hermes/rag-dense-margin` tip `7cc51e61`
(7 new commits since `ad6efaff`, headlined `1905a64a`, `ab75c38d`,
`87f02e45`, `7cc51e61`; plus `82096b6d` answering review 1).
**Method:** diff re-read on fetched refs, no execution, no live endpoint.
All numbers below are author-reported, re-read only.
**Verdict: REQUEST CHANGES** — one resilience defect (S1), one config-split
invalidating the headline proofs (D3), one vacuous guard (C3). The rest approves.
Prior review items R1(partial)/R2/R6/R12/B2/M1–M4 verified fixed or honestly
downgraded in `82096b6d` — credited below. Confidence ceiling 61.8%.

## 0. Credit where measured

- R1 fixed: `provenance_config` (top_k 5, dense_k 20, rrf_k 60, gaps 0.1/0.05,
  fixture schema, 18 queries) now sits beside the numbers. Exemplary.
- R2 honored: Wilcoxon recorded as unpowered (1 discriminating pair, ties 17),
  status kept at candidate. Honesty over freezing.
- R8/R10/R12 fixed (check-then-slice, two-sided band, `GOLD_GRADE`).
- B2/M1–M4 addressed (Proposal status, cortex-anchored 3.0G, investigate-not-reject
  ρ, 0.00-on-pinned drift rule, operationalized trap verdict, scope §6 admitting
  code-blindness with Phase 2 scheduled).
- Still open from review 1: R3 (fusion < BM25 alone), R4 (dense computed before
  guard), R5 (adjudicated ✓ in Wolfean prose — accepted), R7/R9/R11 (untouched:
  silent read skips, id-vs-spec drift, JSON indent fallback counter).

## 1. ML-engineer prism — Tranche 5 (pool, slots, convex)

What holds: per-endpoint slot workers (`endpoint.slots`), index-keyed
reassembly (deterministic by construction, not by timing), empty-input
early return, bounded env parsing everywhere, `Copy` hits enabling the LITM
copies. Slot spec `#slots=` with sane default (`fanout_width`). `health()`
checks only `base_urls[0]` (minor: pool health is first-endpoint health).

- **S1 (major, resilience): fail-fast batch on single poisoned lot.**
  `http.rs`: a worker failing one lot after 3 attempts returns `Err`, and
  `try_join_all` aborts the ENTIRE run — all queued and in-flight lots die
  with it. A 10,645-chunk calibration dies at 99% on one bad lot, with no
  per-lot error record. A benchmark harness must be total: catch per lot,
  continue, report the error vector. Current shape is fine for serving,
  wrong for measurement.
- **S2 (major-minor): stealing without failover.** Retries are pinned to the
  SAME endpoint (`post_batch_to(endpoint_url)`, 3×, 50/100/150 ms backoff —
  cosmetic against 120 s timeouts). A dead endpoint keeps its stolen lots
  through all retries while siblings idle. Either requeue to the shared queue
  on endpoint-level failure or document single-endpoint fate-sharing.
- **S3 (minor): progress overshoot.** `report(done*FANOUT_SUB_LOT)` exceeds
  `texts.len()` on a partial final lot. Cosmetic, fix with `min()`.
- **S4 (minor): ECDF percentile is ordinality in disguise.**
  `percentile = rank/N` discards magnitudes — the exact sin the spec
  condemns in RRF (§WHY-1). If ECDF wins the sweep, the "continuous"
  justification needs rewriting: the win would come from rank robustness,
  not magnitude preservation. Name it honestly.
- **S5 (minor): AbsoluteBand priors are frozen magic.**
  BM25 `/30.0`, cosine `(c−0.2)/0.7` ("typically") — same disease as the
  cured M1/M2. Either measure the bands per corpus or demote AbsoluteBand
  to documented fallback. MinMax and ECDF need no priors; the sweep
  apparently agrees (ECDF/MinMax headlined, AbsoluteBand absent from
  the optimum — confirm in the recorded table).
- **S6 (minor): MinMax single-candidate degenerates.**
  All-equal scores → every candidate `1.0`: no-discrimination reported as
  maximum confidence. Clamp output or document.

| F 80 | Φ 78 | V 70 | C 85 | B 70 | S 90 | K 75 | **Q 78.0 WAG** |

## 2. Data-scientist prism — calibration (MRR 0.6574 @ α=0.60)

- **D1 (major): the flagship numbers live in prose, not in contract.**
  α=0.60 / MRR 0.6574 appear in commit/PR language; the calibration test
  only `println!`s a sweep table for human eyeballing and asserts nothing
  about outcomes (sole assert: fixture schema). R1-redux, one level up:
  record α*, strategy, fixture revision, corpus tree hash, model digest,
  and N in `contracts/embedding-selection.json` (`full_corpus` is still
  `null` at tip) or the optimum does not exist as evidence.
- **D2 (major): 10,620 vs 10,645 unexplained.** The audit froze 10,620;
  calibration reports 10,645 (+25). Candidate causes: `max_commits=5`
  commit-chunks in calibration vs 0 in audit, or tree drift between runs.
  Either is legitimate — neither is recorded. Two "full corpus" numbers
  with no reconciliation break the Chapter 7 corpus-identity rule the
  author just wrote. One line: tree hash + chunk count per run.
- **D3 (major): headline proofs measured at different configs.**
  The gating proof (100% traps rejected) runs at α=0.30 with default
  config; the adopted optimum is α=0.60. Trap-wall × α interaction is
  untested — the wall may not hold at the operating point. Re-run the
  trap matrix at α=0.60 (and at the MinMax/ECDF runner-up) before
  claiming both numbers in one breath.
- **D4 (minor): N=18 queries, again.** Consistent with the author's own
  candidate status — no new objection, just the ceiling: nothing freezes
  until the ≥30-pair fixture lands (spec §7.1: N≥40 with duals).

| F 72 | Φ 75 | V 55 | C 80 | B 65 | S 88 | K 70 | **Q 71.5 WAG** |

## 3. Cynical prism — Tranche 6 (grounding, traps, LITM)

- **C1 (major): trap set is three queries, two of them trivial.**
  HELM_K8S is lexically disjoint (rejection proves substring matching, not
  judgment). UNISWAP sits at 0.25 coverage vs the 0.30 floor — one
  stemming wobble from admission. SWAPFILE shares cortex/host vocabulary
  and survives only by arithmetic luck. "100% traps rejected" = 3/3 on a
  set the gate was tuned against. Demand one sharing-vocabulary adversarial
  trap (e.g. in-corpus terms, out-of-corpus claim) before the wall means
  anything. The methodology's own small-n sermon applies to its own proof.
- **C2 (major): term-coverage over-admits by construction.**
  Truncation stemming (`t[..len-1]`, `t[..len-2]` substring) inflates
  matches; empty-term queries return 1.0 (fully covered by nothing).
  For a safety gate the dangerous direction is over-admission, and this
  gate leans exactly there. Measure the false-admit rate on
  sharing-vocabulary traps (C1) or tighten matching to token identity
  plus declared morphological rules.
- **C3 (major): Guard 2 is vacuous at operating scales.**
  `min_confidence 0.25` vs fused scores that live at ~1.6–100 scale
  (RRF×100+floor, convex×100+floor, ×altitude). At these scales the floor
  admits everything with passing coverage — all observed rejections flow
  through Guard 1. A guard that cannot fire is decoration. Either
  normalize the confidence to the score scale or prove a rejection
  trace through Guard 2 and record it.
- **C4 (major-minor): LITM breaks the altitude invariant.**
  Spec §6.2 orders strict L0-first prompt assembly; `reorder_lost_in_the_middle`
  redistributes to [0,2,…,3,1] AFTER the authority sort, so an L0 chunk can
  land last. Both cannot be normative. Declare which rules: boundaries
  (LITM) or altitude order — or constrain LITM to within-tier permutation.
  Related: reported `confidence` comes from pre-sort top score while
  `top_authority` comes post-sort — mixed semantics in one verdict struct.
- **C5 (minor): 2.0× authority override and 4/3/2/1 weights are frozen.**
  Same magic-constant family, now inside the joie de trier. Measure or
  mark as provisional in code comment.
- **C6 (minor): `max_context_chars` break-loop drops trailing sources
  silently.** Admitted queries carry fewer sources than selected with no
  record. Count dropped sources in the verdict.

| F 65 | Φ 60 | V 70 | C 78 | B 55 | S 85 | K 68 | **Q 68.1 WAG** |

## Residual unknowns

- U1: No re-run here; pool failure modes (S1/S2) read from code paths,
  untriggered. A chaos run (kill one endpoint mid-calibration) would
  settle both in minutes.
- U2: `evaluate_contract_floor` range unexamined (NaN would poison sort
  via `unwrap_or(Equal)`); carried over from review 1.
- U3: Whether the 10,645 run pinned HEAD (Ch7 rule) — unconfirmed from
  the test code (`max_commits=5` reads live history).
- U4: New node `cynic-compute-01` registry entry (`f343c0ed`) not audited
  here — out of the requested T5/T6 code perimeter, flagged for the
  enrollment thread.

## Falsification of this review

Refuted if: per-lot error capture lands (kills S1); failover or
fate-sharing documented (S2); α*/MRR/strategy/tree-hash/model-digest
enter the contract (D1/D2); trap matrix re-runs green at α=0.60 with one
sharing-vocabulary adversarial (D3/C1); a Guard-2 rejection trace is
recorded or the floor rescaled (C3); LITM-vs-altitude precedence is
declared (C4).
