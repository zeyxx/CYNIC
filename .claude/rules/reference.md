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
| gemini-flash | Gemini 2.5 Flash | Google API |
| qwen-7b-hf | Qwen 2.5 7B | HF Inference |
| gemma-12b-stanislaz | Gemma 3 12B Q4 | S. RTX 4060 Ti |
| gemma-4b-ubuntu | Gemma 3 4B Q4 | Ubuntu CPU |

## Infrastructure

| Service | Location | What |
|---|---|---|
| CYNIC Kernel | `<TAILSCALE_UBUNTU>`:3030 | REST API |
| llama-server | `<TAILSCALE_UBUNTU>`:8080 | Gemma 3 4B (CPU) |
| llama-server | `<TAILSCALE_STANISLAZ>`:8080 | Qwen 3.5 9B (GPU) |

## API Endpoints

All require `Bearer $CYNIC_API_KEY` except `/health`.
Rate limit: 30/min. Full contract: `API.md`.

```
GET  /health, /verdicts, /verdict/{id}, /crystals, /crystal/{id}
GET  /usage, /dogs, /agents
POST /judge, /observe
POST /coord/register, /coord/claim, /coord/release
```
