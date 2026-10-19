# CYNIC Knowledge Tensor Architecture — Organic Data-Centric Design

- **schema**: `cynic.knowledge.tensor_architecture.v1`
- **status**: `ratified_draft`
- **doctrine**: Epistemic Bottom-Up, Organic Growth, Machine Proof over Speculation
- **target_crate**: `cynic-knowledge` (Domain & Adapters), `cynic-cli` (Operator Surface)

---

## 1. Executive Summary & Epistemic Thesis

A sovereign system is defined strictly by what it **produces, proves, and enforces**, never by what it claims.
Standard RAG architectures fail in systems engineering because they model retrieval as an unstructured flat vector similarity pipeline. This causes:
1. **Semantic Hallucinations on Code**: Vector distance is fuzzy and hallucinates identifiers, struct names, and error codes.
2. **Temporal Contamination**: Stale documents and obsolete designs are retrieved with equal weight alongside active invariants.
3. **Epistemic Flattening**: Speculative human notes are treated with the same authority as machine-verified proofs ($L_0$ contracts).
4. **Boundary Leaks**: Internal node configurations and secrets leak into public surfaces.

To eliminate these failure modes, CYNIC models its knowledge base not as a linear pipeline, but as a **6-Dimensional Epistemic Tensor**.

---

## 2. The 6-Dimensional Epistemic Tensor ($\mathbf{K} \in \mathbf{S} \times \mathbf{C} \times \mathbf{T} \times \mathbf{E} \times \mathbf{R} \times \mathbf{P}$)

Every atomic unit of knowledge in CYNIC is anchored by a 6-tuple coordinate:

$$\text{KnowledgeUnit} = \langle S_i, C_j, T_k, E_l, R_m, P_n \rangle$$

### 2.1 Dimension $\mathbf{S}$: Substrate Source
The raw physical substrate providing the information:
- **$S_0$ (Contracts & Constitution)**: `contracts/*.json`, `AGENTS.md`, `CONSTITUTION.md`.
- **$S_1$ (Corpus Documentation)**: `docs/**/*.md` (Specs, Designs, Checklists, Foundations).
- **$S_2$ (VCS History)**: Git commit DAG, commit messages, diff hunks, author signatures.
- **$S_3$ (Code Substrate)**: Rust source files (`crates/**/*.rs`), syn AST, traits, dependencies.
- **$S_4$ (Agentic Trajectories)**: Frozen SQLite captures, Lamport tool Posets, CoT traces, compactions.

### 2.2 Dimension $\mathbf{C}$: Capability & Retrieval Mode
The retrieval mechanism applied to the substrate:
- **$C_0$ (Deterministic AST Ingestion)**: PageIndex structural decomposition without loss.
- **$C_1$ (Exact Lexical Engine)**: In-memory BM25, token frequencies, identifier matching ($0.00, 0 GPU).
- **$C_2$ (Causal & Structural Graph)**: DAG traversal, parent-child edges, cross-document links.
- **$C_3$ (Dense Semantic Projection)**: Vector embeddings (HNSW cos-distance) for conceptual intents.
- **$C_4$ (Synthesis & Reasoning)**: LLM-backed answer formulation strictly grounded in retrieved anchors.

### 2.3 Dimension $\mathbf{T}$: Temporal Validity & Decay
The timeline and revocation lifecycle of the fact:
- **`Active`**: Currently in force at `HEAD`.
- **`SupersededBy(SHA)`**: Formally revoked or replaced by a subsequent decision or commit.
- **`Historical`**: Archival capture of past state (immutable receipt).

### 2.4 Dimension $\mathbf{E}$: Epistemic Authority Tier
The verification rigor governing the fact:
- **$E_0$ (Physical Machine Proof)**: Exit code 0, cargo check pass, BLAKE3 hash match, verifiable receipt.
- **$E_1$ (Constitutional Law)**: Explicit repository contracts (`contracts/repository-governance.json`).
- **$E_2$ (Formal Specification)**: Architectural specs (`docs/specs/`).
- **$E_3$ (Exploratory Design)**: Proposals and designs (`docs/design/`).
- **$E_4$ (Agent Self-Report)**: Unverified claim from a model (Zero trust until machine verified).

### 2.5 Dimension $\mathbf{R}$ : Scope Resolution
The granularity scale of the retrieval:
- **Macro**: System-wide architectural invariants and constitutions.
- **Meso**: Crate, subsystem, document section, module.
- **Micro**: Symbol, struct field, exact error string, code line.

### 2.6 Dimension $\mathbf{P}$: Perimeter & Isolation Boundary
The security boundary governing disclosure:
- **`Public`**: Suitable for GitHub and external publication.
- **`FleetInternal`**: Internal cluster infrastructure (Synapse WireGuard, Proxmox topologies).
- **`Privileged/Secret`**: Key material, authority credentials, node signing keys.

---

## 3. Organic Bottom-Up Maturation Ladder

In accordance with Gall's Law and the **Bottom-Up over Top-Down** invariant in `AGENTS.md`, capabilities mature strictly from low-complexity deterministic layers to high-complexity probabilistic layers:

```
                  THE ORGANIC KNOWLEDGE MATURATION LADDER

  Phase 5: Agentic Session Trajectories (Lamport Poset, CoT, Compaction N->1)
     ▲
  Phase 4: Code Substrate AST & Type Knowledge (syn, traits, crate boundaries)
     ▲
  Phase 3: Git VCS Commit History (Parent SHA DAG, commit diffs, attribution)
     ▲
  Phase 2: Cross-Document Causal Graph (MarkdownLink extraction, contract->test wiring)
     ▲
  Phase 1: Deterministic Document & Contract Corpus (158 docs, BM25, Coverage Ratchet)
           [STATUS: PHASE 1 COMPLETED AND SEALED ON MAIN]
```

---

## 4. Phase-by-Phase Roadmap & Acceptance Gates

### Phase 1: Document & Contract Corpus (COMPLETED)
- **Delivered**: `cynic-knowledge` domain crate.
- **Substrates**: $S_0$ (Contracts) and $S_1$ (158 Markdown documents).
- **Capabilities**: $C_0$ (AST tree parsing), $C_1$ (BM25 lexical search with $L_0$ authority boost).
- **Physical Proof**: 158/158 documents parsed (2 244 sections, 489 code blocks, 1.49 MB), `corpus_coverage_ratchet` test passing on builder, `cynic know search` and `cynic know manifest` live.

### Phase 2: Structural & Causal Graph ($S_1 \times C_2$)
- **Goal**: Transform the flat collection of 2 244 sections into a queryable knowledge graph.
- **Components**:
  - Resolve intra-repository `MarkdownLink` targets into explicit cross-edges.
  - Link normative contracts to their verifying test files (e.g. `contracts/repository-governance.json` $\xrightarrow{\text{enforced\_by}}$ `tests/repository_governance.rs`).
- **Acceptance Gate**: Deterministic link-integrity test proving zero broken internal references across all 158 documents.

### Phase 3: Git Commit History & Temporal Validity ($S_2 \times \{C_0, C_1, C_2\}$)
- **Goal**: Ingest the Git commit DAG as live causal memory.
- **Components**:
  - Ingest Git commits (commit SHA, author, timestamp, message, touched files).
  - Enable multi-substrate BM25 search across docs AND commit messages.
  - Track `SupersededBy` temporal invalidations.
- **Acceptance Gate**: CLI command `cynic know log <path>` or `cynic know search --commits` returning exact commit provenance in $< 20$ ms.

### Phase 4: Code Substrate AST ($S_3 \times \{C_0, C_1, C_2\}$)
- **Goal**: Ingest Rust crates, modules, traits, and public functions.
- **Components**:
  - Rust AST symbol indexing without compiling (pure syn lexical parsing).
  - Map symbols to their owning crate and layer in `infra/crate-topology.json`.
- **Acceptance Gate**: Search query `cynic know symbol "DocumentChunk"` instantly locating file and line without ripgrep.

### Phase 5: Agentic Session Trajectories ($S_4 \times \{C_0..C_4\}$)
- **Goal**: Dogfooding on the 325 000 historical messages of CYNIC agents.
- **Components**:
  - Reconstruct Lamport Poset tool graphs and context compactions ($N \rightarrow 1$).
  - Extract proven problem-solution trajectories into the living knowledge base.
- **Acceptance Gate**: Causal query reconstructing the exact reasoning trace that produced any given commit.

---

## 5. Architectural Invariants

1. **Deterministic Baseline First**: No dense vector embedding ($C_3$) or generative LLM synthesis ($C_4$) may be introduced on a substrate before its deterministic AST ($C_0$) and exact lexical index ($C_1$) are verified and ratcheted.
2. **Authority Strictness**: A search result with $E_0$ (Machine Proof) or $E_1$ (Constitution) strictly outranks an $E_3$ (Design proposal) or $E_4$ (Model claim).
3. **Lossless Conservation**: Chunks and AST nodes must deterministically cover the raw bytes of the source file without silent dropped lines.
4. **Zero-I/O Domain Core**: All graph traversal, scoring, and ranking algorithms in `cynic-knowledge` reside in pure domain modules decoupled from filesystem and network side effects.
