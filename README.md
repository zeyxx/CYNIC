# CYNIC

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
the active runtime crates with bounded rustc settings.

Useful local commands:

```bash
make run-kernel
make run-node
make run-mcp
make systemd-list
```

`make test` is intentionally not the canonical gate yet: rustc 1.96.1 currently
ICEs while compiling `cynic-kernel` tests in this workspace. Treat that as
toolchain/test-harness debt; use `make check` as the baseline proof until the
test harness is made stable.

`make build` and `make build-release` are not canonical gates yet: rustc 1.96.1
currently SIGSEGVs/ICEs while compiling `cynic-kernel` binaries in this
workspace. Resolve by pinning a non-ICE toolchain or reducing the kernel compile
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
