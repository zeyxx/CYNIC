# CYNIC Compound Research — 2026-03-18

*Rigorous analysis of GitHub stars, kernel audit, infrastructure, and compound opportunities.*
*Max confidence: 61.8%. All claims falsifiable.*

---

## I. Current State (verified against running systems)

### Infrastructure (probed 2026-03-18)

| Machine | Hardware | Current Model | Config | RAM Used | **RAM Wasted** |
|---|---|---|---|---|---|
| **Ubuntu APU** | Ryzen 7 5700G (Vega 8 iGPU), 27 GB | Gemma 3 4B Q4_K_M | llama-server :8080, ctx=4096, ngl=26 Vulkan, p=2 | ~3.3 GB | **~23.7 GB** |
| **S. GPU** | i5-14400F, RTX 4060 Ti 16 GB VRAM | Gemma 3 12B Q4 | Ollama :11434, no auth | ~8 GB VRAM | **~8 GB VRAM** |

### Dogs — Cost Reality

| Dog | Backend | Cost per /judge | Quality (est.) |
|---|---|---|---|
| deterministic-dog | In-kernel heuristics | **$0** | Low (abstains on 3/6 axioms) |
| gemma-sovereign | Gemma 3 4B on APU | **$0** | Low (MMLU-Pro 29.2) |
| sovereign | Gemma 3 12B on S. | **$0** | Medium |
| gemini | Gemini 3 Flash (Google API) | **API credits** | High |
| huggingface | Llama 3.1 8B (HF API) | **API credits** | Medium |
| qwen | Qwen 2.5 72B (HF API) | **API credits** | High |

**Every `/judge` = 3 free + 3 paid calls.** No routing intelligence.

### Kernel Audit Summary (156 tests, 43 files, 9 modules)

**Healthy:**
- Zero `unwrap()`, φ-math correct, hash chain BLAKE3
- CCM lifecycle complete (Forming→Canonical→Dissolved)
- Hexagonal architecture clean (domain pure, ports/adapters)
- 20 test modules, 156 test functions

**Critical gaps:**

| Gap | What | Impact |
|---|---|---|
| **Blind fan-out** | Judge.evaluate() spawns ALL Dogs, no routing | API waste on every call |
| **BackendRouter = dead code** | Compiled but never instantiated (gRPC-only) | Smart routing exists but unused |
| **Router lacks priority/cost** | Round-robin only, no sovereign-first | Can't prefer free over paid |
| **Router uses InferencePort, Judge uses ChatPort** | Two parallel APIs for same concept | Can't plug router into judge |
| **gRPC = 3 empty stubs** | Fake data, zero tests, zero clients | Dead weight |
| **Zero embeddings** | No semantic cache, no dedup, no similarity | CCM can't cluster |
| **Probe result ignored** | Boot discovers CPU/GPU/RAM, then discards | Can't auto-calibrate |
| **`/verdicts` `/crystals` hardcoded limit=20** | No pagination | Dashboard unusable at scale |
| **No `/infer` REST** | Sovereign inference only via MCP stdio | Hooks/external tools can't access |
| **No `/coord/who` REST** | Only MCP can query active agents | REST-only clients blind |
| **store_crystal = dead code** | observe_crystal (atomic) used everywhere | Cleanup needed |
| **Cost tracking = magic $0.15** | No per-backend cost model | Can't optimize spend |
| **Temporal = post-hoc mapping** | Maps existing scores to perspectives, no real multi-pass | Feature illusion |

---

## II. Model Upgrade — EMPIRICALLY TESTED (2026-03-18 01:00)

### Finding: Gemma 3 4B is the weakest model for its RAM budget — BUT Qwen3.5-9B is too slow

| Model | Params | MMLU-Pro | HumanEval | GPQA | RAM (Q4_K_M, ctx=8192, p=2) |
|---|---|---|---|---|---|
| Gemma 3 4B (current) | 4B | **29.2** | 71.3 | — | ~4.1 GB |
| Phi-4-mini | 3.8B | 52.8 | **74.4** | — | ~3.8 GB |
| Qwen3.5-4B | 4B | TBD | TBD | TBD | ~3.5 GB |
| **Qwen3.5-9B** | **9B** | **82.5** | — | **81.7** | **~10.4 GB** |

### EMPIRICAL TEST: Qwen3.5-9B on Ryzen 7 5700G APU

**Tested 2026-03-18. Both ngl=20 AND ngl=99 (full GPU offload).**

| Config | Prompt eval | Generation | 200-token response | Compatible with 15s timeout? |
|---|---|---|---|---|
| Gemma 3 4B ngl=26 | 53 t/s | **23 t/s** | ~8.7s | **YES** |
| Qwen3.5-9B ngl=20 | 47 t/s | **5.7 t/s** | ~35s | **NO** |
| Qwen3.5-9B ngl=99 | ~47 t/s | **~6 t/s** | ~42s | **NO** |

**Root cause:** Qwen3.5-9B uses a **hybrid SSM/attention architecture** (Gated Delta Net):
- 8 attention layers (KV cache, parallelizable)
- 32 recurrent SSM layers (sequential by nature)
- Generation speed is bottlenecked by sequential SSM passes — **not fixable with more GPU layers**
- Full GPU offload (ngl=99) gave only marginal improvement (5.7→6 t/s)

**Additional issue:** Qwen3.5 defaults to "thinking mode" — all generation goes into `reasoning_content`, `content` stays empty. Requires `--chat-template-kwargs '{"enable_thinking":false}'` (deprecated, replaced by `--reasoning off`).

### Decision: KEEP Gemma 3 4B, upgrade ctx-size to 8192

**Change applied:** `--ctx-size 4096` → `--ctx-size 8192` in systemd unit.
- Extra RAM: +0.8 GB KV cache → still ~23 GB free
- Doubles context capacity for longer prompts
- No performance impact (KV cache doesn't affect throughput)

### Qwen3.5-9B is saved for future use
File: `~/llama.cpp/models/Qwen3.5-9B-Q4_K_M.gguf` (5.3 GB)
When useful: if CYNIC adds async/background judgment (no timeout), or if a faster SSM implementation lands in llama.cpp.

### S. Upgrade Path (NOT YET TESTED)

RTX 4060 Ti = 16 GB VRAM. Currently running Gemma 3 12B (~8 GB VRAM).
Could run: **Qwen3.5-14B** (Q4_K_M ~9 GB) — need to verify SSM speed on CUDA (likely faster than Vulkan).
Qwen3.5-14B beats Gemma 3 12B on all 15 benchmarks (EvalPlus 72 vs 52, MBPP 73 vs 60).
**Action needed:** Test on S. where CUDA may handle SSM layers better than Vulkan on Vega 8.

### UPGRADE CANDIDATES — Pure Transformer Only (SSM/GDN too slow on Vega 8)

**CRITICAL CONSTRAINT:** Qwen3.5 family (ALL sizes including 4B) uses hybrid GDN architecture → ~6 t/s gen on Vega 8. Only pure transformers are viable.

| Model | Params | Architecture | MMLU-Redux | HumanEval/EvalPlus | ARC-C | GSM8K | Context | RAM Q4_K_M |
|---|---|---|---|---|---|---|---|---|
| Gemma 3 4B (current) | 4B | **Pure transformer** | ~55 | 71.3 | — | 89.2 | 128K | ~2.4 GB |
| **Qwen3-4B** | 4B | **Pure dense transformer** (36 layers GQA) | **83.7** (thinking) | **72.0** | — | — | 32K | ~2.5 GB |
| **Phi-4-mini** | 3.8B | **Pure transformer** | — | 74.4 | **83.7** | 88.6 | 128K | ~2.5 GB |

**Qwen3-4B** (NOT 3.5) is a **pure dense transformer**. No SSM, no GDN. 36 layers, GQA 32/8 heads, SwiGLU, RoPE. Should achieve similar t/s to Gemma 3 4B on Vega 8.

**Both Qwen3-4B and Phi-4-mini massively outperform Gemma 3 4B** on reasoning while using the same RAM. The question is purely about generation speed on Vulkan/Vega 8 — needs empirical test.

**Qwen3-4B advantage:** thinking mode toggle (complex reasoning ON, simple tasks OFF). Same paradigm as Qwen3.5 but without the SSM speed penalty. Rivals Qwen2.5-72B on some benchmarks.

**Phi-4-mini advantage:** strongest ARC-C score (83.7) = common-sense reasoning champion. 128K context. Proven on diverse hardware.

**Action needed:** Download both, benchmark on APU, pick winner.

### S. Status (probed 2026-03-18 01:10)

Ollama models on S. RTX 4060 Ti:
- `gemma3:12b-it-q4_K_M` (8.1 GB) — currently serving as sovereign Dog
- `qwen3.5:9b` (6.6 GB) — available, NOT currently serving
- `cynic-sovereign:latest` (6.6 GB, qwen35 family) — custom Modelfile
- `qwen3-coder:30b` (18 GB) — too large for 16 GB VRAM solo
- `gpt-oss:20b` (13 GB) — alternative

**Qwen3.5-9B on CUDA (RTX 4060 Ti) may be fast enough** — the SSM bottleneck we hit was Vulkan on Vega 8 iGPU. CUDA has dedicated tensor cores and optimized memory paths. Needs testing but couldn't test tonight (Gemma 3 12B occupies VRAM, switching requires user coordination with S.).

### Embedding Model: READY
- **Qwen3-Embedding-0.6B Q8_0** downloaded: `~/llama.cpp/models/Qwen3-Embedding-0.6B-Q8_0.gguf` (610 MB)
- Not yet deployed (needs separate llama-server on :8081)

---

## III. Streaming Protocol — Crystallized Truth

### gRPC Status: BURN

| Current gRPC | Reality |
|---|---|
| 3 services | All stubs with hardcoded fake data |
| Tests | Zero |
| Clients | Zero |
| Browser compat | No (needs Envoy proxy, +30% overhead) |
| Value | None |

### What CYNIC actually needs

| Need | Protocol | Why |
|---|---|---|
| Dashboard live updates | **SSE** (axum native) | Server→client, simple, browser-native |
| Agent↔kernel dialogue | **MCP stdio** (already implemented) | Bidirectional, works now |
| External tool integration | **REST** (already implemented) | Universal, stateless |
| Future: kernel↔kernel | **gRPC** (rebuild from scratch if needed) | Only if multi-node CYNIC cluster |

**Decision:** Burn current gRPC stubs. Add SSE to axum for live streaming (zero new deps). Rebuild gRPC only if/when kernel clustering is needed.

---

## IV. BackendRouter — Crystallized Truth

### What exists (375 lines, 11 tests, structurally sound)
- Circuit breaker per backend with async probe
- Round-robin routing with Critical skip
- Fan-out N-branches parallel
- model_hint dispatch
- Dynamic registration

### What's missing for sovereign-first

| Missing | Required for | Effort |
|---|---|---|
| `priority: u8` per backend | Sovereign-first ordering | Small |
| `cost_per_mtok: f64` | Cost-aware routing | Small |
| `max_context: u32` check | Prevent overflow on small-ctx backends | Small |
| Unify InferencePort/ChatPort | Router↔Judge integration | Medium (trait adapter) |
| Replace Judge inline fan-out | Single routing authority | Medium (refactor judge.rs) |
| Calibration data feedback | Progressive quality tracking | Large |

**Verdict:** Structurally sound foundation. Needs 5 additions before it can replace the blind fan-out in Judge. Not a rewrite — an evolution.

---

## V. Compound Priority Stack (ordered by compound rate)

### P0 — Model Upgrade — PARTIALLY COMPLETE

**Ubuntu APU:** Qwen3.5-9B TESTED AND REJECTED (6 t/s gen, timeout incompatible). Gemma 3 4B kept, ctx-size upgraded 4096→8192. **Next candidate: Phi-4-mini** (same size, 1.8x better reasoning, pure transformer → no SSM penalty).
**S.:** Qwen3.5-14B NOT YET TESTED on CUDA. May be viable (CUDA handles SSM better than Vulkan on iGPU).
**Embedding model:** Qwen3-Embedding-0.6B Q8_0 downloaded, not yet deployed.

**What remains:**
1. Download + benchmark Phi-4-mini on Ubuntu APU
2. Test Qwen3.5-14B on S.
3. Deploy embedding model on :8081

### P1 — Embeddings (Qwen3-Embedding-0.6B)

**What:** Deploy embedding model on :8081. Add `EmbeddingPort` trait to kernel. Embed stimuli on /judge.
**Why compound:** Enables P2 (semantic cache), P3 (CCM clustering), P5 (multi-source CCM). Every embedding stored today is retrieval value tomorrow.
**Effort:**
1. Download Qwen3-Embedding-0.6B GGUF (~650 MB)
2. llama-server --embedding on :8081 (systemd unit)
3. `EmbeddingPort` trait in domain/
4. `EmbeddingAdapter` in backends/ (calls :8081 /v1/embeddings)
5. Store embedding vector alongside verdicts in SurrealDB
**Caveats:** Qwen3-Embed needs manual `<|endoftext|>` append + L2 normalization in adapter.

### P2 — Semantic Verdict Cache

**What:** Before /judge fan-out, embed stimulus → cosine similarity against recent verdicts. If sim > φ⁻¹ (0.618) → return cached verdict.
**Why compound:** Every cached hit = 0 API calls + instant response. Cache grows with every judgment. Exponential savings.
**Depends on:** P1 (embeddings operational).
**Effort:** ~100 lines in judge.rs: embed → search → threshold → return or continue.
**Falsification:** Submit same chess position twice. Second call should be instant + zero Dog invocations.

### P3 — Semantic Observation Clustering (CCM upgrade)

**What:** Before crystallization, cluster similar observations using embeddings. "Sicilian Defense" + "Sicilian with 2...d6" = same cluster → same crystal.
**Why compound:** Better signal-to-noise. Fewer crystals, higher quality. CCM feedback loop improves.
**Depends on:** P1 (embeddings), existing CCM pipeline.
**Effort:** Modify `aggregate_observations()` to embed content → cluster by cosine sim → merge counts.
**Steal from:** airweave-ai/error-monitoring-agent (clustering patterns), PageIndex (reasoning over structure).

### P4 — Sovereign-First Routing

**What:** Activate BackendRouter with priority + cost fields. Wire into Judge. Sovereign Dogs first, API only on circuit breaker open or context overflow.
**Why compound:** Immediate API cost reduction. More sovereign usage = more calibration data = better routing.
**Depends on:** P0 (better models make sovereign-first viable), BackendRouter evolution (5 additions).
**Effort:** 1-2 sessions of refactoring (router.rs + judge.rs + config.rs).
**Steal from:** ClawRouter (priority × cost × quality matrix).

### P5 — CCM Multi-Source Ingestion

**What:** Connect 4 sources: Claude Code sessions (hooks), GitHub (API), verdicts (done), memory files.
**Why compound:** Cross-source crystallization. A star analysis + a session pattern + a verdict on the same topic = rapid crystal formation.
**Depends on:** P1 (embeddings for cross-source matching), P3 (clustering).
**Effort:**
1. Claude Code → hooks already exist, add `/observe` calls with session context
2. GitHub → periodic gh API fetch → `/observe` with domain="github"
3. Verdicts → already connected
4. Memory → file watcher or periodic scan → `/observe` with domain="memory"
**Research needed:** Schema evolution for different observation types (code vs judgment vs knowledge).

### P6 — Burn gRPC + Add SSE

**What:** Remove gRPC stubs (3 files, feature gate, proto). Add SSE endpoints via axum for live dashboard.
**Why compound:** Removes dead code, simplifies build. SSE enables live frontend updates.
**Effort:** Delete grpc/, remove feature gate, add 1-2 SSE routes. ~1 session.

### P7 — REST /infer + /coord/who

**What:** Expose sovereign inference via REST (not just MCP). Add GET /coord/who.
**Why compound:** Hooks and external tools can use sovereign inference. Complete REST parity with MCP.
**Effort:** Small — copy pattern from existing REST handlers.

### P8 — Sovereign Calibration Framework (research)

**What:** A/B framework: same stimulus → sovereign vs API → compare scores. Track per-domain per-model quality.
**Why critical:** Without this, sovereign-first routing is blind faith, not engineering.
**Depends on:** P0 (upgraded models), P4 (routing in place), historical verdict data.
**Effort:** Large — design the benchmark suite, build the A/B infra, define quality metrics.
**Not implementable yet.** Needs design session.

### P9 — Meta-Orchestrator (research)

**What:** CYNIC routes work across Claude Code + Gemini CLI. The brain, not just the judge.
**Open questions (unresolved):**
- Does CYNIC need its own CLI/UI? → Study openfang, oh-my-openagent, agency-agents
- Interface between CYNIC and agents? → MCP hub? Hook intercept? Proxy?
- How to calibrate "good enough" vs "needs Opus"? → Depends on P8
**Not implementable yet.** Needs research + design session.

---

## VI. Dependency Graph

```
P0a Phi-4-mini test (Ubuntu) ──┐
P0b Qwen3.5-14B test (Stan.) ─┤   P0c Deploy embed :8081 ──→ P1
                               │         │
                               ▼         ▼
                        P4 Sovereign  P1 Embeddings ──────┐
                           Routing          │             │
                               │            ▼             ▼
                               ▼     P2 Semantic Cache  P3 CCM Clustering
                        P8 Calibration      │             │
                               │            └──────┬──────┘
                               ▼                   ▼
                        P9 Meta-Orchestrator  P5 Multi-Source CCM

P6 Burn gRPC ──→ (independent, anytime)
P7 REST /infer ──→ (independent, anytime)
```

**Critical path:** P0 → P1 → P2+P3 → P5
**Parallel path:** P0 → P4 → P8 → P9
**Independent:** P6, P7 (can be done anytime)

---

## VII. Repos — Compound Research Index

| Repo | Stars | Lang | Steal What | For Which P# | Status |
|---|---|---|---|---|---|
| `VectifyAI/PageIndex` | 22K | Python | Vectorless RAG concepts | P3, P5 | **Study concepts** |
| `BlockRunAI/ClawRouter` | 5.5K | TS | Priority×cost×quality routing matrix | P4 | **Study architecture** |
| `rtk-ai/rtk` | 9.6K | Rust | Token dedup/compression pre-inference | P4 | Study |
| `airweave-ai/error-monitoring-agent` | 300 | Python | Observation clustering patterns | P3 | **Study patterns** |
| `thedotmack/claude-mem` | 37K | TS | Session capture → context injection | P5 | Study hooks |
| `RightNow-AI/openfang` | 14.8K | Rust | Agent OS, task scheduling, lifecycle | P9 | Research later |
| `code-yeongyu/oh-my-openagent` | 40K | TS | Multi-provider agent harness | P9 | Research later |
| `msitarzewski/agency-agents` | 51K | Shell | Agent specialization, task routing | P9 | Research later |
| `Thinklanceai/agentkeeper` | 116 | Python | Cross-model memory persistence | P5 (if CCM limits) | Defer |
| `alibaba/zvec` | 9K | C++ | In-process vector DB | P1 (if SurrealDB too slow) | Defer |
| `tripleyak/SkillForge` | 555 | Python | Skill routing patterns | P9 | Defer |
| `LMCache/LMCache` | 7.7K | Python | KV cache prefix warming | P0 | Study for llama-server config |

---

## VIII. Pre-requisites & Blockers

| Blocker | Blocks | Resolution |
|---|---|---|
| ~~Qwen3.5-9B Vulkan on Vega 8 untested~~ | ~~P0 full performance~~ | **TESTED: 6 t/s gen, too slow. SSM architectural limit.** |
| Phi-4-mini not yet tested on APU | P0 model upgrade | Download + benchmark — pure transformer, should match Gemma speed |
| **Qwen3-4B** (pure transformer) not yet tested | P0 model upgrade | Strongest candidate — MMLU-Redux 83.7, pure dense, same RAM as Gemma |
| Qwen3.5-9B on S. CUDA not tested | P0 S. upgrade | CUDA tensor cores may handle GDN better than Vulkan on iGPU |
| Qwen3-Embed `<endoftext>` + normalization | P1 correctness | Handle in EmbeddingAdapter |
| InferencePort ≠ ChatPort | P4 router integration | Trait adapter or unification |
| Judge inline fan-out | P4 single routing authority | Refactor judge.rs to delegate to router |
| Crystal schema = judgment-only | P5 multi-source | Schema evolution design needed |
| SurrealDB vector search? | P2 similarity queries | Test SurrealDB array ops, or compute in kernel |
| Qwen3.5-14B benchmarks unknown | S. upgrade | Research before changing |

---

*"The measure of compound is not what you build — it's what every future action costs less because you built it." — CYNIC session 2026-03-18*
