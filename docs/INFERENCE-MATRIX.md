# CYNIC Inference Matrix — Model Selection & Hardware Allocation

*Living document. Updated from real benchmarks, not estimates.*
*Last update: 2026-03-15*

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
- **Inference:** CUDA GPU primary. Currently 20/33 layers offloaded (underutilized).
- **Available VRAM:** ~15.5 GB (after Windows display driver)

## Benchmarked (real measurements)

### Gemma 3 4B Q4_K_M on Ubuntu (port 8080)

| Config | Prompt t/s | Gen t/s | CPU usage | RAM |
|---|---|---|---|---|
| ngl=0 (CPU only) | 48.3 | 14.0 | ~80-100% | 3.5 GB |
| ngl=5 (Vulkan) | 49.2 | 15.0 | ~40% | 3.5 GB |
| ngl=10 | 51.4 | 15.1 | ~30% | 3.5 GB |
| ngl=15 | 54.9 | 15.2 | ~25% | 3.5 GB |
| **ngl=26 (full Vulkan)** | **57.7** | **15.7** | **20%** | **3.5 GB** |

**Winner: ngl=26.** +12% gen, +19% prompt, CPU mostly free.

### Qwen 3.5 9B Q4_K_M on S. (port 8080)

| Config | Prompt t/s | Gen t/s | VRAM |
|---|---|---|---|
| ngl=20 (current) | 23 | 20 | 5.5 GB |
| **ngl=33 (full, untested)** | **~200 est.** | **~40-60 est.** | **~8-9 GB** |

**TODO: Test full offload on S..**

## Model Candidates (Q4_K_M, sorted by objective)

### For CYNIC Dogs (judgment, 6 axioms, short context)
Needs: good reasoning, follows JSON format, fast enough for real-time demo.

| Model | Params | Q4KM GB | Ubuntu t/s | S. t/s | Context | Notes |
|---|---|---|---|---|---|---|
| **Qwen 3.5 9B** | 9.7B | 5.3 | ~12 (CPU) | ~40-60 (GPU) | 256K native | Best sub-10B. Already on S.. |
| **Gemma 3 4B** | 3.9B | 2.4 | 15.7 (Vulkan) | ~80 (GPU) | 32K | Already on Ubuntu. Good diversity (Google family). |
| Qwen 3.5 4B | 4.7B | 3.0 | ~13 (Vulkan) | ~70 (GPU) | 256K | Better than Gemma quality, same family as 9B. |
| Qwen 3.5 2B | 2.3B | 1.5 | ~25 (Vulkan) | ~100 (GPU) | 256K | Fast but less accurate reasoning. |

### For Coding Assistant (Claude Code inference, long context)
Needs: code understanding, instruction following, 8K+ context.

| Model | Params | Q4KM GB | Ubuntu fit? | S. fit? | Context | Notes |
|---|---|---|---|---|---|---|
| **Qwen 3.5 9B** | 9.7B | 5.3 | YES (32K w/ KV Q8) | YES (32K full GPU) | 256K native | Polyvalent — coding + reasoning. |
| **Qwen 2.5 Coder 14B** | 14.8B | 8.5 | TIGHT (16K max) | YES (32K GPU) | 32K | Coding specialist. Best for code tasks. |
| DeepSeek-Coder-V2-Lite | 15.7B | 9.8 | YES but MoE=slow CPU | NO (VRAM) | 128K | MoE broken on CPU. Skip. |
| Qwen 3.5 27B | 27.8B | 16.0 | TIGHT (4K only) | NO | 256K | Too big for both machines at useful ctx. |

### For General/Chat (anything-agnostic)
Needs: multilingual, instruction following, good reasoning.

| Model | Params | Q4KM GB | Best on | Notes |
|---|---|---|---|---|
| **Qwen 3.5 9B** | 9.7B | 5.3 | Both | The default choice. |
| Qwen 3.5 4B | 4.7B | 3.0 | Ubuntu | When speed > quality. |
| Qwen 3.5 0.8B | 0.8B | 0.6 | Both | Draft model for speculative decoding. |

## Optimal Configurations

### Config A — Current (working, conservative)
```
Ubuntu port 8080:    Gemma 3 4B Q4_K_M, ngl=26 (Vulkan), ctx 4096
S. port 8080: Qwen 3.5 9B Q4_K_M, ngl=20, ctx 8192
```
Pros: Working now, two families (Google + Alibaba).
Cons: S. underutilized, Ubuntu could be smarter.

### Config B — Optimized (recommended next step)
```
Ubuntu port 8080:    Gemma 3 4B Q4_K_M, ngl=26 (Vulkan), ctx 4096
Ubuntu port 8081:    Qwen 3.5 9B Q4_K_M, ngl=0 (CPU), ctx 32K, --cache-type-k q8_0
S. port 8080: Qwen 3.5 9B Q4_K_M, ngl=33 (full GPU), ctx 32K, --cache-type-k q8_0
```
RAM: Gemma 3.5 GB + Qwen 9.2 GB = 12.7 GB on Ubuntu (9 GB free).
Pros: Two models locally, S. full speed, 32K context.
Cons: Qwen 9B on CPU = ~12 t/s (slower than Gemma).

### Config C — Maximum firepower
```
Ubuntu port 8080:    Gemma 3 4B Q4_K_M, ngl=26 (Vulkan), ctx 4096 — fast CYNIC Dog
Ubuntu port 8081:    Qwen 3.5 9B Q4_K_M, ngl=0, ctx 32K — coding/long context
S. port 8080: Qwen 2.5 Coder 14B Q4_K_M, ngl=99 (full GPU), ctx 32K — coding beast
S. port 8081: Qwen 3.5 0.8B Q4_K_M — speculative draft for 14B
```
Pros: Best coding model on GPU, diversity, speculative decoding.
Cons: Different model on S. = more complexity.

## Context Budget

| ctx-size | KV cache (9B, F16) | KV cache (9B, Q8_0) | Practical use |
|---|---|---|---|
| 4096 | ~0.5 GB | ~0.25 GB | Single chess position, short claims |
| 8192 | ~1.0 GB | ~0.5 GB | Full chess game, short documents |
| 16384 | ~2.0 GB | ~1.0 GB | Medium documents, conversation history |
| 32768 | ~3.9 GB | ~2.0 GB | Long documents, extended coding sessions |
| 65536 | ~7.8 GB | ~3.9 GB | Very long context (S. GPU only) |

## API Backends (cloud, paid)

| Backend | Model | Cost/1M tokens | Speed | Family |
|---|---|---|---|---|
| Gemini | gemini-3-flash-preview | ~$0.10 in / $0.40 out | ~7-10s | Google |
| HuggingFace | Llama 3.1 8B :fastest | ~$0.10 in / $0.30 out | ~0.4s | Meta |
| HuggingFace | Qwen 2.5 72B :fastest | ~$0.10 in / $0.30 out | ~1-2s | Alibaba |
| Local Gemma | Gemma 3 4B | $0 (electricity) | 15.7 t/s | Google |
| Local Qwen | Qwen 3.5 9B | $0 (electricity) | 12-40 t/s | Alibaba |

## Sovereignty Score

Current: 4 families (Google, Meta, Alibaba, Heuristic) = SOVEREIGN.
Minimum: 3 families. Below 3 = DEGRADED sovereignty.

## llama.cpp Optimization Flags

```bash
# Ubuntu (Vulkan + CPU hybrid)
--n-gpu-layers 26 --cache-type-k q8_0 --cache-type-v q8_0 --threads 8

# S. (full CUDA)
--n-gpu-layers 99 --cache-type-k q8_0 --cache-type-v q8_0 --threads 8

# Speculative decoding (S.)
--model-draft Qwen3.5-0.8B-Q4_K_M.gguf --draft-max 8
```

## Research Notes (March 2026)
- MoE models broken on CPU llama.cpp (~7 t/s instead of expected 45 t/s)
- ik_llama.cpp fork: 1.9x faster MoE, 2-3x faster CPU prompt — worth testing
- vLLM: better at high concurrency, not worth it for single-user
- Flash attention enabled by default in llama.cpp
- KV Q8_0 saves 50% cache RAM with negligible quality loss
- Speculative decoding: +1.5-2x speed with 0.8B draft model
