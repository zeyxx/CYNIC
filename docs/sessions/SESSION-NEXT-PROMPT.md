<!-- lifecycle: living -->
# Next Session — v0.8 "Fondation Prouvée" (continued)

*Written 2026-03-27 post-roadmap crystallization. Commit b307b6c on main.*

## What Was Done (2026-03-27 evening session)

1. **Contract tests**: 5 nouveaux (state transitions, read-side gate, sanitization) → 292 tests total
2. **Roadmap v0.8→v1.0**: spec reviewée (SRE + AI infra), VERSION.md aligné, 3 gates/version
3. **Gate validation framework**: 3-tier testing (push/nightly/E2E), 4 signaux, antifragilité
4. **Deploy**: v0.8 security + contract tests déployés en production, crystal loop activé (chess+trading)
5. **InMemory StoragePort adapter**: écrit, compile, clippy clean — pas encore testé par contract tests
6. **8 gaps opérationnels**: diagnostiqués et documentés
7. **Self-observability**: identifié comme le gap le plus profond — l'organisme ne se connaît pas

## Current State

- **Cargo.toml**: 0.7.3 | **HEAD**: b307b6c (18 commits ahead of v0.7.3)
- **Tests**: 292 (8 contract tests on SurrealDB, 0 on InMemory yet)
- **Findings**: 43 fixed / 9 partial / 38 open (of 90)
- **Production**: running, crystal loop ON (chess+trading), 5 Dogs, 50 crystals (43 workflow noise, 6 chess, 1 stress)
- **Dog naming lies**: qwen3-4b-ubuntu runs gemma-3-4b. llama-8b-hf runs Qwen2.5-7B. Not fixed — needs migration plan.

## v0.8 Gates

| Gate | Status | Remaining |
|------|--------|-----------|
| G1: Security closure | In progress | RC1-1 decision (accept-by-design?). Audit remaining CRIT/HIGH. |
| G2: StoragePort agnostic | **In progress** | InMemory adapter exists. Wire contract tests for both adapters. Add 4 more to reach ≥12. |
| G3: Workflow alignment | Partial | Version bump. State dumps deleted. Doc lifecycle tags + make lint-docs still needed. |

## Priority 1 — Wire Contract Tests on InMemory (compound: proves agnosticism)

The InMemory adapter (`storage/memory.rs`) implements StoragePort with the same invariants as SurrealDB. The proof: **same contract tests must pass on both adapters**.

### How to wire

Current tests in `tests/integration_storage.rs` use `common::setup_test_db()` which creates a SurrealDB namespace. Create a parallel test module (or parameterize) that runs the same `contract_*` tests against `InMemoryStorage::new()`.

The 8 existing contract tests:
1. `contract_observe_crystal_rejects_below_quorum`
2. `contract_observe_crystal_content_set_once`
3. `contract_verdict_voter_count_roundtrip`
4. `contract_crystal_forming_to_crystallized_at_21_obs`
5. `contract_crystal_high_obs_low_confidence_decays`
6. `contract_crystal_forming_stays_forming_at_high_confidence`
7. `contract_list_crystals_for_domain_excludes_forming`
8. `contract_observe_crystal_sanitizes_directives`

### 4 more needed for ≥12

9. `contract_crystal_crystallized_to_canonical_at_233_obs` — Canonical threshold
10. `contract_observe_crystal_running_mean` — verify running mean math
11. `contract_delete_crystal_idempotent` — delete non-existent = no error
12. `contract_list_crystals_sorted_by_maturity` — Canonical > Crystallized > Forming

### Pattern

```rust
// Option A: macro that generates tests for both adapters
macro_rules! contract_test {
    ($name:ident, $body:expr) => {
        mod $name {
            #[tokio::test]
            async fn surreal() { /* setup_test_db + body */ }
            #[tokio::test]
            async fn memory() { /* InMemoryStorage::new() + body */ }
        }
    };
}

// Option B: separate test file for InMemory (simpler, less magic)
// tests/contract_memory.rs — imports InMemoryStorage, runs same assertions
```

Choose based on what feels right. Option B is simpler.

## Priority 2 — Self-Observability Thread

Every action should improve the organism's self-knowledge. Weave through all work:

| What | Why | Effort |
|------|-----|--------|
| `/metrics` endpoint (Prometheus format) | 4 golden signals: latency p50/p95/p99, traffic, errors, saturation | Medium |
| Event bus 2 consumers | CCM on VerdictIssued, health on DogFailed — nervous system | Medium |
| Dog naming truth | backends.toml names must match models. Migration plan needed (12 historical IDs in DB). | Small (plan) |
| Crystal drift detection | Confidence trajectories over time. Alert on unexpected decay. | Medium |

## Priority 3 — RC1-1 Decision

MCP has zero auth (RC1-1 PARTIAL). Decide:
- **Accept-by-design**: stdio transport = process trust. Document rationale in tracker. Close as "Accepted."
- **Implement auth**: add bearer token validation to MCP. Parity with REST.

This unblocks v0.8 G1 (Security closure) and eventually v0.9 G3 (cynic_learn gated on RC1 full closure).

## Session Protocol

```
BEFORE any work:
  1. /status — verify kernel running, Dogs healthy, crystal loop ON
  2. cargo test --quiet — verify baseline (292+ tests)
  3. Read this document

DURING work:
  4. Rule 36: bugs before abstractions
  5. Rule 24: names match reality
  6. Every action improves self-observability
  7. /build after ANY kernel code change

BEFORE claiming done:
  8. Only claim what's PROVEN by tests
  9. Update tracker with HONEST status

AT session end:
  10. /cynic-skills:distill
  11. git status --short must show 0 modified files
```

## Key Insight (carry forward)

**The organism doesn't know itself.** No /metrics, event bus dead, Dog names lie. Self-observability is the tissue connecting all organs — not a feature to add later. Every v0.8 gate should ALSO improve the organism's ability to report its own state.

## References

- Roadmap spec: `docs/superpowers/specs/2026-03-27-roadmap-v1-design.md`
- VERSION.md: aligned with reality
- Session memory: `memory/project_session_2026_03_27_roadmap.md`
- InMemory adapter: `cynic-kernel/src/storage/memory.rs`
- Self-observability feedback: `memory/feedback_self_observability.md`
- Gate validation research: Lighthouse (3-tier), Autoresearch (single scalar), npj Complexity 2024 (antifragility)
