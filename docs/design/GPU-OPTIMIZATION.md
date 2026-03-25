# GPU Optimization — Triton, CUDA Kernels, and Reality

*Crystallized 2026-03-15. φ-bounded confidence.*

## The Answer: Fix Configuration First, Custom Kernels Never (For Now)

## Truth Statements

| T# | Truth | Confidence | Impact |
|----|-------|------------|--------|
| T9 | **Fix ngl=20 → ngl=99 on S..** Free 2.4x gen speedup (20→48 t/s), 7.4x prompt speedup (23→170 t/s). 1-line config change. | 58% | **DONE** (applied 2026-03-15) |
| T10 | **Do not write custom Triton/CUDA kernels.** llama.cpp already implements Flash Attention and quantized KV cache for RTX 4060 Ti. Custom kernels would replicate existing optimizations. | 55% | Zero code. |
| T11 | **Triton becomes relevant only when** CYNIC fine-tunes a custom model on crystallized verdicts AND training time on RTX 4060 Ti exceeds acceptable bounds. Months away. | 48% | Defer. Record as CCM future consideration. |
| T12 | **Verdict latency bottleneck is the slowest Dog** (CPU Gemma at 14.4 t/s), not the GPU Dog. Optimizing GPU changes nothing about end-to-end verdict time in parallel execution. | 52% | Architectural awareness, not code change. |

## What We Did (measured, not estimated)

### S. RTX 4060 Ti — Before vs After

| Metric | ngl=20 (before) | ngl=99 + ctx 32K + KV Q8 (after) | Gain |
|---|---|---|---|
| VRAM used | 5.5 GB / 16 GB | **11.3 GB** / 16 GB | Finally utilized |
| Gen speed | 20 t/s | **48 t/s** | **2.4x** |
| Prompt speed | 23 t/s | **170 t/s** | **7.4x** |
| Context | 8K | **32K** | **4x** |
| Short query time | ~26s | **0.5s** | **52x** |

### Ubuntu Vega 8 iGPU — Benchmarked

| Model | CPU-only | Vulkan full | Gain |
|---|---|---|---|
| Gemma 4B gen | 14.4 t/s | 15.7 t/s | +9% |
| Qwen 9B gen | 6.0 t/s | **11.3 t/s** | **+88%** |
| CPU usage during inference | ~80% | **~20%** | CPU freed for other work |

**Key finding:** Vulkan benefits larger models more (+88% for 9B vs +9% for 4B).

## Why NOT Custom Kernels

1. **llama.cpp already does it.** Flash Attention, KV quantization, CUDA-optimized matmul — all built-in, maintained by 100+ contributors.

2. **The free 10x was sitting untouched.** ngl=20 → ngl=99 was a config oversight, not a kernel limitation.

3. **Triton adds a dependency.** OpenAI's Triton compiler = another vendor. llama.cpp's native CUDA path is more sovereign.

4. **The bottleneck is elsewhere.** With Dogs in parallel, verdict latency = slowest Dog = CPU Gemma (14.4 t/s). GPU is not on the critical path.

5. **ROI is terrible.** Weeks of kernel engineering for ~20% on top of already-fast GPU inference. That time builds more CYNIC features instead.

## When Triton/CUDA Makes Sense (Future)

```
IF:
  - CCM has accumulated enough crystals for a training dataset
  - AND we fine-tune a custom Dog on crystallized verdicts
  - AND training time on RTX 4060 Ti exceeds 1 hour per run
  - AND profiling shows a specific kernel bottleneck
THEN:
  - Consider HF CUDA Kernels skill for custom training kernels
  - Consider Triton for fused forward pass of fine-tuned architecture
```

This is months away. Record it, don't build it.

## Remaining GPU Optimization Opportunities

1. **Speculative decoding** — Qwen3.5-0.8B as draft model for the 9B. +1.5-2x speed. Requires downloading the 0.8B (0.6 GB) and adding `--model-draft` flag.

2. **Q5_K_M or Q6_K quantization** — Better quality, S. has VRAM headroom (4.7 GB free). Download higher-quality quant and swap.

3. **ik_llama.cpp fork** — 1.9x faster MoE inference, 2-3x faster CPU prompt. Worth testing on Ubuntu for the Qwen 9B CPU path if/when we add a CPU-only config.
