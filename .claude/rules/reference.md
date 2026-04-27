---
description: Quick-reference data — φ constants, Dogs, infrastructure
globs: ["**"]
---

## φ Constants

φ=1.618034 | φ⁻¹=0.618034 (max confidence) | φ⁻²=0.382 (decay threshold)
HOWL > 0.528 (φ⁻²+φ⁻⁴) | WAG > 0.382 | GROWL > 0.236 | BARK ≤ 0.236

## Dogs (Independent Validators)

| Dog | Model | Where |
|---|---|---|
| deterministic-dog | Heuristics | In-kernel |
| qwen-7b-hf | Qwen 2.5 7B | HF Inference |
| qwen35-9b-gpu | Qwen 3.5 9B Q4 | cynic-gpu (RTX 4060 Ti, 131K ctx) |
| qwen-9b-core | Qwen 3.5 9B Q4 | cynic-core (CPU + Vulkan iGPU) |
| gemini-cli | Gemini (auto) | CLI subprocess (Google subscription) |

## Infrastructure

Source of truth: `~/.config/cynic/fleet.toml`

| Service | Location | What |
|---|---|---|
| CYNIC Kernel | `<TAILSCALE_CORE>`:3030 | REST API |
| llama-server | `<TAILSCALE_CORE>`:8080 | Qwen 3.5 9B (CPU + Vulkan) |
| llama-server | `<TAILSCALE_GPU>`:8080 | Qwen 3.5 9B (GPU, 55 tok/s) |

Full API contract: `API.md`. Env: `${CYNIC_REST_ADDR}`, `${CYNIC_API_KEY}` from `~/.cynic-env`.
