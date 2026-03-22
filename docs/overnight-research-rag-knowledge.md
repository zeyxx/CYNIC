# Overnight Research: RAG, Knowledge Management, and Observation Pipelines
*Date: 2026-03-21 — Research-only session, no code modified*

Scope: 9 repos analyzed for architectures applicable to CYNIC's CCM (Contextual Crystallization Memory)
system. Qwen3-Embedding-0.6B is downloaded but not yet deployed. This document extracts concrete
patterns, evaluates tradeoffs for modest hardware (Ubuntu CPU-only, S. RTX 4060 Ti), and
synthesizes evolution paths for CCM.

---

## Context: Current CCM State

CCM (`cynic-kernel/src/domain/ccm.rs`) is a **frequency-based crystallization engine**:

- **Ingestion**: Claude Code hooks → `POST /observe` → `Observation` records in SurrealDB
- **Pattern extraction**: `extract_patterns()` — tool+target frequency / total observations → score
- **Co-occurrence extraction**: `extract_cooccurrences()` — pairs of files edited in the same session
- **Crystallization**: φ-bounded running mean; Forming → Crystallized (21+ cycles, confidence ≥ φ⁻¹) → Canonical (233+)
- **Feedback**: `format_crystal_context()` — injects mature crystals into Dog prompts, domain-filtered, token-budgeted
- **Deduplication**: FNV-1a content hash as crystal ID — stable, deterministic

The domain trait `EmbeddingPort` exists and has a `NullEmbedding` fallback. The embedding vector
infrastructure is ready (`Embedding::cosine_similarity()` implemented). The **gap**: no adapter
wires Qwen3-Embedding-0.6B, and CCM's pattern identification is purely lexical (string identity),
not semantic.

---

## Repo 1: VectifyAI/PageIndex — Vectorless Reasoning-Based RAG

**Stars**: 22K | **Language**: Python | **License**: MIT

### RAG Architecture

PageIndex is an **agentic, in-context tree index** — explicitly vectorless. It builds a hierarchical
table-of-contents structure from PDF documents through LLM-driven extraction:

1. Detect or generate TOC from the first N pages
2. Assign `physical_index` (page number) to each section
3. Recursively split sections exceeding `max_page_num_each_node` or `max_token_num_each_node`
4. Verify title-to-page assignments (sampling approach, LLM fuzzy-match check)
5. Auto-correct misassignments via `fix_incorrect_toc_with_retries()`
6. Add per-node summaries (optional) via `generate_summaries_for_structure()`

Retrieval: the LLM navigates the tree, reasoning about which branches are relevant rather than
embedding-querying a flat vector store. No chunking — sections are natural document units.

Achieved 98.7% accuracy on FinanceBench, outperforming vector RAG for structured professional documents.

### Observation/Ingestion

Designed for static documents (PDFs, Markdown), not streaming observations. Single-document or
batch processing with concurrent `asyncio.gather()` for page-level checks.

### Clustering/Crystallization

No clustering. The hierarchy IS the structure — the LLM reasons over it rather than discovering
clusters. Analogous to CCM's domain taxonomy, but imposed from document structure rather than
emergent from data.

### In-Process vs External

Pure Python library. No server required. The tree index is serialized as JSON. The "database" is
a JSON file with hierarchical nodes. Very low operational overhead.

### MCP Integration

Has an MCP server at `pageindex.ai/mcp` (cloud-hosted). Not designed for local embedding-free
use with MCP.

### CYNIC Relevance

**Low direct applicability.** PageIndex is optimized for long static documents. CCM ingests
ephemeral tool-use observations, not PDFs. However:

- **Transferable**: the tree-structure approach to organizing crystallized knowledge could inform
  CCM's evolution beyond flat lists of crystals. A hierarchical crystal taxonomy (domain →
  sub-domain → specific patterns) would enable more targeted context injection.
- **Insight**: "similarity ≠ relevance" is correct for CCM too. File co-occurrence frequency
  is a relevance signal that cosine similarity cannot directly capture. Both are needed.
- **Anti-pattern to avoid**: PageIndex's LLM-heavy tree construction (multiple LLM calls per
  document section) is too expensive for CCM's high-frequency observation pipeline.

---

## Repo 2: ForLoopCodes/contextplus — Semantic RAG for Codebases via MCP

**Stars**: 1.5K | **Language**: TypeScript | **License**: MIT

### RAG Architecture

Three-layer architecture over MCP (stdio transport):

1. **Structural layer** — Tree-sitter AST parsing (43 languages), gitignore-aware file traversal,
   produces a `SearchDocument` per file: `{path, header, symbols[], content}`
2. **Semantic layer** — Ollama embeddings (default: `nomic-embed-text`), disk-cached in
   `.mcp_data/embeddings-cache.json`. Hybrid search: `semanticWeight * cosineScore + keywordWeight * keywordScore`
3. **Graph layer** — In-memory property graph with JSON persistence (`.mcp_data/memory-graph.json`),
   decay-weighted edges, cosine auto-linking at threshold 0.72

The hybrid search formula is explicit:
```
combinedScore = semanticWeight * semanticScore + keywordWeight * keywordScore
```
Default weights are configurable. Minimum threshold filters applied before combining.

### Observation/Ingestion

Two ingestion paths:
- **Static**: full codebase scan on startup, batch embedded, cached with file-hash invalidation
- **Incremental**: `EmbeddingTracker` watches file changes (debounce 700ms), re-embeds changed
  files/symbols incrementally. Max 8 changed files per tracker tick.

Embedding batch size: 5–10 (clamped), configurable per GPU/CPU constraints.

### Clustering/Crystallization

`clustering.ts`: **spectral clustering with eigengap heuristic** for automatic cluster count `k`.

Algorithm:
1. Build cosine affinity matrix over all file embeddings
2. Compute normalized Laplacian
3. Eigenvalue decomposition (ml-matrix library)
4. Find optimal `k` via eigengap: `k = argmax(eigenvalue[k] - eigenvalue[k-1])`
5. k-means on eigenvectors (k-means++ seeding)

This is the same class of algorithm used in academic NLP for discovering topics without
pre-specifying count. The eigengap heuristic makes it parameter-free.

### Memory Graph Details

`memory-graph.ts` — the most directly relevant component for CCM:

```typescript
interface MemoryNode {
  id, type, label, content,
  embedding: number[],    // stored with node
  createdAt, lastAccessed, accessCount,
  metadata
}

interface MemoryEdge {
  source, target,
  relation: "relates_to" | "depends_on" | "implements" | "references" | "similar_to" | "contains",
  weight,          // decays: weight * exp(-0.05 * days)
  createdAt
}
```

**Traversal scoring**: `relevance = similarity * 0.6 + (decayedWeight / maxWeight) * 0.4`
**Depth penalty**: `score = decayedWeight * 1/(1 + depth * 0.3) * 100`
**Auto-link threshold**: cosine ≥ 0.72 creates `similar_to` edges automatically
**Pruning**: edges with decayed weight < 0.15 removed; orphan nodes (zero edges, low access, inactive 7d) removed
**Persistence**: scheduled save with 500ms debounce — writes JSON to disk

Search combines semantic (embed query, rank all nodes by cosine) with graph traversal (walk
neighbors of top hits, depth-limited, relation-filtered).

### In-Process vs External

Designed as **MCP server** (subprocess, stdio transport). Ollama must be running separately.
Embeddings are CPU/GPU-agnostic via Ollama. Cache lives on disk in `.mcp_data/`. Zero external
database — pure in-process property graph with JSON persistence.

This is the closest existing architecture to what CCM needs. The operational cost is minimal:
Ollama already serves Qwen3.5-9B, adding a lightweight embed model (nomic or Qwen3-Embed) is
a single `ollama pull` away.

### MCP Integration

This IS an MCP server. The 17 tools are exposed via MCP stdio. Critically relevant tools:

| Tool | Relevance to CCM |
|------|-----------------|
| `upsert_memory_node` | Store crystal as graph node with embedding |
| `create_relation` | Wire crystals with typed edges (co-occurs, implies, etc.) |
| `search_memory_graph` | Semantic search + graph walk — the CCM query |
| `prune_stale_links` | Decay-based garbage collection of weak associations |
| `add_interlinked_context` | Bulk-add observations with auto-similarity linking |
| `retrieve_with_traversal` | Walk from crystal outward — discover related context |

### CYNIC Relevance

**Highest direct relevance of all 9 repos.**

The memory graph architecture is a blueprint for CCM phase 2:
- Replace string-identity deduplication with embedding-based deduplication (cosine ≥ 0.72)
- Add typed edges between crystals (same session, same domain, semantic similarity)
- Graph traversal for richer context injection (not just top-N by confidence, but connected clusters)
- Spectral clustering to auto-discover crystal sub-domains

The key architectural insight: **observations become nodes, co-occurrences become edges**. The
current CCM already computes co-occurrence scores — it just stores them as flat crystal strings
rather than as graph edges. The step to a graph is smaller than it appears.

**Hardware fit**: TypeScript/Node.js, Ollama backend — runs on Ubuntu CPU. Qwen3-Embed-0.6B at
596MB fits comfortably in RAM alongside the existing llama-server.

---

## Repo 3: ItzCrazyKns/Vane — Self-Hosted AI Search Engine (Perplexica Successor)

**Stars**: 33K | **Language**: TypeScript/Next.js | **License**: MIT

### RAG Architecture

Web search RAG engine. Pipeline:
1. Classify question → decide if research is needed
2. If research: submit to SearxNG meta-search backend (privacy-preserving)
3. Optionally search uploaded files via embeddings (semantic search over user documents)
4. Run widgets in parallel (weather, stocks, etc. — structured lookups)
5. Synthesize answer with citations from web results

Three optimization modes (speed/balanced/quality) — the implementation details are internal,
not exposed in architecture docs. Presumed to differ in: number of sources fetched, reranking
depth, and model size used for synthesis.

File uploads get embedded and searched semantically, but the embedding model/storage backend
is not documented publicly. Likely Ollama + local vector store.

### Observation/Ingestion

Web search is the primary ingestion channel. File uploads are secondary. No streaming session
observation. Not an agent observation pipeline.

### Clustering/Crystallization

No clustering — single-turn Q&A with cited sources. Each search is independent; no persistent
knowledge accumulation across sessions.

### In-Process vs External

**Docker-first deployment.** Bundles SearxNG inside the same container. Next.js backend +
frontend in one image. SearxNG is a separate process. Self-hostable on a single machine.

### MCP Integration

None identified. Provides its own web UI and REST API (`POST /api/chat`, `POST /api/search`).

### CYNIC Relevance

**Low direct applicability.** Vane is a user-facing search UI, not an agent observation system.

However, the architecture demonstrates a proven pattern for the **CYNIC temporal/web-search
use case**: classify query → route to appropriate search backend → synthesize with citations.
If CYNIC ever adds web-search grounding for its judgments (e.g., verifying current events),
Vane's SearxNG integration approach is a working reference.

**One transferable insight**: the three-mode optimization (speed/balanced/quality) maps directly
to CYNIC's inference routing — deterministic-dog for speed, sovereign for quality. Vane
demonstrates this is user-comprehensible and useful as an explicit parameter.

---

## Repo 4: firecrawl/firecrawl — Web Data → LLM-Ready Markdown

**Stars**: 96K | **Language**: TypeScript/Node.js | **License**: AGPL-3.0

### RAG Architecture

Firecrawl is a **data acquisition layer**, not a RAG system. It converts web pages to LLM-ready
formats:
- Markdown (clean, stripped of navigation/ads)
- Structured JSON (via LLM extraction with schema)
- Screenshots (vision-capable models)
- HTML (raw)

Handles: JavaScript rendering, auth walls, proxies, dynamic content, rate-limiting. Batch async
processing for thousands of URLs. Change tracking (monitor content deltas over time).

The MCP server (`firecrawl-mcp-server`, separate repo) exposes scrape/crawl/search as MCP tools.
The CLI (`firecrawl-cli`) integrates with Claude Code as a skill.

### Observation/Ingestion

Excellent ingestion infrastructure: scrape individual pages, crawl entire sites, map all URLs,
search the web and return page content. Change-tracking API monitors URLs for content deltas.

### Clustering/Crystallization

No clustering built-in — Firecrawl delivers raw content. Downstream systems cluster/crystallize.

### In-Process vs External

Cloud API primary (api.firecrawl.dev). Self-hosting possible but "not fully ready" per README.
The business model is the hosted API — self-hosting is discouraged in practice.

### MCP Integration

First-class. The `firecrawl-mcp-server` exposes 6 tools: scrape, search, map, crawl, extract,
deep_research. The `firecrawl-cli` installs as a Claude Code skill via `/plugin`.

### CYNIC Relevance

**Low for CCM specifically, but high for CYNIC's web-grounding capability.**

CCM observes agent sessions, not web pages. Firecrawl is the wrong layer for CCM. However:

- For CYNIC's **empirical verification** axiom (VERIFY): Firecrawl + MCP would let Dogs verify
  factual claims against current web content. This is infrastructure-level grounding, not CCM.
- For the **autoimmune research** use case (Jay's project): ingesting medical papers from PubMed/
  arXiv into CYNIC's knowledge base — Firecrawl handles the acquisition, CCM handles the
  crystallization. That pipeline would be: Firecrawl → `POST /observe` → CCM aggregate → crystals.
- **Change tracking** maps to CYNIC's temporal awareness — detecting when previously reliable
  sources change their position.

**Blocker**: Firecrawl's self-hosting is explicitly noted as "not fully ready." Cloud API costs
money. Not appropriate for CYNIC's sovereignty principle. Watch for self-hosting maturity.

---

## Repo 5: langflow-ai/openrag — RAG Platform (Langflow + Docling + OpenSearch)

**Stars**: 3.5K | **Language**: Python/TypeScript | **License**: Apache-2.0

### RAG Architecture

Full-stack RAG platform. Ingestion pipeline (from `ingestion_flow.json`):

1. **DoclingRemote** — structured document extraction (PDFs, DOCX, images, tables) from a
   dedicated Docling Serve instance. Preserves hierarchy and layout.
2. **ExportDoclingDocument** — normalizes Docling output to standard format
3. **DataFrameOperations** (×3) — column transformations, metadata enrichment
4. **SplitText** — chunking (1000 chars, 200 char overlap)
5. **AdvancedDynamicFormBuilder** — collects metadata (owner, connector, access control)
6. **OpenSearchVectorStoreComponentMultimodalMultiEmbedding** — stores with **three separate
   embedding models simultaneously**

Retrieval (from `openrag_agent.json`):
- Langflow Agent orchestrates tool selection
- OpenSearch vector store with multi-embedding support
- Agent decides when to invoke retrieval vs other tools
- Text filter input for constrained retrieval
- MCP tools connectable to agent

Re-ranking details are not exposed in the flow config — likely handled inside OpenSearch
or the Langflow retrieval component.

### Observation/Ingestion

Document-centric. Rich metadata preservation (access control groups, owner, source URL). The
Docling integration handles messy real-world documents including complex PDFs with tables.

### Clustering/Crystallization

No explicit clustering. OpenSearch handles semantic indexing. The "nudges" flow
(`openrag_nudges.json`) presumably surfaces emerging topics or suggested documents — not yet
analyzed in detail.

### In-Process vs External

**Heavy external dependencies**: OpenSearch cluster + Docling Serve + Langflow backend + Next.js
frontend. Docker Compose with separate containers. Enterprise-grade but expensive to operate on
modest hardware.

OpenSearch alone typically requires 2–4GB RAM minimum. Not viable on CYNIC's Ubuntu CPU-only
machine for CCM's real-time observation pipeline.

### MCP Integration

`openrag_url_mcp.json` exists in flows — an MCP-connected retrieval flow. The `openrag-mcp`
package provides an MCP server wrapping the OpenRAG API for Claude Desktop/Cursor integration.

Exposes: RAG-enhanced chat, semantic search, settings management.

### CYNIC Relevance

**Low for hardware reasons; architecturally instructive.**

The three-embedding-model approach is interesting: running three separate embedding models over
the same content at ingestion time gives a richer semantic representation. At query time, results
from all three can be merged (reciprocal rank fusion or score combination). This is a form of
ensemble embedding.

For CYNIC: a lighter variant would be embedding crystal content at crystallization time using
Qwen3-Embed-0.6B, enabling semantic deduplication before a crystal is written. The current
FNV-1a hash only deduplicates exact-string matches — two semantically identical observations
with different wording currently create two separate crystals.

**Transferable technique**: multi-embedding ensemble for robustness (embeds with two models,
takes max similarity). Feasible with Qwen3-Embed-0.6B (596MB) + a second lightweight model.

---

## Repo 6: alibaba/zvec — Lightweight In-Process Vector DB (C++)

**Stars**: 9K | **Language**: C++ (Python + Node.js bindings) | **License**: Apache-2.0

### RAG Architecture

In-process vector database built on **Proxima** (Alibaba's production HNSW engine). Key
properties:

- **Dense + Sparse vectors**: supports both in a single `CollectionSchema`, multi-vector queries
- **Hybrid search**: semantic vector similarity combined with structured attribute filters
- **In-process**: runs as a library embedded in your process — no server, no network overhead
- **Persistence**: collections saved to disk, opened by path
- **Platform support**: Linux x86_64 + ARM64, macOS ARM64 (no Windows)

API pattern:
```python
schema = zvec.CollectionSchema(
    name="crystals",
    vectors=zvec.VectorSchema("embedding", zvec.DataType.VECTOR_FP32, 512),
    # sparse vectors, filters added via additional schema fields
)
collection = zvec.create_and_open(path="./zvec_crystals", schema=schema)
collection.insert([zvec.Doc(id="cry_abc", vectors={"embedding": vec})])
results = collection.query(zvec.VectorQuery("embedding", vector=query_vec), topk=10)
```

Performance claim: "billions of vectors in milliseconds" — benchmark graph in docs for 10M
vectors. Backed by Alibaba's production use in similar-item retrieval at massive scale.

### Observation/Ingestion

No ingestion pipeline — pure storage engine. You bring your own embedding generation.

### Clustering/Crystallization

No clustering built-in. The library does ANN (approximate nearest neighbor) search, not
clustering. External clustering would need to pull vectors and apply e.g. k-means.

### In-Process vs External

**The key value proposition**: zero operational overhead. Binds directly into Python or Node.js
process. Data lives on disk at a path. Comparable to SQLite for relational data — same philosophy
applied to vectors.

Current CYNIC embedding layer is a Rust trait (`EmbeddingPort`). A Rust FFI binding to Zvec
does not exist (Python + Node.js only). This is a limitation for the Rust kernel.

### MCP Integration

None. It's a library, not a service.

### CYNIC Relevance

**Architecturally appealing, practically blocked.**

Zvec would be ideal for embedding crystals in-process: same performance profile as SurrealDB
(already in-process), no extra server. However:

1. **No Rust bindings** — would require writing C FFI bindings or calling via a subprocess.
   SurrealDB's built-in vector support is more pragmatic for CYNIC's Rust codebase.
2. **Hybrid search** — the dense+sparse multi-vector approach is worth tracking. When CYNIC
   adds sparse embeddings (BM25-style term weights), a combined dense+sparse index would improve
   retrieval precision. Zvec is purpose-built for this. Consider when SurrealDB's vector
   support proves insufficient.
3. **Alternative**: SurrealDB 2.x added vector indexing. Check `storage.rs` adapter for whether
   HNSW index creation is already wired — if not, zvec's approach validates that SurrealDB's
   built-in is the right path for the kernel.

**Verdict**: file for later. The architecture is correct; the language bindings are wrong for
the Rust kernel today.

---

## Repo 7: thedotmack/claude-mem — Session Capture + Context Injection for Claude Code

**Stars**: 39K | **Language**: TypeScript/Node.js | **License**: AGPL-3.0

### RAG Architecture

Hybrid semantic + keyword search over Claude Code session observations:

- **SQLite + FTS5** — full-text search over observations, sessions, summaries
- **Chroma vector database** — semantic search via embeddings (model unspecified in README)
- **Hybrid retrieval**: combine FTS5 keyword hits with Chroma semantic hits → merged results

The 3-layer retrieval workflow:
1. `search` — compact index (~50–100 tokens/result) — returns IDs
2. `timeline` — chronological context around specific observations
3. `get_observations` — full details for filtered IDs only (~500–1000 tokens/result)

Token savings via progressive disclosure: ~10x reduction vs fetching everything.

### Observation/Ingestion

7 Claude Code lifecycle hooks capture observations automatically:
- `SessionStart` — new session context injection
- `UserPromptSubmit` — capture each user message
- `PostToolUse` — capture every tool call result (this is the core signal)
- `Stop` — session-end summarization
- `SessionEnd` — finalize and persist

The `PostToolUse` hook is the direct equivalent of CYNIC's `/observe` endpoint. Every tool use
(Edit, Bash, Glob, Read, etc.) gets captured with context and stored in SQLite.

A **worker service** (HTTP API on port 37777) runs as a background process managed by Bun,
handling the heavy processing (summarization, embedding) asynchronously so hooks return fast.

Web viewer UI at `localhost:37777` provides real-time memory stream — useful for debugging.

### Clustering/Crystallization

Not explicit in the README. The system generates **semantic summaries** at `Stop`/`SessionEnd`
using an LLM (configured AI model), then stores them alongside raw observations. The summary
IS the crystallization step — a lossy compression of session observations into a higher-level
narrative.

No frequency-based confidence scoring (unlike CCM's φ-bounded system). claude-mem treats all
observations as equally valid — no quality signal. This is a meaningful architectural difference:
CCM's φ-bounded crystallization filters noise; claude-mem's approach preserves everything but
can be queried selectively.

### In-Process vs External

**Mixed**:
- SQLite: in-process, embedded
- Chroma: external Python process (auto-managed by `uv`)
- Worker: Bun subprocess (auto-started by hooks)

The multi-process architecture is more complex than CYNIC's approach but tolerates failures better
(worker crash doesn't break Claude Code session).

### MCP Integration

Provides 4 MCP tools: `search`, `timeline`, `get_observations`, plus an internal tool. The
progressive disclosure pattern (search-first, detail-on-demand) is a **critical token-efficiency
technique** for any CCM retrieval interface.

The `<private>` tag pattern for excluding sensitive content from storage is also useful — CYNIC's
observation pipeline currently has no selective exclusion mechanism.

### CYNIC Relevance

**High — this is the closest existing system to CCM's actual use case (agent session observation).**

Direct comparison:

| Dimension | claude-mem | CYNIC CCM |
|-----------|-----------|-----------|
| Ingestion | PostToolUse hook → SQLite | `/observe` REST endpoint → SurrealDB |
| Dedup | Session-scoped, no cross-session dedup | FNV-1a content hash (lexical) |
| Crystallization | LLM summary at session end | φ-bounded frequency mean (21 cycles) |
| Search | FTS5 + Chroma hybrid | Currently: exact crystal lookup only |
| Retrieval | Progressive disclosure 3-layer | format_crystal_context() flat list |
| MCP | 4 search tools | cynic_mcp (health, judge, infer, verdicts, crystals, coord) |
| Quality filter | None (everything stored) | φ⁻¹ threshold gates crystallization |

**Transferable patterns**:
1. **Progressive disclosure** for crystal retrieval — CCM currently injects all mature crystals
   above the confidence threshold. A 3-layer pattern (IDs first → timeline → detail) would
   dramatically reduce token cost in Dog prompts.
2. **PostToolUse → async worker** architecture — decoupling hook latency from processing latency.
   CYNIC's hooks currently wait for the REST call. Moving to a local queue + async processor
   would make hooks faster.
3. **`<private>` tag** exclusion — a `CYNIC_OBSERVE=false` env var or per-file exclusion pattern.

---

## Repo 8: airweave-ai/error-monitoring-agent — Error Clustering + Context Enrichment

**Stars**: 303 | **Language**: Python/FastAPI + React | **License**: MIT

### RAG Architecture

The context retrieval is backed by **Airweave** — a managed semantic search service that
federates search across GitHub, Linear, Slack, Notion/Confluence. It acts as a unified
cross-source search API:

```python
results = client.search.search(
    collection_id=COLLECTION_ID,
    query="timeout in sync_worker batch processing",
    source_name="github",  # optional source filter
    limit=5
)
```

Airweave handles embedding, indexing, and retrieval internally. The agent uses it as a
black-box context store.

### Observation/Ingestion

Error ingestion from Sentry, Azure Log Analytics, or custom sources via a `DataSource` base
class. The pipeline is scheduled (cron) or triggered via API:

```
Raw errors → clustering → context enrichment → severity analysis → alerts
```

State persisted in JSON files (`error_signatures.json`, `mutes.json`) — deliberately
database-free for portability.

### Clustering/Crystallization

**Three-stage cascade clustering** — the most instructive clustering approach in this survey:

**Stage 1 — Regex/type clustering** (O(n), zero LLM cost):
- Group by HTTP code, exception class, error message pattern
- Example: all 429s together, all `ConnectionRefusedError` together

**Stage 2 — LLM semantic clustering** (O(k) LLM calls where k << n):
- Within each type-cluster, use LLM to detect functionally equivalent errors
- "Rate limit exceeded" + "HTTP 429" + "Too many requests" → same cluster
- Applied AFTER stage 1, so LLM only sees already-typed groups

**Stage 3 — Cluster merging** (O(k²) comparisons, k small):
- LLM identifies if two type-clusters describe the same root cause
- Merges if identical

This 60–80% LLM call reduction via pre-filtering is directly applicable to CCM's crystallization
pipeline.

### Suppression/Quality Logic

**Smart suppression** prevents alert spam:

| Status | Trigger | Action |
|--------|---------|--------|
| NEW | First occurrence | Always alert + create ticket |
| REGRESSION | Was resolved, reappeared | Always alert + reopen ticket |
| ONGOING | Known open issue | Suppress (comment only) |

Override rule: S1/S2 severity bypasses suppression regardless of status.

This maps to CCM's φ-bounded states: **NEW ≈ Forming**, **ONGOING ≈ Crystallized**, **REGRESSION
≈ Decaying (recovered)**. The suppression logic is more explicit about action routing.

### In-Process vs External

FastAPI backend (lightweight Python), React frontend (demo only), JSON state files.
The heavy dependency is Airweave (managed cloud service). The clustering logic itself is
local Python — no external dependency.

### MCP Integration

None. Uses a custom Airweave SDK.

### CYNIC Relevance

**High for the clustering cascade pattern.**

The three-stage cascade (cheap filter → medium filter → expensive LLM) is the right architecture
for CCM's next phase:

1. **Stage 1** (current CCM): exact-string frequency counting — O(1) per observation, zero cost
2. **Stage 2** (next CCM): embedding similarity deduplication — O(n × embed_cost), but embed is
   ~1ms on Qwen3-Embed-0.6B
3. **Stage 3** (future CCM): LLM-level cluster merging — "Edit storage.rs" + "Modify storage
   adapter" are semantically the same crystal. Run periodically, not on every observation.

The suppression states also suggest a **crystal action routing** layer: when a pattern reaches
Crystallized state with high confidence, it should actively influence routing (not just inject
context). When it Decays, routing reverts. This is a behavioral consequence of the φ-state
machine that isn't yet wired to any downstream action.

---

## Repo 9: alibaba/OpenSandbox — Agent Sandbox Platform

**Stars**: 9K | **Language**: Python + multi-SDK | **License**: Apache-2.0

### RAG Architecture

OpenSandbox is **not a RAG system**. It is an agent execution environment providing:
- Container-based isolation (Docker, Kubernetes, gVisor, Kata, Firecracker)
- Sandbox lifecycle API (create, execute, terminate)
- Code interpreter, filesystem, command execution environments
- Network ingress/egress controls per sandbox

No knowledge management, no embedding, no retrieval.

### Observation/Ingestion

Execution artifacts are observable:
- `logs.stdout`, `logs.stderr` from command execution
- File system operations (read/write logs)
- Code interpreter results with execution metadata

But these are raw execution logs, not semantically indexed.

### Clustering/Crystallization

None.

### In-Process vs External

Container-native. Docker required locally. Kubernetes for scaled deployment. Not trivially
runnable in-process.

### MCP Integration

None natively. Integration examples show Claude Code, Gemini CLI, etc. running **inside** sandboxes.

### CYNIC Relevance

**Indirect, but strategically important for the embed-agent-vision.**

From `project_embed_agent_vision.md`, CYNIC plans an **autonomous code audit agent** that runs
agents in isolation. OpenSandbox is the production-grade infrastructure for that use case:

- Isolate audit agents per session (per-sandbox egress controls prevent data exfiltration)
- Capture execution logs for CCM observation (audit agent's tool calls become observations)
- gVisor/Kata for untrusted code execution (if CYNIC runs arbitrary agent-generated code)

The CNCF Landscape listing indicates this is production infrastructure, not a research prototype.

**Concrete path**: when CYNIC's audit agent (not yet built) executes code analysis tasks, each
audit session runs in an OpenSandbox container. The container's stdout/stderr feeds back to
`POST /observe` for CCM. The sandbox boundary ensures agent isolation while CCM learns from
aggregate behavior across audit sessions.

---

## Synthesis: CCM Evolution Paths

### Current CCM Capabilities (Baseline)

```
Observations → frequency counting → co-occurrence pairs → φ-bounded crystals → prompt injection
```

Strengths:
- Zero external dependencies (runs inside Rust kernel)
- Mathematically principled (φ-bounds prevent noise crystallization)
- Deterministic (FNV-1a hashing, reproducible)
- Fast (pure in-memory computation, SurrealDB for persistence)

Gaps:
- Semantic deduplication absent — "Edit storage.rs" and "Modify the storage adapter" are different
  crystals despite identical meaning
- Flat crystal list — no relationships between crystals
- Prompt injection is a flat sorted list — no graph traversal, no progressive disclosure
- Crystal retrieval is domain-filtered but not query-aware — same crystals injected regardless
  of the specific task at hand
- No temporal decay of crystal content (only φ-state decay, not embedding drift)

### Evolution Path A: Semantic Deduplication (Smallest Step)

**Target**: wire Qwen3-Embedding-0.6B to the existing `EmbeddingPort` trait.

Changes required:
1. Implement `Qwen3EmbedAdapter` in `cynic-kernel/src/adapters/` — POST to llama-server
   `/v1/embeddings` endpoint (already wired for llama-server protocol)
2. In `aggregate_observations()`, before `observe_crystal()`, embed the new content and compare
   cosine similarity against existing crystal embeddings. If cosine ≥ 0.85 with an existing
   crystal, merge into that crystal rather than creating a new one.
3. Store embedding vectors alongside crystals in SurrealDB (add `embedding` field to Crystal
   struct, or a separate `crystal_embeddings` table)

Effort: 1–2 days. Fits within the existing hexagonal architecture — `EmbeddingPort` is already
the right abstraction.

Validates: whether Qwen3-Embed-0.6B at 596MB on Ubuntu CPU is fast enough for real-time
deduplication during aggregation cycles. Expected: ~5–10ms per embedding, acceptable for a
background aggregation task (not on the critical path of `/observe`).

### Evolution Path B: Semantic Crystal Retrieval (Medium Step)

**Target**: query-aware crystal injection instead of top-N by confidence.

When a Dog is about to judge a request (e.g., "Is this Sicilian Defense move good?"), embed the
query and retrieve the most semantically relevant crystals rather than the globally most confident
ones. A chess-domain crystal about "pawn structure in closed positions" is more relevant than a
high-confidence crystal about "SurrealDB connection pooling."

Changes required:
1. `StoragePort::search_crystals(query_embedding: &[f32], domain: &str, topk: u32)` — new
   method using SurrealDB's HNSW index
2. Pass query to `format_crystal_context()` — currently takes only domain + max_chars
3. In `InferencePort` adapter, embed the judge request before injecting crystals

This is "RAG over crystals" — the crystals are the knowledge base, the Dog's request is the
query. Semantically relevant memory, not just high-confidence memory.

Effort: 2–3 days. Requires SurrealDB HNSW index creation for the crystals table.

### Evolution Path C: Crystal Graph (Larger Step, Borrow from contextplus)

**Target**: typed edges between crystals, graph traversal for context injection.

The current crystal store is a flat table. Co-occurrence patterns are stored as separate
crystals (content: "a.rs + b.rs — co-edited in 67% of sessions") but not as edges between
the a.rs and b.rs crystals.

Proposed graph layer (in-memory, persisted to SurrealDB graph tables):

```rust
struct CrystalEdge {
    id: String,
    source_crystal_id: String,
    target_crystal_id: String,
    relation: CrystalRelation,
    weight: f64,           // decays: weight * exp(-0.05 * days)
    created_at: String,
}

enum CrystalRelation {
    CoOccurs,      // edited in the same session
    SemanticallySimilar,  // cosine ≥ 0.72
    Implies,       // crystal A's presence predicts crystal B
    Contradicts,   // crystals with opposing signals
}
```

Traversal for context injection: start from the most query-relevant crystal, walk 1–2 hops,
collect neighborhood. This surfaces related context that direct retrieval would miss.

Effort: 3–5 days. Significant schema migration in SurrealDB (SurrealDB natively supports graph
relations via `->` syntax — this fits naturally).

The eigengap spectral clustering from contextplus could then run periodically over the crystal
embedding space to auto-discover crystal sub-domains (instead of the current manually-assigned
`domain` field).

### Evolution Path D: Progressive Disclosure Retrieval (UX Step, Borrow from claude-mem)

**Target**: 3-layer MCP retrieval interface for CCM crystals.

Current CCM MCP tools expose `crystals` (list) and `crystal/{id}` (single). No search.

Proposed 3-layer pattern:
1. `ccm_search(query, domain, limit)` → compact index: `[{id, confidence, state, snippet}]`
2. `ccm_timeline(crystal_id, window)` → chronological observations around that crystal
3. `ccm_detail(ids[])` → full crystal content + embedding neighbors

This matches claude-mem's approach and provides ~10x token savings for agents querying CCM.
A Dog asking "what do I know about chess endgames?" gets 20 crystal summaries (50 tokens each
= 1000 tokens total) rather than 20 full crystal records (200 tokens each = 4000 tokens).

Effort: 1 day. Pure API layer change, no storage migration needed.

### Evolution Path E: Cascade Clustering (Borrow from error-monitoring-agent)

**Target**: staged observation deduplication to reduce LLM costs at scale.

Stage 1 (current): exact-string identity via FNV-1a hash — O(1), zero cost
Stage 2 (add): embedding cosine similarity ≥ 0.85 — merge into existing crystal at write time
Stage 3 (add): periodic LLM-level semantic merge — run nightly, compare crystals where
  `0.70 ≤ cosine < 0.85` (similar but not duplicate), let a small model (Gemma 3 4B) decide
  if they should merge

Stage 3 is the lightweight equivalent of error-monitoring-agent's LLM clustering stage.
Running it nightly means:
- No latency impact on the observation pipeline
- LLM cost bounded by the number of near-duplicate crystal pairs (should be small for a mature
  system)
- Uses sovereign-ubuntu (Gemma 3 4B) — no cloud API needed

Effort: stage 2 = 1 day (pairs with Path A), stage 3 = 2 days additional.

---

## Recommended Implementation Order

Based on effort, hardware fit, and dependency on Qwen3-Embed-0.6B already being downloaded:

| Priority | Path | Description | Effort | Blocker |
|----------|------|-------------|--------|---------|
| P1 | A | Qwen3EmbedAdapter wired to EmbeddingPort | 1–2 days | None — model is on disk |
| P2 | A | Semantic dedup in aggregate_observations() | 1 day | P1 |
| P3 | D | Progressive disclosure MCP tools for crystals | 1 day | None |
| P4 | B | Query-aware crystal retrieval in format_crystal_context | 2–3 days | P1 + SurrealDB HNSW |
| P5 | E | Cascade clustering stage 2 (cosine merge at write) | 1 day | P1 |
| P6 | C | Crystal graph with typed edges | 3–5 days | P1 + P2 |
| P7 | E | Cascade stage 3 (nightly LLM merge) | 2 days | P5 + P6 |

P1 is the single prerequisite for almost everything else. The embed model is already on disk —
the gap is just writing the adapter. Everything after P1 compounds on itself.

The contextplus memory graph (Path C) is the most architecturally ambitious but also the most
powerful. Contextplus's open-source implementation serves as a TypeScript reference that can
be translated to Rust idioms. The key algorithms (spectral clustering via eigengap, decay-weighted
traversal) are well-documented in its source.

---

## Hardware Constraint Assessment

All paths are compatible with CYNIC's hardware (Ubuntu CPU-only for the kernel):

- **Qwen3-Embed-0.6B** (596MB): fits in RAM alongside llama-server running Gemma 3 4B.
  Expected embed latency on Vega 8 APU or CPU: 5–20ms per text. Acceptable for background
  aggregation (not on the `/observe` hot path).
- **SurrealDB HNSW**: built-in, no additional process. CYNIC already uses SurrealDB.
- **Spectral clustering** (Path C): pure CPU math (affinity matrix + eigendecomposition over
  crystal embeddings). With N < 10K crystals, this is milliseconds on any modern CPU.
- **Progressive disclosure** (Path D): pure Rust API layer. Zero compute overhead.

The only path with meaningful resource concern is the nightly LLM merge (Path E stage 3),
which requires a Gemma 3 4B inference call per near-duplicate pair. At 10–50 such pairs after
system maturity, this is < 1 minute nightly — fully acceptable.

---

## Open Questions for T.

1. **Qwen3-Embed-0.6B vs nomic-embed-text**: contextplus defaults to nomic-embed-text via
   Ollama. Qwen3-Embed-0.6B is designed for multilingual and code — likely better for CYNIC's
   mixed Rust/chess/natural-language crystals. Confirm which model is loaded/accessible on
   Ubuntu before writing the adapter.

2. **SurrealDB HNSW availability**: SurrealDB 2.x added vector search. The current storage
   adapter (`cynic-kernel/src/adapters/`) needs auditing for whether HNSW index creation is
   already wired or needs to be added.

3. **Crystal content granularity**: current crystals are coarse ("Edit storage.rs — 10x
   observed"). With semantic deduplication, should content be richer (include the actual edit
   context)? Richer content → better embedding → better dedup, but higher storage cost.

4. **Decay for crystals**: contextplus uses `exp(-0.05 * days)` for edge decay. CCM currently
   decays crystal state (Crystallized → Decaying) but not the confidence value over time.
   Should crystals that haven't been observed for 30+ days lose confidence? This would make
   CCM a truly living memory rather than an append-only accumulator.

