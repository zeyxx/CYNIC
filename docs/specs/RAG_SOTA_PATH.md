# RAG SOTA Path — Executed Record (Organic / Bottom-Up)

**Schema:** `cynic.spec.rag_sota_path.v1`
**Status:** Living record — updated per frozen run, never rewritten
**Authority:** `contracts/embedding-selection.json` (numbers), `target/convex_sweep_report*.json` (machine reports)
**Method:** `DATA_CENTRIC_RAG_METHODOLOGY.md` (normative) · `rag-calibration-plan.md` (ladder, standing at 18 queries) · `ORGANIC_RAG_BOTTOM_UP_SPEC.md` (phases)

This document records what was actually tried, measured, kept, and killed
on the way to a state-of-the-art sovereign RAG. Principles live in the
methodology doc; this file carries the executed path and the current
position. Every claim below resolves to a contract entry or a machine
report — no aspirations.

## 1. Ladder executed (runs 1–9, 2026-10-14/15)

| Run | Corpus | Queries | Space | Headline |
|---|---|---|---|---|
| 1. contamination screen | 10 772 | 42 | mixed BF16/Q4 | ECDF 0.5317 @0.60, p=0.0156 — signal found, space dirty |
| 2. clean v2 | 12 011 | 42 | mixed | ECDF 0.5179 @0.60, p=0.0139 — contamination did not drive the signal |
| 3. fixture 64 | 12 013 | 64 | mixed | ECDF @0.60, p=0.0064, n=16 — blind inclusion screen works |
| 4. freeze 85 | 12 013 | 85 | mixed | MinMax 0.5990 @0.80, ECDF 0.5308 @0.75, p=0.0073, n=31 — method frozen, not the alpha |
| 5. routing analysis | 12 013 | 85 | mixed vectors | per-doctype optima rs 0.90 / json 0.00 / md 0.45; query-text predictor xval 0.620 over global 0.599 |
| 6. router in domain | 12 013 | 85 | mixed vectors | `route_query` + live split-half: routed 0.6249/0.5492 over global — routing is code, not analysis |
| 7. Q8-uniform | 12 020 | 85 | Q8 single space | MinMax 0.5884 @0.80, ECDF 0.5269 @0.60, p=0.0039 — 5th significant run, 1st in a uniform space; Q8 adopted SSOT |

Per-run curves, Wilcoxon statistics, and provenance configs:
contract entries 1–9 (`run_20261015_q8_uniform_85q` latest).

## 2. Frozen verdicts (full corpus, deterministic code)

- **Convex fusion beats ordinal RRF**, five consecutive significant runs
  (p = 0.0156 / 0.0139 / 0.0064 / 0.0073 / 0.0039). Frozen is the method
  (concave curve, interior optimum, alpha recalibrated per sweep) — never
  the alpha number (ECDF optimum moved 0.60→0.75→0.60 across fixtures).
- **MinMax peaks higher, ECDF is the stable arm**: MinMax optimum wandered
  (0.50/0.90/0.90/0.80) while ECDF stayed concave with an interior optimum.
  Significance is tested on ECDF vs RRF.
- **Per-doctype routing beats global fusion** (live split-half, predictor in
  the loop): MinMax routed 0.6113 vs 0.5884 global; ECDF routed 0.5718 vs
  0.5269 global. Deployment rule in the contract: json → pure BM25
  (fusion harms), rs/md → their xval alphas, recalibrated per corpus.
- **Q8-uniform is the SSOT vector space**: same-file cross-runner cosine
  0.99994 (heterogeneous-hardware residual, 47× tighter than the mixed
  0.99721); bit-identity across devices is unachievable and not required —
  end-to-end reproduction is the gate, and it passed.
- **Cluster resolves from the registry**: explicit `CYNIC_EMBED_URL` wins,
  otherwise `infra/registry.json` `bench_endpoint`s join; excluded runners
  carry no key and stay out by construction (unit-tested).

## 3. Killed hypotheses (with cause of death)

- **Bilateral guard as specified**: REJECTED — null on both corpora, sign
  flips with corpus (plan §6 already recorded; stays dead).
- **BM25-top-1 predictor for routing**: REJECTED — accuracy 0.624 but
  JSON-biased errors kill the gain (realized 0.517). Query-text predictor
  (0.635) adopted instead.
- **Chunk-size hypothesis for prose failures**: DEAD by measurement — same
  file, same sizes score dense 1.0 (RQC2) and 0.0 (RQC3, RQC4). Failures are
  vocabulary mismatch (RQ12: 0/7 query terms in gold) and conceptual
  paraphrase gap. No chunker churn without a size-correlated cause.
- **Q4-everywhere**: SUPERSEDED by Q8 (17× tighter to BF16 at half the RAM).
- **Mixed-space runs 1–6**: SUPERSEDED for future decisions (kept as
  history); all live numbers are Q8-uniform.
- **gold@20 replacing MRR@5**: CORRECTED — report carries both, MRR stays
  the decision metric.

## 4. Blind-spot register (diagnosed, open)

| Symptom | Diagnosis | Next axis |
|---|---|---|
| md prose ≤0.42 everywhere | vocabulary mismatch + paraphrase gap, not chunking | expansion (plan axis C) or md-targeted retrieval |
| RRF destroys perfect BM25 (RQC3, RQ65: 1.0→0.0) | dense-miss drag in ordinal fusion | fixed by routing (json→BM25); residual rs/md cases open |
| json fusion harms (0.816 BM25 alone) | dense noise on identifier-heavy structured text | routed around; encoder-side fix open (model axis B) |
| 63/85 ties in Wilcoxon | fixture saturation on decided queries | disagreement-zone query pairs (plan §5.4) |

## 5. Current SOTA position (Q8-uniform, 85 queries + 5 traps, 12 020 chunks)

BM25 0.5004 · dense 0.5918 · RRF 0.4361 · MinMax 0.5884 @0.80 ·
ECDF 0.5269 @0.60 · routed 0.6113 / 0.5718 · Wilcoxon p=0.0039 (17/5/63).
Runners: cortex + compute-01, Q8_0, slots 4+4, registry-resolved.

## 6. Next rungs (cheapest first, one variable each)

1. Disagreement-zone fixture pairs (plan §5.4) — raise discriminating power
   above the current 22/85 before any new axis.
2. Model axis B (jina-v5-nano/small, Qwen3-0.6B screens) — only remaining
   lever for the md/json encoder-side gaps; degradation + reproducibility
   gates apply.
3. Expansion axis C if B leaves RQ2-class vocabulary gaps.
4. Confidence columns C1–C4 (spec Phase 2.3) and L3 MCP exposure — surface
   work, gated on first-stage recall, not before.
