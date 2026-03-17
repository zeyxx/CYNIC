<!-- Auto-invocation via ~/.claude/commands/cynic-skills/ai-infrastructure/ — no frontmatter here to avoid duplication -->

# AI Infrastructure Engineering

*Distilled from: awesome-system-design-resources, awesome-generative-ai-guide, machine-learning-systems-design, Made-With-ML, nn-zero-to-hero, and production ML engineering practice.*

**Purpose:** Encode ML/AI infrastructure patterns so the LLM applies them automatically — no re-fetching repos, no missed patterns, no debt.

---

## 1 — LLM Inference Serving

### Patterns (MUST apply when building inference pipelines)

| Pattern | Problem it solves | Implementation |
|---|---|---|
| **Cold-start polling** | Server not ready at boot | Exponential backoff (1s→16s), max timeout, health endpoint check |
| **Connection pooling** | HTTP overhead per request | Reuse `reqwest::Client` or equivalent, connection keep-alive |
| **Request batching** | Underutilized GPU/CPU | Accumulate requests over window (5-50ms), send as batch |
| **KV cache management** | Redundant computation on context | Server-side (llama.cpp manages this), client must respect context limits |
| **Streaming responses** | Latency perception, time-to-first-token | SSE/streaming endpoint, yield tokens as generated |
| **Model hot-swap** | Zero-downtime model change | Drain active requests → load new model → resume |
| **Quantization awareness** | Memory vs quality tradeoff | Client knows model quantization (Q4_K_M, Q5_K_S, etc.), adjusts expectations |
| **Context window management** | Prompt exceeds model limit | Truncation strategy (keep system + recent), or sliding window |
| **Concurrent request limiting** | OOM / quality degradation | Semaphore or bounded channel, reject/queue when full |
| **Graceful timeout** | Hung inference | Per-request timeout, cancel token, return partial if possible |

### Health Check Protocol

```
GET /health → { status: "ok" | "no slot available" | "loading model" | "error" }
```

Three health states map to circuit breaker:
- `ok` → Closed (healthy, route requests)
- `no slot available` → Degraded (backpressure, queue or reject)
- `loading model` / `error` → Open (skip, probe periodically)

### OpenAI-Compatible API (de facto standard)

```
POST /v1/chat/completions   → inference
GET  /v1/models             → model discovery
GET  /health                → health (llama.cpp specific)
POST /v1/embeddings         → vector generation
```

**CRITICAL:** Target OpenAI-compatible API as the abstraction boundary. llama.cpp, Ollama, vLLM, TGI all expose this. Your InferencePort trait maps to this, not to vendor-specific APIs.

### Memory Management (for local inference)

| Resource | Monitor | Action |
|---|---|---|
| VRAM/shared RAM | Before loading model | Check available vs model size (GGUF metadata has this) |
| System RAM | During inference | Mmap allows OS to manage, but watch for swap thrashing |
| Swap | Continuous | If swap > 2GB during inference, model too large for hardware |
| Context memory | Per request | n_ctx * n_batch * model_dims * sizeof(float16) |

---

## 2 — Model Serving Architecture

### Single-Node Serving

```
Client → InferencePort → BackendRouter → LlamaCppBackend → llama-server HTTP → Response
                                       → OllamaBackend   → ollama HTTP      → Response
                                       → VllmBackend     → vLLM HTTP        → Response
```

**Rule:** Every backend implements the same port trait. Client never knows which backend serves the request.

### Multi-Node Serving (distributed)

```
Node A (orchestrator):
  Client → InferencePort → BackendRouter
                            ├→ LocalBackend (localhost:11435)
                            ├→ RemoteBackend (vpn-ip:11435)
                            └→ RemoteBackend (another-node:11435)
```

**Rules:**
- Local node CAN do its own inference (no proxy assumption)
- Remote nodes are OPTIONAL — discovered via probe or config
- Network topology (VPN, LAN, localhost) is transparent to domain
- Circuit breaker per backend handles node unreachability

### Distributed Inference Options (2026)

| Approach | How | When to use |
|----------|-----|------------|
| **llama.cpp RPC** | Offloads model layers across nodes via TCP. Memory allocated proportionally per device. | Tensor parallelism — one large model split across multiple machines |
| **OpenAI-compatible HTTP routing** | Each node runs its own server. Router dispatches requests. | Heterogeneous backends (llama.cpp + Ollama + vLLM). Most flexible. |
| **llama.cpp router mode** | Single server, `--model-dir`, routes by model parameter | Multi-model on single node. Simple. |
| **NVIDIA Dynamo** | Dynamic GPU worker allocation, distributed scheduling | Large GPU clusters with NVIDIA hardware |
| **llm-d** | Kubernetes-native, hierarchical KV offloading, cache-aware routing | Production K8s deployments |

**Default choice:** OpenAI-compatible HTTP routing — works with any backend, supports circuit breaker per-node, no coupling between nodes.

### Backend Selection Strategy

| Strategy | When to use |
|---|---|
| **Round-robin** | Homogeneous backends, even load |
| **Capability-match** | Heterogeneous hardware (GPU vs CPU vs APU) |
| **Latency-based** | Mixed local/remote with different latencies |
| **Model-affinity** | Specific model loaded on specific backend |
| **Fan-out** | Need diverse responses (temperature variation) |

---

## 3 — Training & Learning Pipelines

### Online Learning Patterns

| Pattern | How it works | When to use |
|---|---|---|
| **Q-Learning** | State-action-reward table, epsilon-greedy exploration | Discrete action spaces, small state spaces |
| **Thompson Sampling** | Beta distribution per action, sample to explore | Multi-armed bandit (e.g., backend selection) |
| **EWC (Elastic Weight Consolidation)** | Fisher information matrix prevents catastrophic forgetting | Continuous learning without forgetting past tasks |
| **DPO (Direct Preference Optimization)** | Human preference pairs → reward model | Aligning model outputs to preferences |
| **Self-Evolving Agents** | Capture failures → generate corrective skills → redeploy | Agent systems that improve from mistakes (see EvoAgentX, MemSkill) |

### Experiment Tracking

- Every inference = one experiment
- Log: input hash, model used, parameters, output, latency, outcome (if feedback received)
- Use structured logging (not ad-hoc prints)
- Enable replay: given the same input + model + params, can you reproduce the output?

---

## 4 — Vector Storage & Embeddings

### When to use vectors

- **Knowledge crystallization:** Embed accumulated wisdom for semantic retrieval
- **RAG:** Retrieve relevant context before inference
- **Agent memory:** Semantic search over past interactions

### Embedding Strategy

| Decision | Options | Recommended default |
|---|---|---|
| Embedding model | OpenAI, sentence-transformers, local model | Local model preferred for sovereignty |
| Vector DB | Qdrant, Milvus, pgvector, SurrealDB, ChromaDB | Depends on existing stack |
| Dimension | 384 (MiniLM) / 768 (BERT) / 1536 (OpenAI) | Depends on model capability |
| Distance metric | Cosine / Euclidean / Dot product | Cosine (normalized, most common) |

### Chunking (for document ingestion)

- Fixed-size (512 tokens) with overlap (50 tokens)
- Semantic (split on paragraph/section boundaries)
- Hierarchical (summary of summaries)

---

## 5 — Agent Architectures

### Agent Loop Pattern

```
while not done:
    observation = perceive(environment)
    thought = reason(observation, memory)
    action = decide(thought, available_tools)
    result = execute(action)
    memory.update(observation, action, result)
    done = evaluate(result, goal)
```

**Generalized cycle:** PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE

### Multi-Agent Consensus

| Pattern | How it works | Use case |
|---|---|---|
| **Independent evaluation** | Each agent scores independently, aggregate results | Quality assessment, content moderation |
| **Weighted aggregation** | Scores weighted by agent reliability/track record | Backend selection, model routing |
| **BFT consensus** | Tolerate f Byzantine agents from 3f+1 total | Critical decisions requiring fault tolerance |
| **Debate** | Agents argue opposing positions, third agent judges | Controversial or ambiguous decisions |
| **Trust Factor (GaaS)** | Agents earn/lose trust based on compliance and severity-weighted violations | Runtime governance (see arxiv 2508.18765) |

### Tool Use Pattern

```rust
trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, input: &str) -> Result<String, ToolError>;
}
```

Agent selects tools based on task. Domain core defines tool trait. Adapters implement concrete tools.

---

## 6 — ML Monitoring & Observability

### The 4 Signals

| Signal | What to watch | Alert threshold |
|---|---|---|
| **Latency** | Time-to-first-token, total inference time | p99 > 2x baseline |
| **Quality** | Output coherence, axiom scores, user feedback | Moving average drops below phi^-2 (38.2%) |
| **Resource** | VRAM, RAM, CPU, swap, disk I/O | Any resource > 90% sustained |
| **Drift** | Input distribution shift, output distribution shift | KL divergence > threshold |

### Logging Strategy

```
Structured log per inference:
{
  timestamp, request_id, backend_id, model_id,
  input_tokens, output_tokens, latency_ms,
  temperature, top_p, status (ok/error/timeout),
  circuit_breaker_state
}
```

### Health Hierarchy

```
System Health
├── Hardware Health (CPU, RAM, GPU, disk, network)
├── Service Health (gRPC server, SurrealDB connection)
├── Inference Health (backends available, circuit breaker states)
└── Cognitive Health (agent consensus quality, learning loop closure)
```

---

## 7 — Checklist (apply on EVERY AI infrastructure task)

```
Before building:
□ Which serving pattern? (single-node, multi-node, hybrid)
□ Which API standard? (OpenAI-compatible is default)
□ What's the memory budget? (model size vs available RAM/VRAM)
□ What's the latency target? (real-time vs batch)
□ How do we handle backend failure? (circuit breaker state machine)

During building:
□ InferencePort trait is the ONLY interface domain code touches
□ Backend selection strategy is configurable, not hardcoded
□ Health checks are periodic probes, not per-request
□ Concurrent request limit is enforced (bounded channel/semaphore)
□ Every inference is logged with structured data

After building:
□ Can I swap the backend without touching domain code?
□ Can I add a new backend by implementing one trait?
□ Does the system degrade gracefully when a backend dies?
□ Is inference latency monitored with alerts?
□ Can I replay any past inference given its log entry?
```

---

## References (for deep dives — fetch these when needed)

| Resource | What it covers |
|---|---|
| `github.com/ashishps1/awesome-system-design-resources` | System design patterns, distributed systems |
| `github.com/aishwaryanr/awesome-generative-ai-guide` | GenAI patterns, RAG, agents, LLMOps |
| `github.com/chiphuyen/machine-learning-systems-design` | ML system design framework |
| `github.com/GokuMohandas/Made-With-ML` | MLOps, training, deployment, CI/CD for ML |
| `github.com/karpathy/nn-zero-to-hero` | Neural network fundamentals, transformers, tokenization |
| `github.com/huggingface/transformers` | Model implementations, serving patterns |
