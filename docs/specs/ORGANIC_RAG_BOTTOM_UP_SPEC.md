# Organic Bottom-Up RAG Specification & Implementation Checklist

**Schema:** `cynic.spec.organic_rag.v1`  
**Status:** Normative & Operational  
**Authority:** `contracts/foundation.json`, `contracts/inference-calibration.json`  

---

## I. WHAT (Functional & Structural Specification)

This specification defines the architecture, data structures, and phased delivery of CYNIC's Sovereign Matrix Knowledge Engine (Organic RAG). It operationalizes epistemic truth retrieval across the sovereign organism's multi-layered documentation, AST fragments, contracts, and code.

The system replaces ad-hoc LLM prompting with a strictly grounded, deterministic retrieval-augmented generation engine operating under physical host constraints (RAM, memory bandwidth, compute rooflines).

---

## II. INVARIANTS & FALSIFIERS

1. **Physical Substrate Priority (Bottom-Up Invariant):**
   - Memory allocation must never exceed physical host capacity.
   - Admission rule: $AvailableRAM \ge (Weights + KVCache + Activations + 20\% Margin) + 512MB_{OS}$.
   - Single-model execution policy strictly enforced on unstable/APU hosts (e.g. Cortex UMA DDR4).
2. **Epistemic Grounding (Zero-Hallucination Invariant):**
   - If evidence is absent from retrieved chunks, output must strictly formulate: `NON COUVERT PAR LE CORPUS ACTUEL`.
   - Any synthesized claim must be accompanied by explicit source coordinates: `[SOURCE N: path:line_start-line_end]`.
3. **Pure Domain Boundary:**
   - Retrieval scoring, prompt assembly, and graph topology traversal must remain 100% pure (zero network, zero shell, no local compilation on unstable host).
4. **Incremental Ingestion:**
   - Disk scanning must be backed by a persistent cache. Only modified Git blobs (hash/mtime drift) may be re-parsed during interactive querying.

---

## III. IMPLEMENTATION PHASES & CHECKLIST

### Phase 1: Physical Ingestion & Persistent Cache (L0 Substrate)
- [x] **1.1 Persistent Knowledge Cache Format:**
  - Define `cynic.knowledge.cache.v1` serializable to `CynicPaths::state_dir().join("knowledge/corpus_cache.json")`.
  - Store chunk vectors, AST tokens, authority levels, and Git blob SHA-1/SHA-3 hashes.
- [x] **1.2 Incremental Git-Diff Ingestion Engine:**
  - Fast-path verification: compare repository commit SHA / file mtimes.
  - If unmodified: load cached index in $< 5\text{ ms}$.
  - If modified: selectively chunk and index only changed files (`git diff --name-only`).
- [x] **1.3 Rust Code AST Chunking (Beyond Markdown):**
  - Extract public structs, enums, functions, and docstrings from `crates/*/src` via Tree-sitter / syn.
  - Map code symbols to owning crate layer ($L_0$ to $L_4$).

### Phase 2: Graph-RAG & Structural Topology Traversal (L1 Truth Graph)
- [x] **2.1 Dependency Expansion Traversal:**
  - When a chunk is selected by lexical retrieval, query `KnowledgeGraph` (`crates/cynic-knowledge/src/domain/graph.rs`).
  - Automatically fetch parent contracts (`contracts/*.json`) and architectural constraints linked to the document.
- [x] **2.2 Authority tiers (rows of the matrix, live):**
  - `AuthorityTier` (`crates/cynic-knowledge/src/domain/chunk.rs`) has **four**
    rows, not five: `L0Constitution` (foundation invariants), `L1Specification`
    (specs, contracts), `L2Design` (design docs, RFCs), `L3Operation`
    (operational notes, captures). Registry downgrades in
    `contracts/authority-tiers.json` (`cynic.knowledge.authority_tiers.v1`).
  - Authority is stored per chunk and used for **conflict ordering** in graph
    expansion (L0 outranks lower tiers); there is **no multiplicative
    re-ranking** in the current code — the formula below is a target, kept
    here so the gap stays visible:
    $$Score = BM25 \times Boost_{Authority} \times Boost_{Layer} \times Freshness_{Git}$$
- [ ] **2.3 Confidence columns ($C_0..C_4$, target — no code counterpart yet):**
  - $C_0$ retrieved (a chunk came back for the query);
  - $C_1$ lexically relevant (graded relevance $\ge 1$ on the fixture);
  - $C_2$ authority-resolved (conflicts settled toward the higher tier);
  - $C_3$ corroborated (linked to its governing contract via `KnowledgeGraph`);
  - $C_4$ admitted as proof (usable as evidence in a gated decision).
  - A memory item moves from one column to the next only through these
    explicit passages, never by default. No passage is implemented to date:
    only the $C_0 \to C_1$ measurement exists (fixture MRR, see Phase 3).
- [x] **2.4 Contradiction & Deprecation Pruning:**
  - Filter out obsolete documents flagged in `contracts/foundation.json` deleted authorities.

### Phase 3: Dense + Sparse Hybrid Retrieval (L2 Semantic)
- [x] **3.1 Sovereign Embedding Model Integration:**
  - `embeddinggemma-2-bf16` served by llama-server on `cynic-lxc-embedding`
    (lxc/106, `10.10.10.106:8080`, DIM 768); consumed through
    `cynic-inference::SyncEmbeddingBridge` (PR 79, unmerged at time of writing;
    private runtime, dimensions pinned at probe, drift refused) implementing
    knowledge's sync `ChunkEmbedder`.
  - Server constraint observed live: physical batch 512 tokens — inputs above
    it fail with HTTP 500, so measurement inputs are front-truncated.
- [x] **3.2 Reciprocal Rank Fusion (RRF):**
  - Combine Sparse BM25 ranks and Dense Vector cosine similarity:
    $$RRF(d) = \sum_{m \in \{BM25, Dense\}} \frac{1}{k + rank_m(d)}$$
  - Measured on the graded fixture (`retrieval_gain_labels.json`,
    10 pairs / 1 trap / 9 scored, `target/*_gain_measure.json` reports):

    | Backend | Mean MRR (9 queries) | Delta vs BM25 |
    |---|---|---|
    | BM25 only | 0.222 | — |
    | Hybrid + hashed-token stub | 0.133 | -0.089 |
    | Hybrid + neural (EmbeddingGemma-2) | pending | pending |
  - Reading: fusion mechanics proven deterministic; the stub dense signal
    degrades ranking, so the neural run is the one that decides whether dense
    earns its keep. Gain direction stays warn-only until a neural delta lands.

### Phase 4: Production-Grade Oracle & Agent Exposure (L3 Surface)
- [ ] **4.1 In-Tree MCP Exposure (`cynic-mcp`):**
  - Expose `cynic_know_ask` tool over MCP protocol with JSON schema input/output.
  - Allow Hermes, Claude, and local subagents to query living memory natively.
- [ ] **4.2 Streaming CLI Ergonomics:**
  - Streaming SSE with token-by-token output and interactive cancellation.
- [x] **4.3 Epistemic Evaluation Harness (Faithfulness & Relevance):**
  - Automated CI test matrix executing 10 deterministic test questions (invariants, specs, negative out-of-corpus questions).
  - Verify 100% negative rejection rate on hallucination traps.

---

## IV. WHY (Architectural Decisions & Trade-Offs)

1. **Why Incremental Ingestion First?**
   - Re-parsing 287 Markdown files and contracts on every prompt causes noticeable latency and wasted compute. A persistent cache makes queries instantaneous and scalable to the entire Rust codebase.
2. **Why Graph-RAG over Naive Chunking?**
   - Documentation in CYNIC is hierarchical ($L_0$ Constitution down to $L_4$ Runtime). A lone chunk lacks legitimacy unless connected to the contract that governs it.
3. **Why Hybrid BM25 + Dense?**
   - BM25 excels at exact technical identifiers (`CynicPaths`, `LFM2.5`, `Vulkan0`), while dense embeddings capture conceptual inquiries (*"how does mesh networking establish peer consensus?"*). Combining both eliminates blind spots.

---

## V. FALSIFIERS & ACCEPTANCE CRITERIA

1. **Benchmark Acceptance:**
   - Ingestion time for unchanged corpus: $< 10\text{ ms}$.
   - Negative question hallucination rate: exactly $0.0\%$.
   - Grounding citation precision: $100\%$ of claims back-referenced to valid line ranges.
2. **CI Gates:**
   - All tests must compile and pass cleanly on remote builder (`10.10.10.119`) via `cargo test -p cynic-knowledge`.
   - Zero hardcoded paths; strictly governed by `CynicPaths` and `infra/directory-tree.json`.
