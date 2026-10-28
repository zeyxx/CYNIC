# CYNIC Specification: RAG Enhancement — LiquidAI LFM3 & Graph-First Topology

**Schema:** `cynic.spec.rag_enhancement.v1`  
**Status:** Ready for Agentic Delegation  
**Authority:** `contracts/foundation.json`, `docs/specs/DATA_CENTRIC_RAG_METHODOLOGY.md`, `infra/registry.json`  
**Date:** 2026-10-28  

---

## 1. Context & Motivation

CYNIC's knowledge layer currently operates on a hybrid storage substrate combining **SurrealDB 3.1.5** (hosting document chunks and graph edges) and a remote inference node **`cynic-compute-01`** (`192.168.0.25`).

While the baseline RAG pipeline functions reliably with zero-panic thread isolation, two major architectural bottlenecks limit its epistemic strength:
1. **Generation Model Latency and Agentic Horizon:** The current inference engine runs `LFM2.5-2.6B-Q4_K_M.gguf`. LiquidAI has released **`LFM3-1.7B`** (tri-hybrid architecture combining gated short convolutions, sliding-window attention, and global attention), featuring a **256K-token context window**, native agentic post-training (for harnesses like Hermes Agent and OpenClaw), and significantly lower RAM footprint (~1.0 GB in 4-bit, ~1.8 GB in Q8_0).
2. **Topological Blindness of Flat Chunk Retrieval:** As demonstrated in empirical literature (*arXiv:2609.12464*, *arXiv:2502.06781*, and *Context+*), naive vector similarity severs the structural links of code and contracts (e.g. parent modules, inheritance, enforcement chains, dependency bounds), leading to high hallucination rates on architecture queries. SurrealDB already stores graph relations (`doc->enforces->doc`, `doc->depends_on->doc`, `doc->cites->doc`, `doc->contains_chunk->chunk`). Traversing these edges at query time unlocks true **Graph-First Topological Retrieval**.

---

## 2. Invariants & Substrate Ceilings

1. **Substrate Allocation Ceiling (`cynic-compute-01` 192.168.0.25):**
   - Physical RAM: 16 GB (AMD Ryzen 7 5700U with Radeon Lucienne APU).
   - Port `:8080` (Embedding, `embeddinggemma-2`, 768-dim, `-c 8192`): ~2.6 GB VRAM/RAM.
   - Port `:8082` (Reranker, `Qwen3-Reranker-0.6B-Q8_0`): ~2.2 GB VRAM/RAM.
   - Port `:8088` (Inference): The new model `LFM3-1.7B-Q8_0.gguf` (~1.8 GB) or `LFM3-1.7B-Q4_K_M.gguf` (~1.0 GB) must not exceed a working RAM ceiling of `4.0 GB` with a context size `-c 32768` (or `-c 65536`).
   - Total host RAM utilization across all three servers must remain $\le 12.0\,\text{GB}$, leaving at least $4.0\,\text{GB}$ uncommitted for OS and builders.
2. **Zero In-Process Tokio Drops:**
   - Any HTTP calls to the inference engine or SurrealDB must remain strictly isolated on dedicated worker threads (`std::thread::spawn(move || ...).join()`) when initiated from CLI or sync entry points, preventing Tokio runtime panics.
3. **Epistemic Authority Invariant:**
   - Precedence order declared in `contracts/foundation.json` must be preserved:
     $$\text{L0Constitution} \succ \text{L1Specification} \succ \text{L2Design} \succ \text{L3Operation} \succ \text{L4Observation}$$
   - Chunks belonging to higher authority levels maintain score precedence multipliers during reranking.
4. **Verifiable Citation & Grounded Truth:**
   - Output must resolve to bit-coordinates: `[SOURCE N: path:start_line-end_line]`.
   - When context does not contain verifiable proof: emit verbatim `NON COUVERT PAR LE CORPUS ACTUEL`.

---

## 3. Architecture & Query Topology

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Operator / Agent Query                          │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
                 [ 1. Dense Embedding (768-dim) ]
                     http://192.168.0.25:8080/v1
                                    │
                                    ▼
       [ 2. SurrealDB Vector & Graph Traversal (10.10.10.103:8000) ]
        a) Top-K Chunk Selection via HNSW vector index:
           SELECT chunk_id, doc_path, authority, body,
                  vector::similarity::cosine(embedding, $q_vec) AS score
           FROM chunk ORDER BY score DESC LIMIT 10;
                                    │
                                    ▼
        b) Topological Context Expansion (1-hop Graph Traversal):
           FOR EACH chunk:
             - Parent Doc metadata & full header breadcrumbs:
               <-contains_chunk<-doc.{path, authority, title}
             - Inbound / Outbound Normative Relationships:
               ->enforces->doc.path AS enforces
               ->depends_on->doc.path AS depends_on
               ->cites->doc.path AS cites
               <-enforces<-doc.path AS enforced_by
               <-depends_on<-doc.path AS depended_on_by
                                    │
                                    ▼
                 [ 3. Topological Prompt Synthesis ]
         Inject Chunk text + Structural Graph Metadata Header:
         === [SOURCE N] <path> (<lines>, <authority>) ===
         Structure: Part of <doc_title>
         Enforces: [<doc_path>, ...] | Depends On: [<doc_path>, ...]
         Content: <chunk_body>
                                    │
                                    ▼
       [ 4. Streaming Synthesis via LFM3-1.7B (192.168.0.25:8088) ]
         Tri-hybrid 256K context engine streams grounded answer.
```

---

## 4. Work Breakdown for Delegated Agents

The implementation is broken down into four isolated, mechanically checkable tasks.

### Task T1: Staging & Deployment of LFM3-1.7B on `cynic-compute-01`
- **Target Host:** `192.168.0.25` (principal `cynic`, SSH key verified).
- **Actions:**
  1. Download model artifact `LiquidAI/LFM3-1.7B-GGUF` (preferred quant: `LFM3-1.7B-Q8_0.gguf` for precision, fallback: `LFM3-1.7B-Q4_K_M.gguf` for lowest memory):
     ```bash
     huggingface-cli download LiquidAI/LFM3-1.7B-GGUF LFM3-1.7B-Q8_0.gguf \
       --local-dir /home/titouan/cynic/models/
     ```
  2. Inspect the supervisor for PID 1163 (`/home/titouan/cynic/bin/llama-server`) and update service configuration or launch script:
     ```bash
     /home/titouan/cynic/bin/llama-server \
       -m /home/titouan/cynic/models/LFM3-1.7B-Q8_0.gguf \
       --host 0.0.0.0 --port 8088 -c 32768 -t 12 --parallel 1
     ```
  3. Verify health endpoint: `curl -s http://192.168.0.25:8088/health | jq .`.
  4. Update `infra/registry.json` node `cynic-compute-01` service `inference-server`:
     - `artifact`: `"llama-server (LFM3 1.7B Q8_0, Tri-Hybrid 256K)"`
     - `model`: `"LFM3-1.7B-Q8_0.gguf"`

### Task T2: SurrealDB Graph Traversal Adapter in `crates/cynic-storage`
- **Crate:** `crates/cynic-storage`
- **File:** `crates/cynic-storage/src/adapters/surreal_db.rs`
- **Actions:**
  1. Define `TopologicalContext` struct:
     ```rust
     #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
     pub struct TopologicalContext {
         pub parent_path: String,
         pub parent_authority: String,
         pub enforces: Vec<String>,
         pub depends_on: Vec<String>,
         pub cites: Vec<String>,
         pub enforced_by: Vec<String>,
         pub depended_on_by: Vec<String>,
     }
     ```
  2. Implement batch query method:
     ```rust
     pub async fn fetch_topological_context(&self, chunk_ids: &[String]) -> Result<HashMap<String, TopologicalContext>, StorageError>
     ```
     Using single-statement SurrealQL:
     ```surql
     SELECT id,
            <-contains_chunk<-doc.path AS parent_path,
            <-contains_chunk<-doc.authority AS parent_authority,
            <-contains_chunk<-doc->enforces->doc.path AS enforces,
            <-contains_chunk<-doc->depends_on->doc.path AS depends_on,
            <-contains_chunk<-doc->cites->doc.path AS cites,
            <-contains_chunk<-doc<-enforces<-doc.path AS enforced_by,
            <-contains_chunk<-doc<-depends_on<-doc.path AS depended_on_by
     FROM chunk WHERE id IN $ids;
     ```

### Task T3: Synthesis Prompt Enrichment in `crates/cynic-knowledge`
- **Crate:** `crates/cynic-knowledge` (and corresponding CLI hook in `cynic-cli`)
- **Actions:**
  1. Integrate `TopologicalContext` into `KnowledgeChunkMatch` / prompt formatting.
  2. Format the source block in the epistemic system prompt:
     ```text
     === [SOURCE 1] docs/foundation/CONSTITUTION.md (Lignes 29-33, Autorité: L0Constitution) ===
     [TOPOLOGIE: Applique à: crates/cynic-kernel | Dépendances requises: [] | Cité par: 12 specs]
     ... body ...
     ```
  3. Ensure strict adherence to the 5 Epistemic Directives (truth grounded, exact citation, precedence, explicit limit honesty, no slop).

### Task T4: Verification, Gates & Calibration Benchmark
- **Actions:**
  1. Run `cynic storage ping --json` and `cynic storage query --sql "SELECT count() FROM chunk GROUP ALL;"`.
  2. Run `cynic know ask "Quelles sont les obligations de séparation des pouvoirs ?" --prompt-only` to verify presence of topological metadata in the prompt.
  3. Run live inference with `LFM3-1.7B` on `192.168.0.25:8088` and verify latency, memory usage on `cynic-compute-01`, and response accuracy.
  4. Ensure all gates pass: `cynic audit all --json`.

---

## 5. Review & Acceptance Criteria

- [ ] LFM3-1.7B serving on `192.168.0.25:8088` with health status `pass`.
- [ ] RAM on `cynic-compute-01` remains strictly below 12 GB under load.
- [ ] SurrealDB executes topological traversal in under 10 ms per query.
- [ ] `cynic know ask` cites both content line coordinates and topological relationships.
- [ ] Zero compiler warnings (`-D warnings` enforced on all touched crates).
- [ ] Store convergence maintained (`cynic audit all`).
