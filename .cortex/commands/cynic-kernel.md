---
name: cynic-kernel
description: CYNIC Kernel — Architecture Reference. Use when touching kernel source code.
paths: cynic-kernel/src/**
---

# CYNIC Kernel — Architecture Reference

*Pointers, not copies. Run `cargo doc -p cynic-kernel --open` for live API docs.*

---

## IDENTITY

CYNIC is an **epistemic immune system** — independent AI validators reaching consensus under mathematical doubt. φ-bounded confidence (max 61.8%). Sovereignty is non-negotiable.

**Triple Agnosticism:** Hardware-agnostic. Model-agnostic. Domain-agnostic.

---

## HEXAGONAL ARCHITECTURE

```
DRIVING ADAPTERS (in)               DOMAIN CORE                DRIVEN ADAPTERS (out)
====================               ============               ====================
REST (axum) ─────────┐                                        OpenAiCompatBackend ── HTTP
MCP (rmcp/stdio) ────┼────────▶ Domain types + Port traits    SurrealHttpStorage ── HTTP
                                                               EmbeddingBackend ── HTTP
```

**Dependency Rule:** Adapters depend on Port traits in `domain/`. Domain depends on NOTHING external.

---

## SOURCE MAP (read these — not this skill)

| What | Where | When to read |
|---|---|---|
| Backend port | `domain/inference.rs` | Adding/modifying backend contract |
| Chat port | `domain/chat.rs` | Dog↔LLM communication |
| Storage port | `domain/storage.rs` | Persistence (verdicts, crystals, observations, flush_usage) |
| Coord port | `domain/coord.rs` | Multi-agent coordination |
| Dog trait + domain types | `domain/dog.rs` | Stimulus, AxiomScores, Verdict, QScore |
| CCM | `domain/ccm.rs` | Crystallization logic |
| Usage tracking | `domain/usage.rs` | Token consumption, cost |
| Config (SoT) | `infra/config.rs` | BackendConfig, BackendRemediation, validate_config |
| Circuit breaker | `infra/circuit_breaker.rs` | Health state machine |
| Health loop | `infra/health_loop.rs` | Background probing |
| Judge | `judge.rs` | Orchestrator, hash chain, aggregation |
| REST handlers | `api/rest/*.rs` | Endpoint implementations |
| MCP tools | `api/mcp/mod.rs` | MCP server (9 tools) |
| Boot sequence | `main.rs` | Composition root |

All paths relative to `cynic-kernel/src/`.

---

## KEY INVARIANTS (stable truths — verify against code if in doubt)

1. **Boot:** Ring 0 (probe) → Ring 1 (storage) → Ring 2 (Dogs, health loop, remediation) → Ring 3 (MCP or REST)
2. **Dogs return raw AxiomScores** — the kernel phi-bounds and aggregates
3. **DeterministicDog is always present** — free, no LLM dependency
4. **Circuit breaker:** Closed →(N failures)→ Open →(cooldown)→ HalfOpen →(success)→ Closed
5. **Config SoT:** `backends.toml` — one file per Dog (URL, model, auth, health, remediation)
6. **Health is cached** — /health reads circuit breaker state (O(1)), health loop probes every 30s
7. **HTTP 200 = sovereign, 503 = degraded/critical** — monitoring checks status code, not JSON
8. **`#![deny(dead_code, unused_imports)]`** — compiler enforces Rule #9

---

## PHI CONSTANTS

```
φ = 1.618034   φ⁻¹ = 0.618034 (max confidence)   φ⁻² = 0.382 (decay)
HOWL > 0.528   WAG > 0.382   GROWL > 0.236   BARK ≤ 0.236
Score floor: 0.05 (true zero = parsing failure)
```

---

## CHECKLIST

```
Before coding:
□ Does this touch domain core? → zero external dependencies
□ Does this need a port trait? → if it talks to external systems, yes
□ Run cargo doc to understand existing contracts

During coding:
□ Every driven dependency goes through a port trait
□ Every backend has a circuit breaker
□ #![deny(dead_code)] will catch unwired code

After coding:
□ cargo build + test + clippy clean?
□ API contract tests still pass? (rest_routes.rs)
□ Can I swap the implementation without touching domain code?
□ Does the system boot with this component unavailable?
```
