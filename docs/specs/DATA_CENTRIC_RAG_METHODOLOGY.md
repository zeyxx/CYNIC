# CYNIC Data-Centric Bottom-Up RAG Specification & Methodology

**Schema:** `cynic.spec.data_centric_rag.v1`  
**Status:** Normative & Operational  
**Authority:** `contracts/foundation.json`, `contracts/inference-calibration.json`, `contracts/embedding-selection.json`

---

## I. WHAT (Core Philosophy & First Principles)

This specification defines the rigorous, data-centric, first-principles methodology governing CYNIC's Sovereign Matrix Knowledge Engine (Organic RAG). 

A State-Of-The-Art (SOTA) RAG is not an assemblage of ad-hoc prompt tricks, framework defaults, or arbitrary threshold patches. In CYNIC, **architecture is never decreed top-down; it emerges strictly from the empirical observation of physical substrate realities and the continuous mathematical properties of corpus data**.

Every single architectural choice must satisfy three non-negotiable axioms:
1. **Physical Substrate Primacy:** Compute and memory rooflines of the physical host (e.g. Cortex APU UMA DDR4 bandwidth, hardware VRAM/RAM budgets, zero swap) dictate the feasibility of models, dimensions, lot sizes, and indexing strategies.
2. **Data-Centric Representation:** Chunking, indexing, and ranking must map to the intrinsic grammar and syntax of the underlying data (AST tokens, public interfaces, normative contracts), never to arbitrary fixed-token windows.
3. **Continuous Signal & Statistical Discipline:** Retrieval signals (lexical term frequencies, dense cosine geometries) operate in continuous probability spaces. Coarse ordinal discretizations (e.g. unweighted RRF) and heuristic step-function cutoffs are rejected unless proven superior by paired hypothesis testing ($p < 0.05$ Wilcoxon signed-rank on discriminating query pairs).

---

## II. INVARIANTS & FALSIFIERS

1. **Substrate Allocation Ceiling:**
   $$\text{RAM}_{\text{allocated}} \le \text{RAM}_{\text{available}} - 1024\,\text{MB}_{\text{OS}}$$
   Zero swap permitted. Single-model execution policy strictly enforced on APU/UMA hardware.
2. **Zero-Hallucination & Provenance Invariant:**
   - Every synthesized assertion must resolve to an exact bit-coordinate: `[SOURCE N: path:line_start-line_end]`.
   - If evidence is absent from the retrieved chunk space, the output must formulate verbatim: `NON COUVERT PAR LE CORPUS ACTUEL`.
3. **Granularity Alignment (Chunk ID Primacy):**
   - Scoring, caching, and fusion operate at the individual AST fragment level (`chunk_id`), never averaged or squashed at the file-path level.
4. **Statistical Decision Gate:**
   - No model, fusion algorithm, or hyperparameter is frozen on an aggregate mean alone.
   - Adoption requires $N \ge 30$ discriminating query pairs evaluated under the Wilcoxon signed-rank test yielding $p < 0.05$ with consistent sign across two independent corpora (`subset` and `full`).
5. **Corpus Immutability During Benchmarking:**
   - Benchmarking and calibration runs must execute against a clean, pinned Git commit (`HEAD`). Run reports must record the exact `corpus_chunks` count and tree hash to prevent self-referential cache pollution.

---

## III. SEVEN-PHASE DATA-CENTRIC CHECKLIST

```
[ L0 Physical Substrate ]  -> RAM roofline, UMA bandwidth, SIMD vector scan, GGUF quants
            │
[ L1 Topologic AST ]       -> AST grammar parsing, hierarchical contract mapping, chunk_id
            │
[ L2 Latent Space ]        -> Cosine anisotropy, Spearman rank correlation, prompt templates
            │
[ L3 First-Stage Retr. ]   -> Chunk-level BM25 vs isolated dense, independent Recall@K
            │
[ L4 Continuous Fusion ]   -> ECDF calibration, Min-Max score normalization, convex fusion
            │
[ L5 Epistemic Grounding]  -> Confidence floor, strict line-range attribution, context ratio
            │
[ L6 Decision Metrology ]  -> Dual-query test fixtures (n>=40), Wilcoxon p<0.05, frozen trees
```

### Chapter 1: Physical Substrate & Hardware Roofline Profiling (L0)
- [ ] **1.1 UMA Memory Roofline & Memory Budget:**
  - Profile hardware memory bandwidth ($GB/s$) and latency on target host (Cortex APU DDR4).
  - Enforce hardware admission: $\text{GGUF Weights} + \text{KV Buffers} + \text{Inference Lot} \le \text{Host Limit}$.
- [ ] **1.2 In-Memory Brute-Force Vector Scan Roofline:**
  - Profile brute-force cosine similarity over $N$ chunks in pure Rust (AVX2/SIMD).
  - Invariant: Zero external vector database or HNSW daemon introduced while RAM brute-force scan executes in $< 10\,\text{ms}$ for $N \le 50\,000$.
- [ ] **1.3 Bit-Identical Model Reproducibility:**
  - Verify that identical GGUF weights executed twice over the same input batch produce bit-identical FP32 vector projections ($L_2$ error $= 0.0$).

### Chapter 2: Data Anatomy & Structural AST Ingestion (L1)
- [ ] **2.1 Corpus Length & Type Distribution Mapping:**
  - Profile the empirical distribution of document lengths, Markdown section depths, and Rust code item sizes across the workspace.
- [ ] **2.2 Grammar-Driven Chunking (Zero Arbitrary Token Splitting):**
  - Rust code: Chunk boundaries strictly follow AST nodes (`struct`, `enum`, `fn`, `impl`) via `syn`/`tree-sitter`, prepending docstrings.
  - Markdown: Chunk boundaries strictly follow semantic section headers (`#`, `##`, `###`), preserving code block integrity.
- [ ] **2.3 Chunk Identity & Line-Coordinate Pinning:**
  - Every chunk receives an immutable `chunk_id` composed of `hash(source_path, line_start, line_end, content_digest)`.
  - Deprecate file-path level aggregation in favor of unique `chunk_id` identification.

### Chapter 3: Latent Space Geometry & Embedding Metrology (L2)
- [ ] **3.1 Vector Space Anisotropy Measurement:**
  - Compute pairwise cosine similarity distribution over 1,000 random corpus pairs to measure the representation cone effect.
  - Apply $L_2$ normalization or mean-centering if severe directional collapse ($\mu_{\cos} > 0.7$) is observed.
- [ ] **3.2 Quantization Distortion Assessment (BF16 vs Q8 vs Q4):**
  - Compute Spearman rank correlation of distances across quantization tiers on 1,000 reference pairs before assessing retrieval tasks.
  - Ensure quantization preserves nearest-neighbor topology.
- [ ] **3.3 Model-Card Protocol Ingestion:**
  - Ensure model-specific instruction formatting (`retrieval.passage:`, `retrieval.query:`, template strings) is codified as pure data in `TextPipeline` and hashed into vector cache keys.

### Chapter 4: First-Stage Recall & Modality Decomposition (L3)
- [ ] **4.1 Independent Modality Evaluation:**
  - Evaluate Sparse (BM25) and Dense (Cosine) independently across all benchmark queries at depths $K \in \{5, 10, 20, 50\}$.
- [ ] **4.2 Failure Mode Classification Matrix:**
  - Categorize every retrieval miss into one of three structural classes:
    1. *Exact technical identifier miss* (Lexical failure due to stemming/tokenization).
    2. *Conceptual / Paraphrase miss* (Dense failure due to encoder blindspot).
    3. *Vocabulary disconnect* (Shared miss requiring query reformulation/expansion).

### Chapter 5: Signal Theory & Continuous Score Fusion (L4)
- [ ] **5.1 Empirical Cumulative Distribution Function (ECDF) Mapping:**
  - Plot and fit the real score distributions of BM25 (open-ended heavy-tailed) vs Dense Cosine ($[-1, 1]$ bounded).
- [ ] **5.2 Continuous Score Normalization (Min-Max & Z-Score):**
  - Implement and evaluate continuous normalization over top-$K$ candidates rather than discarding score magnitudes via ordinal ranks.
- [ ] **5.3 Calibrated Convex Combination:**
  - Implement continuous linear combination:
    $$S(d) = \alpha \cdot \hat{S}_{\text{dense}}(d) + (1 - \alpha) \cdot \hat{S}_{\text{bm25}}(d)$$
  - Measure performance of convex combination against ordinal Reciprocal Rank Fusion (RRF).
- [ ] **5.4 Conditional Two-Stage Re-Ranking:**
  - Evaluate cross-encoder re-ranking strictly if and only if first-stage fused Recall@20 reaches an empirical plateau.

### Chapter 6: Epistemic Grounding & Source Synthesis (L5)
- [ ] **6.1 Epistemic Coverage Thresholding:**
  - Measure the empirical minimum confidence score below which LLM answers degrade into hallucination.
  - Reject queries below this threshold with `NON COUVERT PAR LE CORPUS ACTUEL`.
- [ ] **6.2 Topological Altitude Hierarchy:**
  - Ensure retrieved context is ordered by CYNIC constitutional hierarchy ($L_0$ Constitution $\succ L_1$ Contract $\succ L_2$ Spec $\succ L_3$ Operational Code).
- [ ] **6.3 Context Budget & Lost-in-the-Middle Optimization:**
  - Measure effective signal-to-noise ratio within the assembled prompt, minimizing distracting irrelevant context.

### Chapter 7: Metrology, Hypothesis Testing & Ground Truth Growth (L6)
- [ ] **7.1 Dual-Query Fixture Maturation ($N \ge 40$):**
  - Grow the test fixture in balanced pairs: 1 natural query + 1 adversarial counterpart (paraphrase or out-of-corpus trap).
  - Ground truth labels must target exact `chunk_id` coordinates, not file paths.
- [ ] **7.2 Wilcoxon Signed-Rank Decision Protocol:**
  - Report paired wins, losses, ties, and exact p-values for all candidate changes.
  - Freeze rule: $p < 0.05$ on both `subset-1000` and full corpus.
- [ ] **7.3 Anti-Pollution Commit Locking:**
  - Execute calibration runs on detached clean worktrees at fixed commit hashes. Exclude generated benchmark outputs from the indexed corpus.

---

## IV. WHY (First Principles & Architectural Trade-offs)

1. **Why Continuous Score Space over Ordinal RRF?**
   - Reciprocal Rank Fusion ($1/(k + \text{rank})$) was designed as a coarse heuristic when search engine score distributions are incomparable or unavailable. In an integrated sovereign runtime, raw BM25 scores and vector similarities are fully accessible. Discarding score magnitudes destroys confidence margins, forcing the system to rely on fragile step-function patches.
2. **Why Chunk-ID Granularity over File-Path Aggregation?**
   - CYNIC documentation files contain heterogeneous sections ranging from constitutional invariants to detailed CLI usage. Merging chunk scores by file path contaminates focused technical answers with unrelated document prose and makes precision citations impossible.
3. **Why Statistical Hypothesis Testing over Mean MRR?**
   - On small query sets ($N \le 20$), individual query swings produce massive changes in arithmetic means ($1/N \approx 5.5\%$). A single lucky match can simulate architectural improvement. Paired non-parametric tests (Wilcoxon) test whether the distribution of improvements consistently dominates regressions.

---

## V. FALSIFIERS & STATISTICAL GATES

| Checkpoint | Metric / Observable | Falsification Condition (Rejection) |
|---|---|---|
| **Substrate Safety** | RSS Memory Peak | Peak RSS $> 3.0\,\text{GB}$ or any page fault swap $\rightarrow$ Instant Abort |
| **Quantization Fidelity** | Spearman $\rho$ (BF16 vs Quant) | $\rho < 0.95$ on 1,000 paired sample vectors $\rightarrow$ Reject Quantization |
| **Corpus Stability** | BM25 Drift Anchor | Mean BM25 MRR moves $> \pm 0.02$ across identical runs $\rightarrow$ Tree Polluted |
| **Model Adoption** | Wilcoxon Signed-Rank | $p \ge 0.05$ or contradictory win/loss sign between subset and full $\rightarrow$ Reject Model |
| **Epistemic Integrity** | Negative Question Trap | Any hallucinated claim on trap queries ($> 0.0\%$) $\rightarrow$ Instant Gate Failure |
