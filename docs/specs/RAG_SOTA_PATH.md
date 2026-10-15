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
- **Per-doctype routing beats global fusion** — SUPERSEDED by run 10 (task
  prefixes). Under prefixes the predictor-in-loop realized 0.6558 vs global
  0.6965 (MinMax) and 0.5704 vs 0.5725 (ECDF): per-group alphas dilute a
  strong global. The json→pure-BM25 rule died first (json at alpha 0.45
  reaches 0.8395 under prefixes). `route_query` stays compiled with its
  unit tests as instrumentation; deployment is global MinMax alpha 0.80.
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

## 5. Current SOTA position (Q8-uniform, 87 queries + 5 traps, 12 028 chunks)

BM25 0.5128 · dense 0.6828 · RRF 0.4525 · MinMax 0.6977 @0.80 ·
ECDF 0.5824 @0.75 · Wilcoxon p=0.0003 (31/6/50, n_nonzero 37).
Pipeline: query prefix `task: search result | query: `, docs
`title: {filename} | text: {body}`. Runners: cortex + compute-01, Q8_0,
slots 4+4, registry-resolved. Cluster fanout proven live (128-text probe:
16 sublots → cortex 11, compute 5; per-endpoint pools with headroom-guarded
stealing — the old shared queue starved every peer past the first).
Deployment: global MinMax alpha 0.80; MRL-256d adopted for future runs.
Wave 4 (2026-10-16, fixture 85→87): RQ95 (BM25-win paraphrase, 1.00/0.50)
and RQ96 (identifier tie, 1.00/1.00) admitted via pure-arm screen; A2
rejected ×3 (checklist memory passage unreachable by rephrase — depth
diagnosis open, not a 4th rephrase). Freeze check: old-85 reproduction
within 0.01 on all arms (RRF/ECDF exact, dense Δ 0.0094 = live-workspace
corpus drift); 87-mean moves are the predicted selection effect. Freeze holds.
MRL-256 screen (2026-10-15, `CYNIC_TRUNC_DIM=256`, pure re-rank, zero
re-embed): dense 0.5918, MinMax 0.5884, ECDF 0.5278, Wilcoxon p=0.0039 —
identical to 768d on every decision. Google's −2.5 code points do not
materialize on this workload; 256d adopted for future runs (3× lighter
cache, faster cosine), frozen 768d artifacts untouched.
Task prefixes (2026-10-16, EmbeddingGemma-2 card protocol, full re-embed,
run 10): dense 0.6906 (+0.0988), MinMax 0.6965 @0.80 (+0.1081), ECDF
0.5725 @0.75 (+0.0456), Wilcoxon p=0.0003, n_nonzero 37 (31/6/48). The
pipeline fields existed for this since day one — the largest single gain
of the program. KILLED in the same run: per-doctype routing (realized
0.6558 < global 0.6965; prefixes lifted json to 0.8395 at alpha 0.45,
so the json→pure-BM25 rule and its justification died).

## 6. Next rungs (SOTA methods, cheapest first, one variable each)

The meme that re-aimed this program: studying single scores in 2026 is the
moon-shot — the field has named methods for every one of our blind spots,
and the organic order applies them cheapest-first instead of admiring the
problem. Each rung below names its SOTA reference, its target symptom, and
its gate; training-free rungs come before any gradient step.

1. **Disagreement-zone fixture pairs** (retrieval evaluation standard
   practice: power comes from informative queries, not volume) — raise
   discriminating power above the current 22/85. Gate: Wilcoxon n_nonzero
   grows without moving frozen optima.
2. **Cross-encoder rerank on fused top-20** (ms-marco-MiniLM / jina-reranker
   pattern: cheap, no training, strongest published lever on prose recall)
   — targets the md ≤0.42 gap directly. Gate: routed ECDF moves, zero new
   BM25-perfect destructions, latency budget stated.
3. **Model axis B screens** (jina-embeddings-v5 nano/small, Qwen3-Embedding
   0.6B) — encoder-side lever for json/md gaps; degradation and
   reproducibility gates from the plan apply unchanged.
4. **Hard-negative contrastive fine-tune** (the fixture IS the training set:
   `crates/cynic-knowledge/tests/retrieval_training_pairs.json`, positives
   graded ≥1, negatives mined from our own dense top-30 misses) — InfoNCE on
   EmbeddingGemma with in-batch + mined negatives. Gated on a training rig
   (no CUDA today) and on rungs 1–3 plateauing; file-level positives are a
   v1 approximation, chunk-level grades come first if noise dominates.
5. **Late interaction (ColBERTv2) / learned sparse (SPLADE)** — only if the
   fine-tuned bi-encoder still leaves identifier-heavy json gaps; index-size
   and Vulkan-serving costs measured before adoption, not after.
6. **Surface work** (confidence columns C1–C4, L3 MCP exposure) — gated on
   first-stage recall, not before.
