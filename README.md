# CYNIC

A low-layer sovereign runtime for multi-agent systems.

CYNIC coordinates human intent, operating systems, source matter, builds, artifacts, agents, inference, hosts, and fleets without collapsing their authorities into one controller. Every mutation needs read-back proof.

The operational core is:

- `crates/cynic-kernel`: coordination bus, observations, health, runtime read-back.
- `crates/cynic-host`: portable host and OS substrate primitives.
- `crates/cynic-node`: node-local runtime supervision and probes.
- `crates/cynic-mcp`: agent-facing MCP projection.
- `crates/cynic-cli`: operator command surface.
- `crates/cynic-protocol`: shared no-I/O protocol types.
- `crates/cynic-askesis`: proof/history support.
- `infra/`: registry, systemd, storage, firewall, and agent exposure projection.
- `contracts/foundation.json`: constitutional and evolutive policy contract.

## Validation

```bash
make check
```

`make check` is the canonical gate (admitted build runner only). Machine-readable status: `cynic dev check --json`, `cynic dev framework --json`. Compiler and build policy: `infra/RUST_COMPILER_ALPHAS.md`.

Agent entrypoint and full protocol: `AGENTS.md`.

## Authority

When files disagree:

1. Observed runtime state.
2. `contracts/foundation.json`.
3. `infra/runtime-authority.json`.
4. `infra/registry.json`.
5. `Cargo.toml`.
6. Canonical Rust crates.
7. `infra/agent-exposure.json`.

Keep only surfaces that build, deploy, supervise, secure, expose, or prove the active low-layer organism.
