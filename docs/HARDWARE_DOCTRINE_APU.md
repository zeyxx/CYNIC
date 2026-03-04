# 🤖 CYNIC Hardware Doctrine: The APU Reality (Ryzen 5700G)

*Date: March 4, 2026*
*Lens: Robotics & AI Infra*

## 1. The Empirical Finding
During the E2E Reality Observation (Task 2 to 10), CYNIC suffered from severe latency issues (120,000ms timeouts) when attempting to use `qwen3.5:9b` via Ollama.
The `ComputeHAL` reported:
`HAL: Final fallback to CPU. Cores: 16`

**Conclusion:** Ollama on native Windows with AMD hardware defaults to pure CPU (AVX2) inference. The Vega 8 integrated GPU (iGPU) is completely wasted. This is an unacceptable metabolic inefficiency.

## 2. The Solution: Vulkan over ROCm
While ROCm is the AMD standard for Linux, **Vulkan** is the most stable and performant cross-platform compute API for APUs on Windows.

CYNIC's `ComputeHAL` has been patched to natively recognize and allocate memory for Vulkan architectures, treating up to 50% of system RAM as dynamically shared VRAM.

## 3. Mandatory Setup for Sovereignty (Action Plan)
To achieve sub-100ms time-to-first-token on the Ryzen 5700G:

1. **Stop using Ollama** for heavy models. It is a CPU trap on this specific hardware.
2. **Download `llama-server.exe`** (from the `llama.cpp` releases, specifically the `vulkan` build).
3. **Run models natively** via the Vulkan backend:
   ```powershell
   ./llama-server.exe -m path/to/model.gguf --port 8080 -ngl 99 --backend vulkan
   ```
4. **Configure CYNIC**: CYNIC's `LlamaCppAdapter` will seamlessly connect to this standard OpenAI-compatible API, bypassing the CPU bottleneck entirely.

## 4. Synaptic Plasticity
CYNIC's `ExperienceVault` successfully demonstrated that if a model is too slow (CPU bound), it will automatically downshift to a smaller parameter model (`qwen2.5:3b`) to maintain responsiveness. However, solving the hardware root cause (Vulkan) is the architect's duty.
