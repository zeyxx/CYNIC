---
description: Rust kernel rules — loaded when editing cynic-kernel/
globs: ["cynic-kernel/**"]
---

## Kernel Rules

### Enforced (make lint-rules / compiler)

K1. **Domain purity.** Zero `#[cfg]` in domain code (except `#[cfg(test)]`). — `make lint-rules`
K2. **Every adapter through a port trait.** No raw `reqwest::Client` outside `backends/` and `storage/`. — `make lint-rules`
K3. **No logic duplication across API surfaces.** Pipeline lives in `pipeline.rs`. REST and MCP call it. — `make lint-rules` (protected function list: `format_crystal_context`, `compute_qscore`, `trimmed_mean`)
K4. **No trait name collisions.** Each port trait name unique across `domain/`. — `make lint-rules`
K5. **No cross-layer type leakage.** App services never import from `api/`. Domain never exposes infra types. — `make lint-rules` (checks `serde_json::Value`, `reqwest::`, `surrealdb::`, `axum::` in domain/)

### Design Principles

K6. **Timeout every background `.await`.** Any `.await` inside `tokio::spawn` must use `tokio::time::timeout()`.
K7. **`Display` implies `Error`.** Every Display for error reporting must also impl Error.
K8. **Every SQL query has an integration test.** Round-trip: INSERT + SELECT + assert on shape. — `make check-storage` (when SurrealDB available)
K9. **HTTP status codes are the contract.** Monitoring checks status codes, never parses JSON.
K10. **Agents use the platform.** Agents delegate persistence/judgment/learning to the kernel. No agent-owned DBs.

### LLM Development Principles

The codebase is the prompt. Every pattern in code will be replicated by future LLM sessions. Bad patterns spread exponentially — each copy reinforces the signal.

K11. **Extract at 2, not 3.** LLMs replicate patterns before humans notice duplication. Extract into a function/method at the 2nd occurrence, not the 3rd. — `make lint-rules` (shares K3 protected function list; add new entries when duplication is found)
K12. **`#[allow]` is an instruction.** Every lint suppression tells the next LLM "do this." Require adjacent `// WHY:` comment explaining the suppression. Suppress without justification = amplified debt. — `make lint-rules`
K13. **Shared logic across API surfaces = one function.** REST and MCP must call the same computation (extends K3). If a health check, status computation, or data extraction appears in both → extract to domain or shared module. Never duplicate across api/rest/ and api/mcp/.
K14. **Poison/missing = assume degraded.** When reading shared state (`RwLock`, `Option`), the fallback on error must be the SAFE default (degraded/unavailable), never the OPTIMISTIC default (ok/sovereign). `unwrap_or(true)` for degradation checks, never `unwrap_or(false)`.

## Build

- `make check` = build + test + clippy + lint-rules + lint-drift + audit (mandatory after every change)
- `make test-gates` = R21 gate falsification (inject violation → verify gate catches → restore)
- `#![deny(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]` via workspace lints
- `#![cfg_attr(test, allow(dead_code, clippy::unwrap_used, clippy::expect_used))]` in `lib.rs` — test code is exempt. Do NOT add per-function `#[allow(clippy::expect_used)]` in `#[cfg(test)]` modules; the crate-level exemption already covers them.
- Workspace lints in root Cargo.toml (Axum/Embark baseline)
- Integration tests: `tests/integration_storage.rs` with `#[ignore]`

K15. **Every producer has a consumer that ACTS.** Storing, displaying, or logging is not consuming. A consumer must change system behavior: gate a request, trigger a state transition, emit an alert that is routed to a human. `store_*` without an acting reader = Rule 3 violation. `emit_event` without an acting handler = dead nervous system. — `make lint-drift` (producer-consumer audit)
