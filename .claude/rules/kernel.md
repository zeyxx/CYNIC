---
description: Rust kernel rules — loaded when editing cynic-kernel/
globs: ["cynic-kernel/**"]
---

## Kernel-Specific Rules

4. **Domain purity.** Zero `#[cfg]` in domain code.
10. **Timeout every background `.await`.** Any `.await` inside `tokio::spawn` must use `tokio::time::timeout()`.
11. **`Display` implies `Error`.** Every Display for error reporting must also impl Error.
17. **Every adapter through a port trait.** No raw `reqwest::Client` outside `backends/` and `storage/`. Enforced by `make lint-rules`.
18. **Every SQL query has an integration test.** Round-trip: INSERT + SELECT + assert on shape.
19. **No logic duplication across API surfaces.** Pipeline lives in `pipeline.rs`. REST and MCP call it. Enforced by `make lint-rules`.
22. **No trait name collisions.** Each port trait name unique across `domain/`. Enforced by `make lint-rules`.
32. **No cross-layer type leakage.** App services never import from `api/`. Domain never exposes infra types. Enforced by `make lint-rules`.

## Build

- `make check` = build + test + clippy --release (mandatory after every change)
- `#![deny(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]` in lib.rs
- Workspace lints in root Cargo.toml (Axum/Embark baseline)
- Integration tests: `tests/integration_storage.rs` with `#[ignore]`
