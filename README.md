# CYNIC

[![CI](https://github.com/zeyxx/CYNIC/actions/workflows/ci.yml/badge.svg)](https://github.com/zeyxx/CYNIC/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.95-orange.svg)](https://www.rust-lang.org)

**A low-layer sovereign runtime for multi-agent systems.**

CYNIC coordinates human intent, operating systems, source matter, builds, artifacts, agents, inference, hosts, and fleets without collapsing their authorities into one controller. It proves every mutation with read-back evidence.

CYNIC is being rebuilt around a minimal low-layer Rust runtime baseline.

The current operational core is:

- `crates/cynic-kernel`: coordination bus, observations, health, runtime read-back.
- `crates/cynic-host`: portable host and OS substrate primitives.
- `crates/cynic-node`: node-local runtime supervision and probes.
- `crates/cynic-mcp`: agent-facing MCP projection.
- `crates/cynic-cli`: operator command surface.
- `crates/cynic-protocol`: shared no-I/O protocol types.
- `crates/cynic-askesis`: proof/history support.
- `infra/`: registry, systemd, storage, firewall, and agent exposure projection.
- `contracts/foundation.json`: small constitutional and evolutive policy contract.

Deleted legacy authorities such as `axioms/`, `organs/`, stale docs, Python lifecycle
gates, and UX containers do not define the runtime anymore.

## Validation

```bash
make check
```

`make check` validates the machine-readable baseline and runs `cargo check` on
the active runtime crates with bounded rustc settings, including
`CARGO_INCREMENTAL=0` as the current compiler-stability workaround.

Useful local commands:

```bash
make run-kernel
make run-node
make run-mcp
make systemd-list
make install-cli
make audit-runtime-phases
```

Recommended operator loop after CLI edits:

```bash
make operator-loop
```

Equivalent manual loop:

```bash
make install-cli
make check-installed-runtime
make audit-runtime-phases
make audit-surface-matrix
```

For explicit root refresh without touching the source discovery path:

```bash
make install-cli-system-refresh
```

Or force a custom artifact path directly:

```bash
CYNIC_CLI_INSTALL_SOURCE=target/debug/cynic make install-cli-system-refresh
```

`make install-cli` is idempotent:
- it updates only when the compiled artifact changes,
- it avoids a no-op failure when source and target are already the same file,
- and in non-writable hosts it avoids blind repeated root actions: choose an explicit writable `CLI_USER_TARGET`, or set `CYNIC_CLI_AUTO_SYSTEM=1` to route a controlled root install.

If you must run `cynic bootstrap framework install-cli` manually outside the repo root,
set `CYNIC_STANDALONE_CLI_SOURCE` (or `CYNIC_CLI_ARTIFACT`) to the built binary path and set
`--target` explicitly when you intentionally want to reinstall over the installed artifact path.

```bash
CYNIC_STANDALONE_CLI_SOURCE=target/debug/cynic sudo cynic bootstrap framework install-cli --json --apply
```

Cargo execution is repo-scoped. The workspace uses `scripts/cargo`, a repo-local
`CARGO_HOME`, and explicit source/network modes instead of relying directly on
the host user's `~/.cargo`.

Default mode:

```bash
make check
```

Vendored mode after a one-time online bootstrap:

```bash
./scripts/cargo-vendor-bootstrap
CARGO_SOURCE_MODE=vendor make check
```

When network access is genuinely available and a registry refresh is intended:

```bash
CARGO_NETWORK_MODE=online make local-check
```

`make test` is intentionally not the canonical gate yet: on Thursday, July 16,
2026, `rustc 1.96.0` can still ICE while compiling workspace tests. Treat that
as toolchain/test-harness debt; use `make check` as the baseline proof until
the test harness is made stable.

`make build` and `make build-release` are not canonical gates yet: on Thursday,
July 16, 2026, `rustc 1.96.0` still shows workspace binary-instability paths.
Resolve by pinning a non-ICE toolchain or reducing the kernel/node compile
surface before treating binary build as runtime proof.

## Authority

When files disagree, use this order:

1. Observed runtime state.
2. `contracts/foundation.json`.
3. `infra/runtime-authority.json`.
4. `infra/registry.json`.
5. `Cargo.toml`.
6. Canonical Rust crates.
7. `infra/agent-exposure.json`.

The baseline rule is simple: keep only surfaces that build, deploy, supervise,
secure, expose, or prove the active low-layer organism.
