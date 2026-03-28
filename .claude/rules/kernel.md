---
description: Rust kernel rules — loaded when editing cynic-kernel/
globs: ["cynic-kernel/**"]
---

## Kernel Rules

### Enforced (make lint-rules / compiler)

K1. **Domain purity.** Zero `#[cfg]` in domain code (except `#[cfg(test)]`). — `make lint-rules`
K2. **Every adapter through a port trait.** No raw `reqwest::Client` outside `backends/` and `storage/`. — `make lint-rules`
K3. **No logic duplication across API surfaces.** Pipeline lives in `pipeline.rs`. REST and MCP call it. — `make lint-rules`
K4. **No trait name collisions.** Each port trait name unique across `domain/`. — `make lint-rules`
K5. **No cross-layer type leakage.** App services never import from `api/`. Domain never exposes infra types. — `make lint-rules`

### Design Principles

K6. **Timeout every background `.await`.** Any `.await` inside `tokio::spawn` must use `tokio::time::timeout()`.
K7. **`Display` implies `Error`.** Every Display for error reporting must also impl Error.
K8. **Every SQL query has an integration test.** Round-trip: INSERT + SELECT + assert on shape. — `make check-storage` (when SurrealDB available)
K9. **HTTP status codes are the contract.** Monitoring checks status codes, never parses JSON.
K10. **Agents use the platform.** Agents delegate persistence/judgment/learning to the kernel. No agent-owned DBs.

## Build

- `make check` = build + test + clippy + lint-rules + lint-drift + audit (mandatory after every change)
- `#![deny(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]` in lib.rs
- Workspace lints in root Cargo.toml (Axum/Embark baseline)
- Integration tests: `tests/integration_storage.rs` with `#[ignore]`
