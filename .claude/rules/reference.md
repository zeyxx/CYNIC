---
description: Reference data — API, Dogs, axioms, infrastructure
globs: ["**"]
---

## Axioms

| Axiom | Judges |
|---|---|
| FIDELITY | Faithful to truth? Sound principles? |
| PHI | Structurally harmonious? Proportional? |
| VERIFY | Testable? Verifiable or refutable? |
| CULTURE | Honors traditions and patterns? |
| BURN | Efficient? Minimal waste? |
| SOVEREIGNTY | Preserves agency and freedom? |

CYNIC judges SUBSTANCE, not FORM.

## φ Constants

φ=1.618034 | φ⁻¹=0.618034 (max confidence) | φ⁻²=0.382 (decay threshold)
HOWL > 0.528 (φ⁻²+φ⁻⁴) | WAG > 0.382 | GROWL > 0.236 | BARK ≤ 0.236

## Dogs (Independent Validators)

| Dog | Model | Where |
|---|---|---|
| deterministic-dog | Heuristics | In-kernel |
| qwen-7b-hf | Qwen 2.5 7B | HF Inference |
| qwen35-9b-gpu | Qwen 3.5 9B Q4 | cynic-gpu (RTX 4060 Ti, 131K ctx) |
| gemma-4-e4b-core | Gemma 4 E4B Q8 | cynic-core (CPU + Vulkan iGPU) |
| gemini-cli | Gemini (auto) | CLI subprocess (Google subscription) |

## Infrastructure

Source of truth: `~/.config/cynic/fleet.toml`

| Service | Location | What |
|---|---|---|
| CYNIC Kernel | `<TAILSCALE_CORE>`:3030 | REST API |
| llama-server | `<TAILSCALE_CORE>`:8080 | Gemma 3 4B (CPU) |
| llama-server | `<TAILSCALE_GPU>`:8080 | Qwen 3.5 9B (GPU, 55 tok/s) |

## API Endpoints

All require `Bearer $CYNIC_API_KEY` except `/health`, `/live`, `/ready`, `/metrics`, `/events`.
Rate limit: 30/min. Full contract: `API.md`.

```
GET  /health, /live, /ready, /metrics, /events
GET  /verdicts, /verdict/{id}, /crystals, /crystal/{id}
GET  /usage, /dogs, /agents
GET  /observations, /sessions, /session/{agent_id}/compliance, /compliance, /audit
POST /judge, /observe, /crystal
DELETE /crystal/{id}
POST /coord/register, /coord/claim, /coord/claim-batch, /coord/release
POST /dogs/register, /dogs/{id}/heartbeat
DELETE /dogs/{id}
```
