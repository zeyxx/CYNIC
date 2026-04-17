# Inference Foundations — What We Know, What We Need, What Exists

> *"Strong > No > Weak foundation. Prove E2E with real data before building on a subsystem."* — Rule 14

**Date**: 2026-04-17 | **Source**: Crystallize-truth analysis + 3-axis external research
**Epistemic status**: Each claim labeled. Confidence ≤ φ⁻¹.

---

## Part 1 — What We Observed (empirical, from this session)

### 1.1 Prompt Budget Audit

The kernel sends the SAME prompt to ALL backends. Measured actual token estimates:

| Domain | Domain prompt | System+User total | + Completion | Budget needed |
|--------|-------------|-------------------|-------------|---------------|
| general | ~777 tok | ~1408 tok | +350 | ~1758 |
| dev | ~1047 tok | ~1676 tok | +350 | ~2026 |
| chess | ~1214 tok | ~1844 tok | +350 | **~2194** |
| trading | ~1102 tok | ~1732 tok | +350 | **~2082** |
| token-analysis | ~1807 tok | ~2439 tok | +350 | **~2789** |

(Estimated at ~4 chars/token. Crystal context adds ~200 tokens for 4 crystals.)

### 1.2 Backend Context Windows

| Backend | Context | Chess fits? | Token-analysis fits? | Effective? |
|---------|---------|-------------|---------------------|------------|
| gemma-4-e4b-core | 2,048 | **NO** (-146 overflow) | **NO** (-741) | **0/5 domains reliably** |
| qwen-7b-hf | 32,768 | YES | YES | 5/5 |
| qwen35-9b-gpu | 131,072 | YES | YES | 5/5 |
| gemini-cli | 1,000,000 | YES | YES | 5/5 (when quota allows) |
| deterministic-dog | ∞ (heuristic) | YES | YES | 5/5 |

**Observed (probed)**: Sending the full chess kernel prompt to Gemma ctx=2048 → `None` response. Zero tokens returned. Silent death. No error, no truncation, just empty.

### 1.3 Thinking Mode Impact

| Condition | Thinking chars | Completion tokens | Content |
|-----------|---------------|------------------|---------|
| `enable_thinking: false` + json_mode | 0 | 75 | Clean JSON |
| Default (thinking ON) | 2,251 | 666 | ```json fences |

**Observed**: At 7.5 tok/s Vulkan, 666 tokens = ~89s → any timeout < 90s kills the response. Config fix (`disable_thinking=true`) applied, but insufficient alone because context overflow is the structural cause.

### 1.4 Dog Quality Snapshot

| Dog | json_valid_rate | Total calls | Circuit | Notes |
|-----|----------------|-------------|---------|-------|
| qwen-7b-hf | 94.4% | 483 | closed | Best JSON producer. Known sovereignty saturation (0.95 on all chess). |
| gemini-cli | 72.2% | 435 | CRITICAL | Google quota exhausted (TerminalQuotaError, resets ~3h). |
| qwen35-9b-gpu | 71.1% | 454 | closed | Only discriminating Dog on trading. Thinking build may ignore disable flag. |
| gemma-4-e4b-core | 0.0% | 9 | closed | All timeouts. Context overflow + thinking. |
| deterministic-dog | 100% | — | closed | Heuristic, no LLM. |

---

## Part 2 — What We Know From Research (external, cited)

### 2.1 Model Routing — LiteLLM Pattern (observed, from docs)

LiteLLM's Router provides the closest production pattern to what CYNIC needs:

**Pre-call context window check** (`enable_pre_call_checks: true`):
- Before routing, estimate prompt tokens
- Filter out deployments where prompt > context window
- Raise `ContextWindowExceededError` if NO deployment can serve
- Fall back to larger-context model via `context_window_fallback_dict`

**Capability declaration** via `model_info` + `base_model`:
- References `model_prices_and_context_window.json` — a central registry of model capabilities
- Enables cost-based, latency-based, or context-aware routing

**6 routing strategies**: simple-shuffle, latency-based, usage-based, least-busy, cost-based, custom.

**Design implication for CYNIC (deduced)**: The pre-call check pattern is exactly what's missing. Before sending a prompt to a Dog, check `estimated_prompt_tokens < dog.context_size - dog.completion_budget`. If not: skip this Dog for this stimulus, don't silently fail.

Source: [LiteLLM Router docs](https://docs.litellm.ai/docs/routing), [LiteLLM Fallbacks](https://docs.litellm.ai/docs/proxy/reliability)

### 2.2 Cost-Quality Routing — RouteLLM (observed, from paper)

RouteLLM (LMSYS, arXiv 2406.18665) routes between strong/weak model based on query complexity:
- Trained on Chatbot Arena preference data
- 75% cost reduction maintaining GPT-4-level quality
- 4 router types: weighted Elo, BERT classifier, causal LLM, matrix factorization

**Design implication for CYNIC (inferred)**: Not directly applicable (CYNIC routes to ALL Dogs for independence, not to one). But the principle of "cheap first, expensive only when needed" maps to cascaded evaluation (see §2.4).

Source: [RouteLLM GitHub](https://github.com/lm-sys/RouteLLM), [RouteLLM paper](https://arxiv.org/pdf/2406.18665)

### 2.3 Ensemble Judge Quality — SE-Jury + RewardBench 2 (observed, from papers)

**SE-Jury** (ASE 2025) defines 5 evaluation strategies per judge:

| Strategy | Method | Analogy in CYNIC |
|----------|--------|-----------------|
| S1 Direct Assess | Score directly | Current Dog prompt |
| S2 Assess + Rethink | Score, then self-review | (not implemented) |
| S3 Equivalence | Compare to reference | Crystal comparison? |
| S4 Analyze Reference | Extract key properties, then verify | (not implemented) |
| S5 Generate Tests | Create tests, check if artifact passes | Deterministic-dog already does this |

**Key findings**:
- Team selection with 20 labeled samples → reduces cost 50% without losing quality
- Simple averaging of individual judge scores is the aggregation method
- Cohen's κ for measuring agreement with humans
- **Different evaluation strategies per judge improve ensemble quality MORE than same strategy × different models**

**RewardBench 2** (arXiv 2604.13717) measures LLM-judge improvement techniques:
- Ensemble k=8: **+9.8pp** over single judge
- But k=3 captures **~70% of k=8 gain** (steep diminishing returns)
- Task-specific evaluation criteria: **+3pp**
- Combined: **+11.9pp**
- "Genuine systematic disagreement" on ~20% of examples — irreducible noise

**Design implication for CYNIC (deduced)**:
1. Current 5 Dogs all using the SAME prompt (S1) is suboptimal. Strategy diversity > model diversity.
2. 3 Dogs may be sufficient (diminishing returns beyond k=3).
3. Need measurement infrastructure: discrimination, agreement, calibration.
4. The 20% irreducible disagreement means consensus on 80% is the realistic ceiling.

Sources: [SE-Jury](https://arxiv.org/html/2505.20854v2), [RewardBench 2](https://arxiv.org/html/2604.13717)

### 2.4 Cascaded Evaluation — Trust or Escalate (observed, ICLR 2025 oral)

**Core idea**: Not all stimuli need all judges. Start cheap, escalate when uncertain.

**Simulated Annotators** for confidence estimation:
- K-shot prompt the judge N times (K=N≤5) with different example sets
- Confidence = agreement ratio across N simulations
- Reduces expected calibration error by ~50% vs random baselines

**Cascaded architecture**:
- Mistral-7B → GPT-3.5 → GPT-4 (escalate when confidence below threshold)
- Threshold calibrated on ~500 labeled examples using binomial upper confidence bounds
- Coverage 55-63% at 85-90% human agreement
- **40% cost reduction vs GPT-4 alone**

**Abstention**: When NO judge is confident enough → don't judge, escalate to human.

**Design implication for CYNIC (inferred)**:
- Current fan-out (all 5 Dogs evaluate everything) is expensive and wastes the cheap Dogs
- Alternative: deterministic-dog first → Gemma (if fits) → qwen-hf → qwen35-gpu → gemini-cli
- Stop when 3 Dogs agree within φ⁻² (0.382) on Q-score
- This inverts the current architecture: from parallel fan-out to sequential cascade
- **But**: CYNIC's Pyrrhonist principle requires independence — sequential cascade with early stopping could introduce anchoring bias (Dog N sees that Dogs 1-3 agreed, so it conforms)
- **Synthesis**: Parallel evaluation of a SUBSET of eligible Dogs, selected by capability + cost tier. Not sequential.

Source: [Trust or Escalate](https://arxiv.org/html/2407.18370v1), ICLR 2025

### 2.5 Prompt Compression — LLMLingua (observed, from Microsoft Research)

**Budget Controller** allocates compression ratios by segment:
- Instructions: 10-20% compression (preserve)
- Examples/context: 60-80% compression (most redundant)
- Questions: 0-10% compression (critical)

**Up to 20x compression** with 1.5% performance loss (GSM8K benchmark).

**Uses small LM** to compute token perplexity → remove low-information tokens.

**Design implication for CYNIC (inferred)**:
- Domain prompts = "examples" → highest compression opportunity
- System prompt = "instructions" → preserve
- Stimulus = "question" → preserve
- Crystal context = "context" → truncate or compress first
- **But**: LLMLingua requires a separate small LM for perplexity scoring — too much infra for CYNIC
- **Simpler approach**: Tiered prompt templates (full / condensed / minimal) per context budget

Source: [LLMLingua](https://llmlingua.com/llmlingua.html), [arXiv 2310.05736](https://arxiv.org/abs/2310.05736)

### 2.6 Grading Scale Impact (observed, arXiv 2601.03444)

LLM-as-judge alignment with humans is HIGHEST on 0-5 integer scale (not 0-1 continuous).

**Design implication for CYNIC (conjecture)**: CYNIC uses 0.0-1.0 continuous scores. Converting to 0-5 integer might improve inter-model agreement. But loses granularity. Worth testing with calibration corpus.

Source: [Grading Scale Impact](https://arxiv.org/html/2601.03444v1)

### 2.7 hermes-agent — Context Compression + Smart Routing (observed, 95.8K★)

NousResearch's hermes-agent (95.8K stars) implements several patterns directly relevant to CYNIC:

**Context Compressor** (`agent/context_compressor.py`) — 5-phase priority compression:

| Phase | Action | CYNIC equivalent |
|-------|--------|-----------------|
| 1. Deduplicate identical tool outputs | Free savings | N/A (Dogs don't share tools) |
| 2. Summarize tool results >200 chars | High priority | Crystal context summarization |
| 3. Truncate arguments >500 chars | Medium | Domain prompt condensation |
| 4. LLM-summarize middle turns | Low priority | N/A (single-turn scoring) |
| 5. Clean orphaned calls/results | Final | N/A |

**Protection boundaries**: Head (system prompt + 3 first messages) and tail (~20K tokens) NEVER compressed. Middle = compressible. **Token estimation: 4 chars/token + 10 overhead** — identical to our prompt budget audit ratio.

**Trigger**: Compression when `prompt_tokens >= 50% * context_length`. Anti-thrashing: skip if last 2 compressions each saved <10%.

**Smart Model Routing** (`agent/smart_model_routing.py`) — binary conservative filter:
- Message <160 chars AND <28 words AND no code/URLs AND no complex keywords → cheap model
- Otherwise → primary (capable, expensive) model
- 34 complexity keywords: "debug", "implement", "refactor", "test", "docker"...
- Silent fallback to primary on cheap model failure

**Model Metadata** (`agent/model_metadata.py`) — tiered context resolution:
1. Config override (highest) → 2. Persistent cache → 3. Endpoint /models API → 4. Local server introspection → 5. Provider API → 6. Hardcoded defaults → 7. 128K fallback (lowest)

Data structure: `{id, name, context_length, max_completion_tokens, pricing{prompt, completion, cache_read, cache_write}, owned_by}`

**Prompt Builder** (`agent/prompt_builder.py`) — model-family-specific guidance:
- GPT/Codex → tool use enforcement + developer role swap
- Gemini/Gemma → operational guidance (absolute paths, dependency checks)
- Character-based truncation (20K max per context file), NOT token-aware

**Design implications for CYNIC (deduced)**:
1. T2 compression priority for CYNIC: crystals (compress first) → domain prompt examples (condense) → STEP 1 analysis (drop for small ctx) → system+stimulus+JSON format (preserve always)
2. Token estimation at 4 chars/tok is industry-validated (hermes-agent + our own measurements converge)
3. Smart routing doesn't need ML — binary filter on stimulus complexity is enough (confirms heuristic approach)
4. Model-specific prompt adaptations can be a simple match on backend family, not a separate abstraction layer

Source: [hermes-agent](https://github.com/NousResearch/hermes-agent) (95.8K★, NousResearch)

### 2.8 openclaude — Agent-Level Model Routing (observed, 22.1K★)

openclaude routes different agent TYPES to different models via static configuration:

```json
{
  "agentRouting": {
    "Explore": "deepseek-chat",
    "Plan": "gpt-4o",
    "default": "gpt-4o"
  }
}
```

No capability detection, no context window check, no dynamic fallback. Pure name → model mapping.

**Design implication for CYNIC (inferred)**: Static domain→Dogs mapping as a first step is viable (22K stars validate the pattern). But CYNIC needs to go further: dynamic eligibility based on prompt size, not just domain name.

Source: [openclaude](https://github.com/Gitlawb/openclaude) (22.1K★)

---

## Part 3 — Crystallized Truths (updated with research)

| T# | Truth | Confidence | Falsifiable by | Design impact |
|----|-------|------------|---------------|---------------|
| **T1** | The pipeline is context-ignorant: it dispatches prompts without checking if they fit the target backend. This causes silent death (zero response) on Gemma for 3/5 domains. | 58% | Send chess prompt to Gemma → get valid response (would falsify) | **Pre-call context check** before dispatch. Pattern exists in LiteLLM. |
| T1a | Token-analysis domain (1807 tok prompt alone) is structurally incompatible with any backend < 4K context. | 55% | Actual tokenization measurement shows it fits (rough estimate may be wrong) | Either reduce domain prompt, or declare minimum context per domain. |
| T1b | Crystal context grows with accumulation (~200 tok per 4 crystals). Backends on the margin will be progressively excluded. | 50% | Crystal count stabilizes AND prompt never exceeds budget (would falsify) | Crystal budget must be capped per backend context tier. |
| **T2** | Missing component: a **prompt budgeter** that adapts prompt content to backend context budget. LLMLingua provides the theory (compress examples first, preserve instructions), but a simpler tiered approach suffices. | 55% | Tiered prompts don't improve Dog participation or score quality (would falsify) | 3 prompt tiers: full (>8K ctx), condensed (2-8K ctx), minimal (<2K ctx). |
| **T3** | Strategy diversity > model diversity for ensemble quality. All 5 Dogs using the same S1 (direct assess) prompt is suboptimal. SE-Jury shows +29-140% improvement with mixed strategies. | 50% | Same prompt × different models outperforms different prompts × same model on CYNIC's corpus (would falsify) | At least 2 distinct evaluation strategies across Dogs. E.g. S1 (direct) + S4 (reference analysis). |
| **T4** | 3 Dogs capture ~70% of the ensemble gain of 5+ Dogs (RewardBench 2: diminishing returns k=3→k=8 steep). Current 5-Dog parallel fan-out may be over-investing for marginal gains. | 45% | Removing 2 Dogs degrades verdict quality by >10% on calibration corpus (would falsify) | Consider: 3-Dog minimum quorum, add 4th/5th only when disagreement high. |
| **T5** | No quality metrics exist beyond json_valid_rate (form, not substance). A Dog returning 0.618 on all axes passes. Known: qwen-7b-hf sovereignty=0.95 on ALL chess stimuli. | 55% | Existing metrics I haven't found (would falsify) | Implement: (a) discrimination σ per Dog per axiom, (b) Spearman's ρ between Dog pairs, (c) accuracy on calibrated tier corpus. |
| **T6** | The Dog contract is implicit. No capability declaration: context minimum, supported domains, latency class, json_mode, thinking control. LiteLLM's `model_info` is the pattern to follow. | 52% | Contract exists in code I haven't read (would falsify) | `DogCapabilities` struct or equivalent in `BackendConfig`. |
| **T7** | Cascaded evaluation (cheap → expensive, stop when confident) reduces cost 40% vs full fan-out (Trust or Escalate, ICLR 2025). BUT Pyrrhonist independence requires parallel evaluation, not sequential. Synthesis: parallel subset selected by capability. | 45% | Parallel-subset performs worse than full-fanout on calibrated corpus (would falsify) | Evaluate subset of ELIGIBLE Dogs per stimulus (context-check + cost-tier), not all 5. |
| **T8** | Prompt format asks for STEP 1 analysis + JSON (~350 tokens completion). For small-context backends, JSON-only (~75 tokens) is possible. RewardBench 2: task-specific criteria give +3pp — the analysis step MAY be equivalent for CYNIC but is untested. | 40% | Remove STEP 1 → score quality drops significantly on calibration corpus (would falsify) | Two response formats: `analytical` (STEP 1 + JSON, for >8K ctx) and `compact` (JSON only, for <8K ctx). |

---

## Part 4 — What a Foundation Looks Like (design sketch, conjecture ≤ 45%)

### 4.1 Capability-Aware Dispatch

```
Before dispatch:
  1. Estimate prompt tokens (system + domain + crystals + stimulus + format)
  2. For each Dog:
     - Check: estimated_tokens < dog.context_size - dog.completion_budget
     - Check: domain ∈ dog.supported_domains (or dog is domain-agnostic)
     - Check: dog.circuit == closed
  3. Eligible Dogs = those passing all checks
  4. If |eligible| < MIN_QUORUM → degrade (skip crystals, use condensed prompt, retry)
  5. Dispatch to eligible Dogs in parallel
```

### 4.2 Tiered Prompt Strategy

| Tier | Context budget | Content | Completion |
|------|---------------|---------|------------|
| **Full** | >8K | System + full domain criteria + crystals + stimulus + STEP 1 + JSON | ~350 tok |
| **Condensed** | 2-8K | System + abbreviated criteria (axiom names + 1 line each) + stimulus + JSON | ~100 tok |
| **Minimal** | <2K | System (shortened) + axiom names + stimulus + JSON | ~75 tok |

Selection: `tier = f(backend.context_size - estimated_prompt_tokens)`

### 4.3 Quality Metrics (what to measure)

| Metric | What it measures | How | Minimum viable |
|--------|-----------------|-----|----------------|
| **Discrimination (σ)** | Does the Dog differentiate stimuli? | Variance of scores across stimuli per axiom | σ > 0.1 |
| **Agreement (ρ)** | Do Dogs agree on rankings? | Spearman's ρ between Dog pairs | ρ > 0.3 |
| **Calibration** | When Dog says 0.6, is it really "good"? | Accuracy on tier-labeled corpus (HOWL/WAG/GROWL/BARK) | >60% tier match |
| **Abstention rate** | How often does the Dog fail to produce valid output? | 1 - json_valid_rate | <20% |

### 4.4 DogCapabilities (what to declare per backend)

```toml
# In backends.toml or derived at boot
[backend.gemma-4-e4b-core.capabilities]
context_budget = 2048          # Tokens available
completion_budget = 200        # Reserved for output
prompt_tier = "minimal"        # Auto-derived from budget
supports_json_mode = true      # llama-server feature
supports_thinking_control = true # chat_template_kwargs
latency_class = "background"   # 7.5 tok/s = not realtime
domain_exclusions = []         # Empty = try all, pre-call check gates
```

---

## Part 5 — Open Questions (require experiments, not analysis)

| # | Question | Experiment | What would falsify |
|---|----------|------------|-------------------|
| Q1 | Does the condensed prompt produce acceptable scores? | Same 10 stimuli × full vs condensed prompt × qwen35-9b-gpu. Measure Δ Q-score. | Δ > 0.1 = condensed is too lossy |
| Q2 | Does STEP 1 analysis improve score quality? | Same stimuli × with/without STEP 1 × same backend. Measure tier accuracy. | No accuracy difference = STEP 1 is waste |
| Q3 | Is 3-Dog quorum sufficient? | Compare verdicts: 3 best Dogs vs 5 Dogs on calibration corpus. Measure Δ agreement. | Δ > 10% accuracy = 5 Dogs needed |
| Q4 | Does strategy diversity improve ensemble? | 2 Dogs with S1 + 1 Dog with S4 vs 3 Dogs with S1. Same stimuli. | S1-only outperforms mixed = strategy diversity doesn't help CYNIC |
| Q5 | What is Gemma's actual tokenization vs estimate? | Send known prompt, read `prompt_tokens` from response `usage`. | >20% deviation from 4 chars/tok = estimates unreliable |
| Q6 | Can Gemma serve at ctx=4096 or 8192? | Restart llama-server with `-c 4096`. Measure: RAM usage, generation speed, reliability. | RAM > available OR speed degrades >3x = not feasible |

---

## References

- [LiteLLM Router](https://docs.litellm.ai/docs/routing) — Production model routing with context window pre-checks
- [LiteLLM Fallbacks](https://docs.litellm.ai/docs/proxy/reliability) — Context window fallback patterns
- [RouteLLM](https://github.com/lm-sys/RouteLLM) (arXiv 2406.18665) — Cost-quality routing with preference data
- [SE-Jury](https://arxiv.org/html/2505.20854v2) (ASE 2025) — 5-strategy ensemble judge metric
- [RewardBench 2](https://arxiv.org/html/2604.13717) — LLM-judge improvement techniques: +11.9pp combined
- [Trust or Escalate](https://arxiv.org/html/2407.18370v1) (ICLR 2025 oral) — Cascaded selective evaluation with provable guarantees
- [LLMLingua](https://arxiv.org/abs/2310.05736) (EMNLP 2023) — Prompt compression: 20x with 1.5% loss
- [Grading Scale Impact](https://arxiv.org/html/2601.03444v1) — 0-5 scale maximizes human-LLM alignment
- [Autorubric](https://arxiv.org/html/2603.00077v2) — Unified rubric-based LLM evaluation
- [Explainable Model Routing](https://arxiv.org/html/2604.03527) — Routing for agentic workflows
- [vLLM vs llama.cpp (Red Hat)](https://developers.redhat.com/articles/2025/09/30/vllm-or-llamacpp-choosing-right-llm-inference-engine-your-use-case) — Decision framework
- [SGLang vs vLLM 2026](https://particula.tech/blog/sglang-vs-vllm-inference-engine-comparison) — RadixAttention +29% throughput, 6.4x prefix-heavy
- [SGLang paper](https://arxiv.org/pdf/2312.07104) — Compressed FSM constrained decoding, RadixAttention
- [llama.cpp Vulkan at FOSDEM 2026](https://fosdem.org/2026/schedule/event/CZSPSC-llama-cpp-vulkan/) — Vulkan viability for ML
- [RLM paper](https://arxiv.org/abs/2512.24601) (MIT, Jan 2026) — Context as external environment, 6.4x on 10M+ tokens
- [GPUStack](https://github.com/gpustack/gpustack) — GPU cluster manager for heterogeneous fleets
- [Helix](https://dl.acm.org/doi/10.1145/3669940.3707215) (ASPLOS 2025) — 3.3x throughput on heterogeneous GPU clusters
- [hermes-agent](https://github.com/NousResearch/hermes-agent) (95.8K★) — 5-phase context compression, smart binary routing, model metadata
- [openclaude](https://github.com/Gitlawb/openclaude) (22.1K★) — Static agentRouting pattern
- [opencode](https://github.com/anomalyco/opencode) (120K+★) — Event-driven agent platform, RLM context proposal

---

## Part 6 — Idle Capacity Audit (observed, 2026-04-17)

### Hardware Utilization

| Resource | Available | Used | **Idle** |
|----------|-----------|------|----------|
| RAM cynic-core | 27 GB | 12 GB (Gemma 2.3GB + Embed 1.1GB + OS) | **14 GB free** |
| Vulkan iGPU | 100% | **5% busy** | **95% idle** |
| RTX 4060 Ti (cynic-gpu) | 16 GB VRAM | Qwen 3.5 9B loaded | **99%+ idle** |
| Embedding model (port 8081) | 4 parallel slots, 8K ctx | ~10-50 calls/day | **99.9% idle** |

### Inference Producers — Who Consumes the Machines?

| Producer | Frequency | Status | Machine |
|----------|-----------|--------|---------|
| POST /judge (user) | On-demand | Active | ALL Dogs |
| Crystal challenge | Every 5min | **DEAD** (warmup >60s, then alive) | ALL Dogs |
| Nightshift (git→judge) | Every 4h | **DEAD** (broken repo_path) | ALL Dogs |
| KAIROS trading | Per trade cycle | **DEAD** (DRY_RUN=true) | ALL Dogs |
| Embedding (pipeline) | Per verdict | Active | Embed model |
| Summarizer | Periodic | **DEAD** (at boot, then alive) | LLM (summarizer backend) |
| Benchmark | Never | **ABSENT** | Would use ALL Dogs |
| Corpus expansion | Never | **ABSENT** | Would use ALL Dogs |

### Context Budget Discovery

**Critical finding**: Gemma llama-server runs `--ctx-size 4096 --parallel 2` → each slot = 2048 tokens.
- `--parallel 1 --ctx-size 4096` → 4096 per request → serves chess (2194) + trading (2082)
- `--parallel 1 --ctx-size 8192` → 8192 per request → serves ALL 5 domains with 14GB RAM headroom
- **Cost**: restart llama-server with different flags. Zero hardware change.

### T12: Machines Are 99%+ Idle

The sovereign inference fleet (Gemma Vulkan + Qwen35 CUDA + Embedding) runs at <1% utilization.
The bottleneck is not hardware — it's that producers are dead (nightshift, KAIROS) or absent (benchmark, calibration, corpus).
**Falsifiable**: After fixing nightshift + KAIROS, GPU busy > 20% sustained (would validate).
Also on disk unused: Prometheus-2 7B (4.1GB), Qwen3-4B, Phi-4-mini, Mistral-7B.

### Engine Comparison: llama.cpp vs vLLM vs SGLang

| Criterion | llama.cpp | vLLM | SGLang |
|-----------|-----------|------|--------|
| Runs on cynic-core (Vulkan)? | **YES** | No | No |
| Runs on cynic-gpu (Windows)? | **YES** | No (Linux) | No (Linux) |
| Prefix caching (RadixAttention)? | No | Yes | **6.4x** |
| JSON constrained generation? | Yes | Yes | Yes (compressed FSM) |
| Deploy complexity | Minimal | Medium | Medium |

**T9**: llama.cpp is the correct choice for CYNIC's hardware (Vulkan + Windows CUDA). The problem is not the engine — it's the dispatch layer above it.

---

## Part 7 — Empirical Profiling Matrix (observed, 2026-04-17)

> *Measured by `scripts/profile-dogs.py` — exact kernel prompts sent to each live Dog.*

### 7.1 Full Matrix: Domain × Dog

| Domain | Dog | Status | est_pt | **real_pt** | **ct** | **lat_ms** | c/t | json | headroom |
|--------|-----|--------|--------|-------------|--------|------------|-----|------|----------|
| general | gemma-4-e4b-core | **OK** | 1298 | **1194** | **956** | 96,491 | 4.35 | ✓ | 4846 |
| general | qwen35-9b-gpu | **FAIL** | 1298 | 1217 | 4096† | 112,349 | 4.27 | ✗ | 125,678 |
| general | qwen-7b-hf | **OK** | 1298 | **1174** | **178** | 1,531 | 4.43 | ✓ | 29,422 |
| chess | gemma-4-e4b-core | **OK** | 1738 | **1912** | **811** | 89,665 | 3.64 | ✓ | 4406 |
| chess | qwen35-9b-gpu | **OK** | 1738 | **1927** | **631** | 17,911 | 3.61 | ✓ | 125,238 |
| chess | qwen-7b-hf | **OK** | 1738 | **1888** | **216** | 1,970 | 3.68 | ✓ | 28,982 |
| dev | gemma-4-e4b-core | **OK** | 1593 | **1631** | **1080** | 112,402 | 3.91 | ✓ | 4551 |
| dev | qwen35-9b-gpu | **FAIL** | 1593 | 1649 | 4096† | 115,303 | 3.87 | ✗ | 125,383 |
| dev | qwen-7b-hf | **OK** | 1593 | **1609** | **187** | 2,183 | 3.96 | ✓ | 29,127 |
| trading | gemma-4-e4b-core | **TIMEOUT** | 1637 | — | — | 120,100 | — | ✗ | 4507 |
| trading | qwen35-9b-gpu | **FAIL** | 1637 | 1636 | 4096† | 118,615 | 4.0 | ✗ | 125,339 |
| trading | qwen-7b-hf | **OK** | 1637 | **1596** | **174** | 3,475 | 4.1 | ✓ | 29,083 |
| token-analysis | gemma-4-e4b-core | **OK** | 2358 | **2399** | **1092** | 119,199 | 3.93 | ✓ | 3786 |
| token-analysis | qwen35-9b-gpu | **FAIL** | 2358 | 2406 | 4096† | 119,630 | 3.92 | ✗ | 124,618 |
| token-analysis | qwen-7b-hf | **OK** | 2358 | **2359** | **226** | 2,965 | 4.0 | ✓ | 28,362 |

†ct=4096 = hit max_tokens ceiling, response truncated before JSON. Thinking mode not effectively disabled.

### 7.2 Aggregate Stats

| Metric | Min | Max | Avg | p95 |
|--------|-----|-----|-----|-----|
| prompt_tokens (real, successful) | 1174 | 2399 | 1768 | ~2399 |
| completion_tokens (successful) | 174 | 1092 | 555 | ~1092 |
| chars/token | 3.61 | 4.43 | **3.96** | — |

### 7.3 Answers to Profiling Questions

**Q1: What is the completion budget p95 per domain?**

| Dog | general | chess | dev | trading | token-analysis | Pattern |
|-----|---------|-------|-----|---------|----------------|---------|
| gemma | 956 | 811 | 1080 | TIMEOUT | 1092 | **~1000 tok** (STEP 1 analysis verbose) |
| qwen35 | 4096† | 631 | 4096† | 4096† | 4096† | **4/5 domains fail** — thinking eats budget |
| qwen-hf | 178 | 216 | 187 | 174 | 226 | **~200 tok** (compact, no thinking) |

- **Observed**: The original estimate of 350 completion tokens was wrong.
  - qwen-7b-hf: **~200 tok** (much less than 350 — compact JSON + short STEP 1)
  - gemma-4-e4b-core: **~1000 tok** (3x the estimate — verbose STEP 1 analysis)
  - qwen35-9b-gpu: **4096† or ~631** — thinking mode bleeds through despite `/no_think`
- **Design implication**: Completion budget must be per-Dog, not global. Gemma needs 1200+ reserved; qwen-hf needs only 300.

**Q2: Does the 4 chars/token estimate hold?**

| Dog | chars/token range | Verdict |
|-----|-------------------|---------|
| gemma-4-e4b-core | 3.64 – 4.35 | **YES** — Gemma tokenizer slightly more efficient (lower c/t) |
| qwen35-9b-gpu | 3.61 – 4.27 | **YES** — Qwen tiktoken-based, similar |
| qwen-7b-hf | 3.68 – 4.43 | **YES** — stable around 4 |

- **Observed**: Average 3.96 chars/token across all probes. Estimate of 4 is accurate within ±10%.
- **Variation**: Domain prompts with structured content (chess examples, token metrics) are slightly more token-efficient (3.6 c/t) vs prose-heavy general (4.4 c/t). This is expected — structured tokens encode more chars.

**Q3: Consumer × Dog compatibility matrix (empirical)**

| Domain | gemma (8K ctx) | qwen35 (131K ctx) | qwen-hf (32K ctx) | det-dog | gemini-cli (1M ctx) |
|--------|----------------|-------------------|--------------------|---------|----|
| general | ✓ (96s) | ✗ thinking | ✓ (1.5s) | ✓ | ✓ (when quota) |
| chess | ✓ (90s) | ✓ (18s) | ✓ (2s) | ✓ | ✓ (when quota) |
| dev | ✓ (112s) | ✗ thinking | ✓ (2s) | ✓ | ✓ (when quota) |
| trading | ✗ timeout | ✗ thinking | ✓ (3.5s) | ✓ | ✓ (when quota) |
| token-analysis | ✓ (119s) | ✗ thinking | ✓ (3s) | ✓ | ✓ (when quota) |
| **Effective** | **3/5** | **1/5** | **5/5** | **5/5** | **5/5** |

- **CRITICAL**: qwen35-9b-gpu is broken on 4/5 domains. Only chess works. The `/no_think` flag is NOT suppressing thinking mode — the model generates thousands of thinking tokens that eat the 4096 max_tokens ceiling, truncating before JSON output.
- **Gemma**: Works on 3/5 but latency (89-119s) means any timeout <120s kills it. Trading timed out at exactly 120s. At 7.5 tok/s × ~1000 completion tokens = ~133s expected.
- **qwen-7b-hf**: Only reliable Dog. 5/5 domains, 1.5-3.5s latency, 100% json_valid.

**Q4: Summarizer profile**

- **Backend**: Gemma (port 8080 on cynic-core) via `SovereignSummarizer`
- **System prompt**: "You are a concise session summarizer. Output 2-3 sentences, no preamble." (72 chars / ~18 tok)
- **Max tokens**: 512 completion
- **Input**: Session content (variable, typically 500-2000 chars / ~125-500 tok)
- **Estimated budget**: ~550-1000 total tokens (well within 8192 ctx)
- **Latency concern**: At 7.5 tok/s × 200 completion tok = ~27s. Acceptable for background task.
- **Status**: DEAD at boot, alive after warmup (per TODO). Not a prompt budget issue — operational.

### 7.4 New Truths (from profiling)

| T# | Truth | Confidence | Falsifiable by |
|----|-------|------------|---------------|
| **T10** | qwen35-9b-gpu thinking mode is NOT disabled by `/no_think` or `chat_template_kwargs`. 4/5 domains produce ct=4096 truncated responses with no JSON. The 9B model ignores the disable flag (known build b8422 issue). | 55% | A response from qwen35 with ct<1000 on general/dev/trading domains (would falsify) |
| **T11** | Completion budget varies 5x across Dogs: qwen-hf ~200 tok, gemma ~1000 tok, qwen35 4096+ (thinking). A global 350-tok estimate is wrong. Budget must be per-Dog. | 55% | Re-profiling shows consistent completion sizes across Dogs (would falsify) |
| **T12** | The 4 chars/token heuristic is accurate: measured avg 3.96. Domain-specific variation ±10% (3.6 for structured, 4.4 for prose). Safe for pre-call context checks. | 55% | Measurement on different stimuli shows >20% deviation (would falsify) |
| **T13** | Only 2 of 5 Dogs are reliably producing valid JSON across all domains: qwen-7b-hf and deterministic-dog. Gemma serves 3/5 (latency kills trading). qwen35 serves 1/5. gemini-cli depends on Google quota. Effective quorum for most domains = 3 (det + qwen-hf + gemma-when-fast-enough). | 50% | Fixing qwen35 thinking mode → 5/5 domains work (would falsify the "1/5" claim) |
