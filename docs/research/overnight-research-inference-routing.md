# Overnight Research: Inference Routing & Optimization
## What Six Starred Repos Can Teach CYNIC

**Date:** 2026-03-21
**Scope:** Architecture patterns, CPU inference optimizations, routing/selection logic, token compression — extracted from source code, not just READMEs.

---

## 1. BlockRunAI/ClawRouter — LLM Router for Agents

**What it is:** TypeScript LLM router, 44 models, <1ms local routing, crypto payment (x402/USDC), agent-native.

### Architecture

Request flow: `prompt → rule-based scorer (14 dimensions, <1ms) → if ambiguous: LLM classifier (~$0.00003/call, 200-400ms) → tier selection → model`.

Four tiers: SIMPLE / MEDIUM / COMPLEX / REASONING. Three routing profiles: eco / auto / premium. A fourth profile, "agentic", activates automatically when `agenticScore >= 0.5`.

**Key insight: two-stage classification with confidence gating.** The rule-based path handles 70-80% of requests at zero cost. The LLM fallback only fires when rule confidence is below a configurable threshold, using a sigmoid calibration of distance from tier boundaries.

### 14-Dimension Scoring (rules.ts)

Scores every request across weighted dimensions, each returning [-1, 1]:
1. Token count (short = simple, long = complex)
2. Code presence keywords
3. Reasoning markers
4. Technical terms
5. Creative markers
6. Simple indicators (negative weight)
7. Multi-step patterns
8. Question complexity
9. Agentic task indicators
10. Imperative verbs
11. Constraint indicators
12. Output format keywords
13. Reference complexity
14. Negation complexity
15. Domain-specific keywords

Critical detail: **system prompts are excluded from keyword scoring**. Scoring runs only on user message. System prompts contaminate scoring.

### Response Cache (response-cache.ts)

Content-addressed cache: SHA hash of canonicalized JSON `{model, messages, params}`. TTL 10 min, LRU at 200 entries, max 1MB/entry. Strips streaming flag and request IDs from cache key.

### Patterns for CYNIC

- **Two-stage Dog selection**: Run deterministic-dog first. Only escalate to LLM Dogs when confidence < threshold.
- **Confidence-gated escalation**: Sigmoid distance-from-boundary directly implementable in VerdictCache.
- **Content-addressed verdict cache**: Hash identical judgment requests to avoid re-running all Dogs.

---

## 2. rtk-ai/rtk — CLI Token Killer

**What it is:** Rust CLI proxy, 60-90% token reduction, <15ms overhead, SQLite tracking.

### Architecture

Six-phase: Parse → Route → Execute → Filter → Print → Track.

Hook-based deployment: `PreToolUse` hook rewrites `git status` → `rtk git status` transparently. 100% adoption with zero cognitive overhead.

### Filtering Strategies (filter.rs)

1. **Smart Filtering** — language-aware comment/boilerplate stripping
2. **Grouping** — aggregates similar items by pattern
3. **Truncation** — aggressive strips function bodies, leaving signatures
4. **Deduplication** — collapses repeated lines with counts

### Patterns for CYNIC

- **`rtk discover`** pattern: Analyzes history for missed optimization opportunities. CYNIC needs "dead Dog detection" — identifying duplicate/near-duplicate judgments.
- **Tiered compression**: Add `?format=compact` to `/judge` returning `{score, tier, top_signal}` instead of full 6-axiom breakdown.

---

## 3. RightNow-AI/picolm — 1B LLM on $10 Hardware

**What it is:** 2,500-line pure C inference engine. TinyLlama 1.1B at 45MB RAM. Zero deps, GGUF native, SIMD.

### Memory Architecture

**Model stays on disk via mmap, only one layer at a time in RAM.** OS paging = layer cache. Zero-copy.

KV cache is FP16 (not FP32). For 4B model at ctx 4096: ~80MB vs ~160MB.

### CPU SIMD (tensor.c)

Compile-time dispatch: ARM NEON / x86 SSE2 / scalar fallback. All hot-path ops use same 4-wide pattern.

Multi-threaded matmul: work-stealing row division. Thread 0 runs inline (no creation overhead).

Pre-computed RoPE tables eliminate transcendental calls from hot loop. Flash attention: online softmax eliminates O(seq_len) buffer.

### KV Cache Persistence

`--cache <file>` saves prompt state. Re-runs with same system prompt skip prefill entirely. **Directly applicable to CYNIC's fixed axiom system prompt.**

### Grammar-Constrained JSON

`--json` flag guarantees valid JSON from 1B model. Eliminates most common small-model failure mode.

### Patterns for CYNIC

- **KV cache persistence**: Add `--cache-prompt` to llama-server. Eliminates prefill on every sovereign-ubuntu request (~30-50% speedup).
- **Thread tuning**: Match physical cores, not logical (HT hurts inference).
- **JSON healing retry**: If parse fails, retry once at temperature 0.

---

## 4. AlexsJones/llmfit — Hardware-Aware Model Selector

**What it is:** Rust TUI/CLI, detects hardware, scores models across quality/speed/fit/context. REST API for schedulers.

### Scoring (fit.rs)

Four-component composite score (0-100 each):
- **Quality** = base(param_count) + family_bump + quant_penalty + task_alignment
- **Speed** = bandwidth-based: `(gpu_bandwidth_GBps / model_size_GB) * 0.55 * run_mode_factor`
- **Fit** = memory utilization efficiency
- **Context** = context window vs use-case target

CPU-only formula: `k / params_B * quant_speed_multiplier * thread_bonus`

### Quantization Hierarchy

```
Q8_0 → Q6_K → Q5_K_M → Q4_K_M → Q3_K_M → Q2_K
bpp:  1.05    0.85    0.68     0.58     0.48    0.37
penalty: 0    -1      -2       -5       -8      -12
```

`best_quant_for_runtime_budget` walks hierarchy top-down. Q4_K_M = sweet spot (penalty -5, speed +15%).

### REST API

`GET /api/v1/models/top?limit=5&min_fit=good&use_case=coding` — designed for cluster schedulers. Exactly the agent-to-CYNIC interaction model for inference proxy vision.

### Patterns for CYNIC

- **Hardware-aware Dog registration**: Query `llmfit serve` on each sovereign machine at registration time.
- **Bandwidth-based tok/s formula**: `(bandwidth / model_GB) * 0.55` for latency-aware routing.
- **`llmfit recommend`** for model selection instead of manual benchmarks.

---

## 5. ggml-org/whisper.cpp — CPU Inference Optimization

### Key Principles

- **Zero memory allocations at runtime.** All buffers pre-allocated at model load.
- SIMD dispatch: ARM NEON / x86 AVX2/AVX512 / POWER VSX / OpenVINO
- **OpenVINO backend**: runs on Intel iGPU at >3x vs CPU-only. Check Ubuntu machine for Intel iGPU.

### Thread Tuning

whisper.cpp prints thread/SIMD capabilities. Diagnostic pattern worth replicating in `/health`.

### Patterns for CYNIC

- **OpenVINO for iGPU**: `lspci | grep -i vga` — if Intel iGPU exists, potential free 3x acceleration.
- **`--n-parallel 1`** on llama-server eliminates dynamic allocation.
- **Add inference capabilities to `/health`**: SIMD features, thread count, KV cache type.

---

## 6. unslothai/unsloth — Fine-Tuning & Inference

### Technical Innovations

- **Custom Triton kernels**: Fused RMSNorm, RoPE, LoRA MLP in single passes
- **Gemma 2/3 specific**: Flash Attention with logit softcapping (`tanh(x/30)*30`)
- **Sequence packing**: Multiple short sequences in one context window, eliminating padding
- **Self-healing tool calling**: Auto-retry on invalid JSON

### Gemma 3 4B Fine-Tuning

- LoRA rank 16-32 sufficient for domain adaptation
- 4-bit base + LoRA fits in 6GB VRAM (S. RTX 4060 Ti)
- GRPO can align toward φ-scoring using existing verdict data as preference signal

### Patterns for CYNIC

- **Fine-tune sovereign-ubuntu on CYNIC verdicts**: 1,000+ (position, verdict) pairs → specialized chess-judgment Dog at $0/inference.
- **GRPO on verdicts**: Pipeline already produces (prompt, score, axiom_breakdown) tuples = direct training signal.

---

## Synthesis: What CYNIC Should Steal

### Tier 1 — Config-Only (today)

1. **`--cache-prompt`** on llama-server → skip prefill on repeated system prompt (~30-50% speedup)
2. **Verify SIMD flags**: `llama-server --version 2>&1 | grep -i avx` — confirm AVX2 active
3. **Thread count benchmark**: `-t 1,2,4,6,8` — find real optimum for Ubuntu CPU
4. **Confirm KV cache type**: Q8_0 already optimal per config

### Tier 2 — Design & Implement (1-3 days)

5. **Confidence-gated Dog escalation** (ClawRouter) — run deterministic-dog first, skip LLM Dogs if confidence > φ⁻¹
6. **Content-addressed verdict cache** — SHA256(model_id + normalized_prompt), TTL 1h
7. **Compressed verdict format** — `?format=compact` → `{score, tier, top_signal}`
8. **Inference diagnostics in `/health`** — SIMD, threads, KV type, ctx_size

### Tier 3 — Strategic (weeks)

9. **Hardware-aware Dog registration** via llmfit REST API
10. **`llmfit recommend`** for model selection
11. **Gemma 3 4B fine-tuning** on CYNIC verdict data (unsloth + S. GPU)
12. **Grammar-constrained JSON healing** in sovereign Dog adapter

## Cross-Cutting Observations

**The routing hierarchy problem**: All six repos converge — cost and latency scale with capability, most requests don't need max capability. CYNIC fans out to all Dogs simultaneously. Synthesis: **fan out only to the minimum Dog ensemble needed for the confidence target**.

**Measurement prerequisite**: rtk's `gain`, llmfit's bandwidth estimation, picolm's timing — you cannot optimize what you don't measure. CYNIC tracks judgment counts but not per-Dog latency/tokens. Adding per-Dog timing reveals bottlenecks.

**Sovereign inference is the right bet**: picolm runs LLM on $10 hardware. Unsloth makes fine-tuning accessible. llmfit proves hardware-aware selection is solved. Commodity hardware + specialized fine-tuning is increasingly competitive with API calls.

**Token compression compounds**: rtk saves 80% on CLI, ClawRouter saves 74-100% on routing. These multiply. CYNIC's compact verdict format is the same idea on its own API surface.
