# CYNIC Inference Matrix — Model & Hardware Allocation

*Living document. Updated from real benchmarks, not estimates.*
*Last update: 2026-03-15*

## Purpose

CYNIC is not just a judgment engine. The inference infrastructure serves:
1. **CYNIC Dogs** — epistemic judgment via 6 axioms
2. **Coding agent** — local inference for development tasks
3. **Reasoning** — deep analysis, math, logic
4. **CCM learning** — crystallizing patterns from all inference activity
5. **General chat** — multilingual, instruction following
6. **Sovereignty** — reducing dependency on any single vendor

All models serve multiple purposes. The matrix is organized by **use case**, not by model.

---

## Hardware

### Ubuntu Desktop (T.)
- **CPU:** AMD Ryzen 7 5700G — 8C/16T, 4.7 GHz, Zen 3
- **iGPU:** AMD Radeon Vega 8 — 512 shaders, Vulkan, shared RAM (UMA)
- **RAM:** 28 GB DDR4
- **Inference:** CPU + Vulkan iGPU hybrid. Full Vulkan offload = **20% CPU usage** during inference.
- **Available for models:** ~22 GB (after OS, Claude Code, kernel, SurrealDB)

### S. Desktop
- **CPU:** Intel i5-14400F — 10C/16T
- **GPU:** NVIDIA RTX 4060 Ti — 16 GB VRAM, CUDA 12.4
- **RAM:** 16 GB DDR5
- **Inference:** CUDA GPU primary. Currently 20/33 layers offloaded (**massively underutilized**).
- **Available VRAM:** ~15.5 GB (after Windows display driver)

### Cloud APIs (paid, sovereignty diversity)
- **Gemini 3 Flash** — Google, $20 budget (~185K requests)
- **HuggingFace Providers** — Meta/Alibaba, $20 budget (~118K requests)

---

## Benchmarked (real measurements on our hardware)

### Gemma 3 4B Q4_K_M on Ubuntu

| Config | Prompt t/s | Gen t/s | CPU usage | RAM |
|---|---|---|---|---|
| ngl=0 (CPU only) | 48.3 | 14.0 | ~80-100% | 3.5 GB |
| **ngl=26 (full Vulkan)** | **57.7** | **15.7** | **20%** | **3.5 GB** |

### Qwen 3.5 9B Q4_K_M on S.

| Config | Prompt t/s | Gen t/s | VRAM |
|---|---|---|---|
| ngl=20 (current) | 23 | 20 | 5.5 GB |
| **ngl=33 (full GPU, TODO)** | **~200 est** | **~40-60 est** | **~8-9 GB** |

---

## Use Case Matrix

### UC1 — CYNIC Judgment (Dogs)
*Fast, follows JSON, good reasoning per axiom. Short context (4K) sufficient.*

| Model | Q4 GB | Ubuntu | S. | Strength | Family |
|---|---|---|---|---|---|
| **Gemma 3 4B** ✅ deployed | 2.4 | 15.7 t/s Vulkan | ~80 t/s | Fast Dog, diversity | Google |
| **Qwen3.5-9B** ✅ S. | 5.3 | ~12 t/s CPU | ~40-60 t/s | Best quality sub-10B, thinking mode | Alibaba |
| Phi-4-mini-reasoning | 2.5 | ~14 t/s Vulkan | ~75 t/s | Math axioms, MS diversity | Microsoft |
| Qwen3.5-4B | 3.0 | ~13 t/s Vulkan | ~70 t/s | Better than Gemma, same Qwen family | Alibaba |

**CCM learns:** Axiom scoring patterns per domain. "Sicilienne = Howl" crystallizes after 21 concordant verdicts.

### UC2 — Coding Agent (local Claude Code alternative)
*Code generation, debugging, review. FIM support for autocomplete. 8K-32K context needed.*

| Model | Q4 GB | Ubuntu | S. | HumanEval | FIM | Context |
|---|---|---|---|---|---|---|
| **Qwen2.5-Coder-14B** | 8.5 | TIGHT | YES (32K GPU) | 88.4 | **YES** | 128K |
| **Qwen3.5-9B** ✅ | 5.3 | YES (32K KV Q8) | YES | ~85 est | no | 262K |
| Qwen2.5-Coder-7B | 4.5 | YES | YES | ~80 | **YES** | 128K |
| DeepSeek-Coder-V2-Lite | 9.8 | MoE=slow | NO | ~86 | YES | 128K |

**CCM learns:** Structural couplings ("rest.rs change → check judge.rs"), error→fix maps ("serde duplicate → lenient parse"), deploy sequences.

### UC3 — Deep Reasoning (math, logic, proof, analysis)
*Chain-of-thought, explicit `<think>` blocks. Quality over speed.*

| Model | Q4 GB | AIME | GPQA-D | MATH-500 | S.? | Notes |
|---|---|---|---|---|---|---|
| **DeepSeek-R1-Distill-Qwen-14B** | 9.1 | **69.7** | 59.1 | **93.9** | YES | Best reasoning under 16 GB |
| **Qwen3.5-9B (thinking)** ✅ | 5.3 | -- | **81.7** | -- | YES | Thinking mode = strong CoT |
| Phi-4-reasoning-plus | ~8.0 | ~75 | ~60 | -- | YES | Competes with o3-mini |
| **Phi-4-mini-reasoning** | 2.5 | **57.5** | 52.0 | **94.6** | YES | Tiny math beast |

**CCM learns:** Decision patterns, logical frameworks, proof strategies that recur across problems.

### UC4 — Multilingual (French + English)
*Native French quality, instruction following, long context.*

| Model | Q4 GB | French | English | Context | Fits where? |
|---|---|---|---|---|---|
| **Mistral Small 3.2 24B** | 14.3 | **Excellent** (native) | Excellent | 128K | Ubuntu CPU only |
| Qwen3.5-9B | 5.3 | Good | Excellent | 262K | Both |
| Gemma 3 4B/12B | 2.4/7.0 | Good (140+ langs) | Good | 128K | Both |

**CCM learns:** Bilingual patterns, terminology preferences, translation quality signals.

### UC5 — Speed-Critical (real-time, instant feedback)
*Sub-second response needed. CYNIC fast Dog, autocomplete, quick checks.*

| Model | Q4 GB | Ubuntu t/s | S. t/s | Quality |
|---|---|---|---|---|
| **Qwen3.5-0.8B** | 0.6 | ~50 Vulkan | ~200+ GPU | Limited (draft model) |
| **Qwen3.5-2B** | 1.5 | ~25 Vulkan | ~100 GPU | Acceptable |
| **Gemma 3 4B** ✅ | 2.4 | 15.7 Vulkan | ~80 GPU | Good |

**CCM learns:** Which patterns are simple enough for fast Dogs vs which need full analysis.

### UC6 — CCM Training Pipeline (future)
*Generate training data, fine-tune Dogs, evaluate results.*

| Model | Role | Where | Notes |
|---|---|---|---|
| Qwen3.5-9B | Generate verdicts for dataset | Both | Primary verdict source |
| All Dogs | Multi-model consensus | Both | Consensus = training signal |
| HF Jobs (GPU compute) | Fine-tune small model on crystals | HuggingFace cloud | $20 HF credits |
| Qwen3.5-0.8B | Target for fine-tuning | Both | Small model + crystal wisdom = fast wise Dog |

**CCM learns:** Meta-patterns about its own learning (what crystallizes well, what dissolves, what domains benefit most from persistence).

---

## Model Landscape Winners — March 2026

| Category | Winner | Params | Q4 GB | Key Benchmark | Notes |
|---|---|---|---|---|---|
| **Polyvalent** | Qwen3.5-9B | 9.7B | 5.3 | MMLU-Pro 82.5, GPQA-D 81.7 | Undisputed sub-10B champion |
| **Coding** | Qwen2.5-Coder-14B | 14.8B | 8.5 | HumanEval 88.4 | FIM support, code specialist |
| **Reasoning** | DeepSeek-R1-Distill-14B | 14.8B | 9.1 | AIME 69.7, MATH 93.9 | Explicit CoT chains |
| **Judgment** | Qwen3.5-9B (thinking) | 9.7B | 5.3 | Thinking CoT > specialized judges | Research-confirmed |
| **Multilingual** | Mistral Small 3.2 24B | 24B | 14.3 | Native French | French company |
| **Speed/Quality** | Gemma 3 4B | 3.9B | 2.4 | HumanEval 85.4 | 15.7 t/s Vulkan |
| **Tiny Reasoning** | Phi-4-mini-reasoning | 3.8B | 2.5 | AIME 57.5 | Microsoft family |
| **Draft** | Qwen3.5-0.8B | 0.8B | 0.6 | N/A | Speculative decoding +1.5-2x |

### Key Findings
- **Qwen3.5-9B** is the undisputed champion sub-10B — beats GPT-OSS-120B (13x larger) on GPQA
- **MoE models broken on CPU** — ~7 t/s instead of 45 t/s. GPU-only, none fit 16 GB
- **Llama 4 dead for consumer** — all MoE, minimum 55 GB
- **Thinking models >> specialized judges** — CoT outperforms purpose-built judge models
- **KV Q8_0** saves 50% cache RAM with negligible quality loss
- **Speculative decoding** with 0.8B draft = +1.5-2x speed, free quality

---

## Download Priority

| # | Model | Q4 GB | Primary Use | Secondary Use | Family |
|---|---|---|---|---|---|
| 1 | **Qwen3.5-9B** ✅ | 5.3 | Judgment, coding, general | CCM training source | Alibaba |
| 2 | **Gemma 3 4B** ✅ | 2.4 | Fast Dog, speed-critical | Sovereignty diversity | Google |
| 3 | **DeepSeek-R1-Distill-14B** | 9.1 | Deep reasoning | Complex analysis | DeepSeek |
| 4 | **Qwen2.5-Coder-14B** | 8.5 | Coding agent, FIM | Code review Dog | Alibaba |
| 5 | **Phi-4-mini-reasoning** | 2.5 | Math reasoning | MS family diversity | Microsoft |
| 6 | **Qwen3.5-0.8B** | 0.6 | Speculative draft | CCM fine-tune target | Alibaba |
| 7 | **Mistral Small 3.2 24B** | 14.3 | French specialist | Mistral family | Mistral |

---

## Optimal Configurations

### Config A — Current (working)
```
Ubuntu  :8080  →  Gemma 3 4B,   ngl=26 Vulkan,  ctx 4096,   15.7 t/s
S. :8080  →  Qwen 3.5 9B,  ngl=20 CUDA,    ctx 8192,   20 t/s
Cloud      →  Gemini 3 Flash + HF Llama + HF Qwen 72B
```

### Config B — Optimized (recommended)
```
Ubuntu  :8080  →  Gemma 3 4B,   ngl=26 Vulkan,  ctx 4096        (fast Dog)
Ubuntu  :8081  →  Qwen 3.5 9B,  CPU,  ctx 32K, KV Q8_0          (coding/long ctx)
S. :8080  →  Qwen 3.5 9B,  ngl=99 CUDA,  ctx 32K, KV Q8_0 (quality+speed)
Cloud      →  Gemini 3 + HF Llama + HF Qwen 72B
```

### Config C — Firepower
```
Ubuntu  :8080  →  Gemma 3 4B,        ngl=26 Vulkan,  ctx 4096    (fast Dog)
Ubuntu  :8081  →  Qwen 3.5 9B,       CPU,  ctx 32K               (polyvalent)
S. :8080  →  Qwen2.5-Coder-14B, ngl=99,  ctx 32K           (coding beast)
S. :8081  →  Qwen3.5-0.8B draft                             (speculative +2x)
Cloud      →  Gemini 3 + HF Llama + HF Qwen 72B + HF DeepSeek
```

### Config D — Maximum sovereignty (future, all models downloaded)
```
Ubuntu  :8080  →  Gemma 3 4B         (fast Dog, Google)
Ubuntu  :8081  →  Qwen 3.5 9B        (polyvalent, Alibaba)
Ubuntu  :8082  →  Phi-4-mini-reason.  (math, Microsoft)
S. :8080  →  Qwen2.5-Coder-14B  (coding, Alibaba)
S. :8081  →  DeepSeek-R1-14B     (reasoning, DeepSeek) — swap with Coder
Cloud      →  Gemini 3 + HF Llama + HF Qwen 72B + Mistral (when French needed)
```
7 families, 8 models, zero single point of failure.

---

## Context Budget

| ctx-size | KV cache (9B, F16) | KV cache (9B, Q8_0) | Practical use |
|---|---|---|---|
| 4096 | ~0.5 GB | ~0.25 GB | Chess positions, short claims, CYNIC Dog judgment |
| 8192 | ~1.0 GB | ~0.5 GB | Full chess game, short documents |
| 16384 | ~2.0 GB | ~1.0 GB | Medium documents, conversation history |
| 32768 | ~3.9 GB | ~2.0 GB | Long documents, extended coding sessions |
| 65536 | ~7.8 GB | ~3.9 GB | Very long context (S. GPU only) |

---

## Sovereignty Score

| # families active | Status | Meaning |
|---|---|---|
| 5+ | **SOVEREIGN** | Full independence |
| 3-4 | DEGRADED | Acceptable but watch |
| 1-2 | CRITICAL | Dangerous vendor dependency |

Current: 4 local (Alibaba, Google, Heuristic, HF-Meta) + 2 cloud (Google API, HF API) = **SOVEREIGN**

---

## llama.cpp Optimization Flags

```bash
# Ubuntu (Vulkan + CPU hybrid)
--n-gpu-layers 26 --cache-type-k q8_0 --cache-type-v q8_0 --threads 8

# S. (full CUDA)
--n-gpu-layers 99 --cache-type-k q8_0 --cache-type-v q8_0 --threads 8

# Speculative decoding (S., with draft model)
--model-draft Qwen3.5-0.8B-Q4_K_M.gguf --draft-max 8
```

---

## What CCM Crystallizes Per Use Case

| Use Case | Crystal Type | Example | Trigger |
|---|---|---|---|
| Judgment | Axiom pattern | "Sicilienne = Howl, conf 0.55" | POST /judge |
| Coding | Structural coupling | "rest.rs change → check judge.rs" | Test fail after change |
| Coding | Error→Fix | "serde duplicate → lenient parse" | Build error pattern |
| Reasoning | Decision pattern | "When axioms conflict → geometric mean resolves" | Repeated reasoning |
| Infrastructure | Config wisdom | "Vulkan ngl=26 → 15.7 t/s, 20% CPU" | Benchmark result |
| User | Preference | "Probing reality > abstract plans" | Repeated feedback |

---

## Research Notes (March 2026)
- MoE broken on CPU llama.cpp (~7 t/s vs expected 45)
- ik_llama.cpp fork: 1.9x faster MoE, 2-3x faster CPU prompt
- vLLM: better at high concurrency, not worth for single-user
- Flash attention enabled by default in llama.cpp
- KV Q8_0: 50% cache savings, negligible quality loss
- Speculative decoding: +1.5-2x with 0.8B draft
- Thinking models outperform specialized judge models (research confirmed)
