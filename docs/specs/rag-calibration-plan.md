# RAG Calibration Plan — Organic / Bottom-Up

How CYNIC retrieval improves: data first, one variable at a time, each step
gated by the graded bench. No default is baked in before full-corpus proof.
No architecture is proposed before the numbers demand it.

> Standing as of 2026-10-14 (18 queries) below is history. Live position:
> `RAG_SOTA_PATH.md` (9 runs, 85 queries, frozen convex method, per-doctype
> routing, Q8-uniform space). This plan stays the ladder; the path doc is
> the scoreboard.

## 0. Standing (measured, deterministic since RRF tie-break fix)

| Condition (subset-1000, no prefix) | BM25 | Dense | Hybrid RRF | Guard 0.1 |
|---|---|---|---|---|
| Mean MRR | 0.574 | 0.226 | 0.431 | **0.523** |

- Bench runs are bit-reproducible across processes (base1 == base2 exactly).
- BM25 wins 9 queries alone; fusion wins 4 (RQ1/RQ2c/RQ6/RQ7), breaks 4
  (RQ5/RQ11/RQ12/RQ13 pattern). Dense alone is weak (0.226): fusion is what
  makes it useful, and the guard is what stops it from demoting BM25.
- Full-corpus (9570 chunks) calibration in progress; subset numbers are
  provisional until it lands.

## 1. Rules (non-negotiable)

1. **One variable per run.** Model, pipeline, k, guard, corpus — never two.
2. **Determinism first.** Any nondeterminism (HashMap order, unstable sort,
   unseeded sampling) is a bug, fixed before measuring.
3. **Subset screens, corpus decides.** subset-1000 for cheap screening
   (~90 s); full corpus for freezing (~15 min). Thresholds freeze only on
   full corpus.
4. **Cache keys separate everything.** Model × pipeline tag already scopes
   vector files; any new axis (k, guard, expansion) must be visible in the
   report, never silent.
5. **Near-miss vs total-miss.** Every zero is diagnosed with gold@20 per
   side before any fix is proposed.
6. **No baked defaults.** `guard_gap=None`, RRF k=60 stay until a frozen
   calibration says otherwise, on full corpus, reproduced twice.

## 2. Axis order (data need, cheapest first)

### A. Fusion mechanics (no re-embed — pure rerank of cached vectors)

- **A1. RRF k sweep: {10, 20, 60}.** SOTA (parameterized-fusion, SciFact)
  freezes κ=20 over 60. Cost: ~30 s per k. Gate: pick argmax hybrid on
  full corpus; report per-query deltas, not just the mean.
- **A2. Guard threshold freeze: {None, 0.05, 0.1, 0.25}.** Current best
  0.1 (+0.092 on subset). Freeze on full corpus. Watch the SOTA warning:
  skipping a side can lose nDCG — if guard harms any full-corpus win,
  prefer soft weighting (next).
- **A3. Dense-side margin (QuDAR-confidence).** Mirror `bm25_margin` on the
  dense top1-top2 cosine gap; fuse by margin-weighted scores instead of
  the hard gate. Only if A2 leaves wins on the table.

### B. Embedding models (one re-embed each — screen on subset first)

Reference: EmbeddingGemma-2 BF16 (MTEB-code 78.68, our dense 0.226).

| # | Model (GGUF) | Size | Why | A priori |
|---|---|---|---|---|
| B1 | embeddinggemma-2 Q4 (`ggml-org`/`unsloth`) | 271 MB | Same weights: isolates quant cost. Free speed if ≈equal. | neutral, faster |
| B2 | jina-embeddings-v5-text-nano-retrieval | 212 MB | Retrieval-tuned head; our task IS retrieval. MMTEB 65.5. | hopeful |
| B3 | jina-embeddings-v5-text-small-retrieval | 596 MB | Best sub-1B quality (MMTEB 67.0, Qwen3-0.6B base). | ceiling |
| B4 | Qwen3-Embedding-0.6B | 596 MB | Raw backbone, Apache-2.0, MTEB ~67.8. Fallback if jina GGUF misbehaves. | ceiling |
| B0 | All-MiniLM-L6-v2 | 23 MB | Floor baseline: quantifies what "small" costs. | floor |

Protocol per model: subset-1000 screen (hybrid@guard0.1 + dense isolated +
RQC4 sentinel). Full corpus only for models beating reference. Per-model
pipeline config via existing `TextPipeline` (prefixes/template cover
Qwen/jina instruction formats); cache separates automatically.

Gate B: adopt iff hybrid improves AND dense isolated does not regress AND
RQC4 moves. Otherwise the model is out — no second chances on vibes.

### C. Query expansion (only if B leaves RQ2-class gaps)

RQ2 ("disk space reclaimed" → retention/TTL vocabulary) fails both sides:
the expansion axis (synonym/rephrase variants fused as extra signals,
QuDAR perspective 2). Not started until B freezes — expansion on top of a
weak encoder measures the wrong thing.

### D. Score-fusion (only if RRF plateaus)

Convex combination with min-max normalization and dense prior α∈[0.55,0.95]
(Fusion-analysis 2210.11934: CC beats RRF in/out-of-domain). Needs score
normalization machinery — the most invasive axis, so it goes last.

## 3. Blind-spot register (diagnosed, not yet fixed)

| Query | Symptom | Diagnosis | Axis |
|---|---|---|---|
| RQ2_DISK_RECLAIM | 0.0 both sides | Vocabulary gap; gold valid but thin (1 shared term) | C (expansion) |
| RQC1_CONSENSUS | 0.0, term present 7× in gold | Ranking failure, not coverage — needs gold@20 | A (mechanics) |
| RQC4_PARAPHRASE | 0.0 dense | Abstraction sentinel; strongest anti-signal on encoder | B (model) |
| RQ2b/RQC1-class | 0.0–0.125 dense | French/conceptual queries: encoder language check | B (model) |

## 4. Anti-goals

- No cross-encoder reranker before first-stage recall is fixed (masks).
- No vector DB / HNSW service before the bench says brute force is the
  bottleneck (it is not: 9570 chunks score in milliseconds).
- No LLM judge in the loop before training-free signals plateau (cost).
- No new fixture queries until the current 18 are all explained (no
  hiding misses in averages over more queries).

## 5. Run economy: ladder, cut rules, relaunch triggers

Measured costs (cortex CPU, EmbeddingGemma-2 BF16, ~12 chunks/s):

| Rung | Corpus | Cold embed | Hot (cache hit) | Use |
|---|---|---|---|---|
| L0 smoke | subset 100 | ~15 s (est.) | seconds | Code-change gate |
| L1 screen | subset 1000 | ~90 s | ~30 s | Axes, thresholds, models |
| L2 freeze | full (~10.5k) | ~900 s | ~60–120 s | Freezing only |

### 5.1 Ladder discipline

- New/changed bench, fusion, or scorer code: L0 twice — outputs must be
  bit-identical, else no bigger run (determinism gate).
- New axis value (threshold, k, prefix): L1. Promote to L2 only on L1 win.
- New model: L1 screen per contract gates; L2 only for models beating
  reference.
- L2 runs freeze exactly one decision each. No exploratory L2.
- Reproducibility gate: same GGUF bytes, 3 runs, bit-identical means —
  a model that wobbles is out before calibration starts.
- Degradation gate: before adopting any model, a noise probe set (typos,
  jargon, terse queries) must not regress vs reference. A model gaining
  on clean queries but losing on noisy ones is not progress.

### 5.2 Cut rules (when to abort a run)

- L1 hybrid below BM25 by more than 0.10 with zero rescued queries:
  dead axis, cut, do not promote.
- Embed throughput collapses (>3× the ~12/s baseline): infra fault,
  cut and diagnose (server, not the bench).
- Determinism gate fails (L0 twice differ): cut everything, fix the
  nondeterminism first — all prior means are suspect.
- Full-corpus BM25 mean moves vs the frozen 0.574 by more than 0.02:
  corpus drifted under our feet — cut, diff the corpus, re-baseline.

### 5.3 Relaunch triggers (when cached numbers die)

| Change | Invalidates | Action |
|---|---|---|
| `scorer.rs` / `hybrid_rrf.rs` fusion logic | ALL hybrid/dense numbers | Re-run L1 baseline + L2 freeze |
| `retrieval_gain.rs` bench logic | ALL means | Re-run L1 baseline; L2 if frozen values move |
| Fixture grades/queries | Affected queries only | Re-run L1; report per-query deltas |
| Corpus content (chunk count delta) | ALL numbers | Re-baseline BM25 first (drift check §5.2) |
| New embedding model | That model's columns only | Screen per contract; others stand |
| Pipeline (prefix/template/max_chars) | That pipeline's vectors | Fresh cache file automatically; re-embed |

### 5.4 Fixture growth protocol (the "more data" rule)

Section 4's anti-goal stands with one door: new queries arrive in
pairs — one natural phrasing, one adversarial (paraphrase/trap) — each
with gold grades AND a trap sibling. Every addition re-runs L1 and must
not move frozen L2 values by more than 0.01, or the freeze reopens.
Target: 50 queries before any ML routing (Lex-Router's own floor).

Minimums, grounded in our data (per-query sd≈0.43, bimodal 0/1):

- Mean claims: ±0.05 needs ~250 queries (out of reach); ±0.10 needs
  ~60. At n=18 the mean CI spans ±0.2 — never compare two means
  directly.
- Comparison claims (Wilcoxon): 6 unanimous discriminating pairs reach
  p<0.05. Ties do not vote. At our 50% tie rate that means **30–40
  queries (≈15–20 discriminating pairs)** to settle a fusion claim.
- Growth runs in **batches of 6** (3 natural/adversarial pairs) aimed
  at disagreement zones (margin<0.05, dense-win, bm25-win) — power
  comes from informative queries, not volume. One L1 per batch (~90 s).
- Double stop rule: **success** (p<0.05 with same sign on two corpora)
  or **futility** (two batches without tightening — the effect is
  absent, cut). The dear cost is writing and grading, not running:
  batch, never drip-feed.
- Flow: proposals (human or `llm:<model>`) → `cynic know grades review`
  deterministic gate → human confirm → fixture. The gate checks
  structure and trap discipline; semantic truth stays human until the
  epistemic engine calibrates proposer weights (phase 3).

### 5.5 Statistics standard

With n=18, means do not conclude — paired per-query deltas do. Every
fusion claim reports wins/losses/ties plus the Wilcoxon signed-rank
statistic (captures direction AND magnitude; the sign test ignores
magnitude). Freeze requires p<0.05 AND same-sign wins on two corpora
(subset + full). A mean gain with p>0.1 is noise, however pretty.

### 5.6 Corpus pinning (learned 2026-10-14, the hard way)

The corpus is the repo we edit: every doc/code edit moves chunk ids, and
RRF top-5 swings ±0.1 for 5 changed chunks while BM25 holds 0.574 and
dense holds. Consequences:

- L2 freeze runs happen on a clean tree at a pinned commit. The report
  already carries `corpus_chunks`; the run procedure records HEAD.
- No file edits anywhere in the worktree between a baseline and its
  challenger (same-corpus rule). Our own calibration docs outrank gold
  (RQC2 fell 1.0→0.25 under our plan doc) — self-pollution is measured,
  not excluded.
- BM25 mean is the drift anchor: it held 0.574 across three corpora
  (10560/10580/10585 chunks). Any run where it moves >0.02 is cut.
- Subset screens undervalue dense (0.226 subset vs 0.457 full): model
  screens report both, decide on full.

## 6. Frozen verdicts (full corpus, deterministic code)

| Claim | Subset-1000 | Full | Wilcoxon | Verdict |
|---|---|---|---|---|
| RRF beats BM25 | 0.431 < 0.574 | 0.434 / 0.588 (corpus-conditional!) | — | **NOT FROZEN**: value swings ±0.1 with corpus; RRF is knife-edge on ties |
| Guard 0.1 beats RRF | 0.523 (+0.092) | +0.052 / -0.102 (corpus-conditional) | p≈0.31 / p≈0.59 | **REJECTED**: null on both corpora, sign flips with corpus |
| Dense scales with corpus | 0.226 | 0.457 | — | **CONFIRMED**: subset starves dense; screens must note the bias |
| BM25 stable across corpora | 0.574 | 0.574 ×3 | — | **CONFIRMED**: the drift anchor |

Where the guard dies, per query (10585 corpus, 5 wins / 4 losses / 9 ties):
fires-and-loses on RQ4/RQ6/RQ13 (dense was 1.0 — the one-sided gate
ignores dense confidence); fires-and-wins on RQ2b/RQ2c/RQC3/RQC2.
The specified next step is the two-sided gate (A3): fire only when BM25
is confident AND dense is not. RQ2b stays flagged: both sides know the
answer yet RRF ranks it #4 — an RRF-multiplier pathology (authority ×
contract-floor reorder fused ranks), audited separately, not patched
around.

External review (my-stack specialist, 2026-10-14) adopted: Wilcoxon as
standard (§5.5), RRF-k sweep stays A1, dense-margin stays A3, B1 first.
Corrected, not adopted: gold@20 never replaced MRR@5 — the report
carries both (graded top-5 + diagnostic depth). Added from review:
degradation gate (typo/jargon/noise queries before adopting a model)
and reproducibility gate (same GGUF, 3 runs, bit-identical) join §5.1;
per-model instruction/prefix risks join the selection contract.
