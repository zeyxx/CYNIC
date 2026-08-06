# CYNIC Sovereign Inference Operational Benchmark Summary

**Execution Target:** Local AMD Ryzen 7 5700G APU (Vulkan RADV 22.2 GB unified RAM)  
**Methodology:** Native HTTP SSE streaming (`/v1/chat/completions`, `stream: true`, `stream_options.include_usage: true`) with Bearer token authentication  
**Workload Dataset:** `infra/operational-benchmarks.json` (4 real operational CYNIC tasks: Kernel Audit, Refactoring Architecture Plan, JSON Extraction, Fleet CoT Reasoning)

---

## 1. Measured Model Performance Matrix

| Model Profile | Param Size | Quantization | Thinking Mode | Avg TTFT (ms) | Avg Prompt Processing (tok/s) | Avg Token Generation (tok/s) | Operational Suitability |
|---|---|---|---|---|---|---|---|
| **minicpm-1b-q8** | 1B | Q8_0 | **OFF** | **68.0 ms** | **899.9 tok/s** | **39.2 tok/s** | Extremely fast real-time chat & JSON extraction |
| **minicpm-1b-q8** | 1B | Q8_0 | **ON** | 125.8 ms | 446.5 tok/s | 37.0 tok/s | Fast CoT reasoning with negligible overhead |
| **vibethinker-3b-q4** | 3B | Q4_K_M | **OFF** | **103.8 ms** | **607.8 tok/s** | **17.9 tok/s** | Sub-second latency CoT and agent planning |
| **vibethinker-3b-q4** | 3B | Q4_K_M | **ON** | 420.2 ms | 154.6 tok/s | 17.9 tok/s | High quality reasoning with extended CoT |
| **qwen35-2b-q4** | 2B | Q4_K_M | **OFF** | 484.0 ms | 114.7 tok/s | 23.4 tok/s | High speed balanced background tasks |
| **qwen35-2b-q4** | 2B | Q4_K_M | **ON** | 539.2 ms | 98.8 tok/s | 23.2 tok/s | Balanced CoT reasoning |
| **qwythos-9b-q4** | 9B | Q4_K_M | **OFF** | 8 450.2 ms | 10.4 tok/s | 5.2 tok/s | Deep codebase auditing and long context |
| **qwythos-9b-q4** | 9B | Q4_K_M | **ON** | 8 956.8 ms | 10.0 tok/s | 5.2 tok/s | Sovereign 9B multi-step reasoning |
| **bonsai-27b-q1** | 27B | Q1_0 | **OFF** | 3 009.2 ms | 18.6 tok/s | 5.5 tok/s | High parameter density complex reasoning |
| **bonsai-27b-q1** | 27B | Q1_0 | **ON** | 4 917.0 ms | 11.3 tok/s | 5.4 tok/s | Complex CoT architecture synthesis |
| **qwen36-27b-iq2** | 27B | IQ2_M | **OFF** | 18 049.8 ms | 3.1 tok/s | 2.4 tok/s | Ultra-high capacity 27B sovereign fallback |
| **qwen36-27b-iq2** | 27B | IQ2_M | **ON** | 16 239.5 ms | 3.3 tok/s | 2.4 tok/s | Deep CoT architecture synthesis |

---

## 2. Safety Preflight Rejections (GGUF Architecture Boundaries)

Safety preflight in `cynic-inference` correctly blocked unsupported model architectures before running:
- `lfm25-2b-q4` (`lfm2` architecture unsupported by llama-server b9600)
- `shieldstral-3b-q8` (`mistral3` architecture unsupported by llama-server b9600)
- `qwen36-35b-iq2` (`qwen35moe` architecture unsupported by llama-server b9600)

---

## 3. Key Observations & Thinking Impact

1. **Thinking OFF vs Thinking ON**:
   - **TTFT (Time To First Token)**: Turning Thinking OFF cuts TTFT by **~50% to 75%** on fast models (`minicpm-1b-q8`: 125ms -> 68ms, `vibethinker-3b-q4`: 420ms -> 103ms).
   - **Generation Speed (tok/s)**: Token generation speed is identical between Thinking ON and OFF (~39 tok/s for MiniCPM, ~18 tok/s for VibeThinker).
2. **APU Bandwidth Tiers**:
   - Micro models (1B - 3B): **18 to 44 tok/s** generation.
   - Large models (9B - 27B): **2.4 to 5.5 tok/s** generation (constrained by DDR4 system RAM bandwidth).
