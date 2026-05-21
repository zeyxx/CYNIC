---
description: Quick-reference data — φ constants, Dogs, infrastructure
globs: ["**"]
---

## φ Constants

φ=1.618034 | φ⁻¹=0.618034 (max confidence) | φ⁻²=0.382 (decay threshold)
HOWL > 0.528 (φ⁻²+φ⁻⁴) | WAG > 0.382 | GROWL > 0.236 | BARK ≤ 0.236

## Dogs (Independent Validators)

Declared in `~/.config/cynic/backends.toml` — loaded at kernel boot if env vars resolve.

| Dog | Model | Where | Activation |
|---|---|---|---|
| deterministic-dog | Heuristics | In-kernel | Always (no config required) |
| qwen-7b-hf | Qwen 2.5 7B | HF Inference | If HUGGINGFACE_API_KEY set |
| qwen25-7b-core | Qwen 2.5 7B Q4 | cynic-core (CPU + Vulkan) | If <TAILSCALE_CORE> reachable |
| qwen36-27b-gpu | Qwen 3.6 27B IQ3_XXS | cynic-gpu (RTX 4060 Ti) | If <TAILSCALE_GPU> reachable |
| gemini-cli | Gemini (auto) | CLI subprocess | If GEMINI_API_KEY set + gemini-cli on PATH |

## Infrastructure

Source of truth: `~/.config/cynic/fleet.toml`

| Service | Location | What |
|---|---|---|
| CYNIC Kernel | `<TAILSCALE_CORE>`:3030 | REST API |
| llama-server | `<TAILSCALE_CORE>`:8080 | Qwen 2.5 7B (CPU + Vulkan) |
| llama-server | `<TAILSCALE_GPU>`:8080 | Qwen 3.6 27B IQ3_XXS (GPU, 19.7 tok/s) |

Full API contract: `API.md`. Env: `${CYNIC_REST_ADDR}`, `${CYNIC_API_KEY}` from `~/.cynic-env`.
