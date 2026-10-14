# RAG Calibration Plan — Organic / Bottom-Up

How CYNIC retrieval improves: data first, one variable at a time, each step
gated by the graded bench (`cynic_eval::bench::retrieval_gain`, 18 graded
queries + 1 trap). No default is baked in before full-corpus proof. No
architecture is proposed before the numbers demand it.

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
