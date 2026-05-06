# MCP Proxy Extraction — Design Spec

**Date:** 2026-05-07
**Status:** Draft
**Scope:** Extract the MCP-to-REST proxy from cynic-kernel into a standalone `cynic-mcp` crate.

---

## Problem

The kernel is a 45K-line monolith. The MCP proxy (`--mcp` mode) is a stateless HTTP forwarder with zero kernel imports, yet it compiles inside the kernel binary — pulling in 31 dependencies and 12 modules it never touches. This violates SoC and blocks federation (MCP is hardwired to one kernel address).

## Decision

Extract `cynic-mcp` as a new workspace crate: library + binary. The proxy becomes a standalone ~600L binary with ~9 dependencies. The kernel drops `rmcp` and the `--mcp` code path.

## Crate Structure

```
cynic-mcp/
  Cargo.toml
  src/
    lib.rs            — re-exports: CynicMcpProxy, types, local_tools
    types.rs          — MCP param structs, McpRateLimit, validate_agent_id
    proxy.rs          — CynicMcpProxy: HTTP forwarding + #[tool] methods
    local_tools.rs    — validate (cargo check) + git (local repo state)
    main.rs           — binary entry: stdout guard → env → proxy → serve(stdio) → shutdown
  build.rs            — injects CYNIC_VERSION from git (same pattern as kernel)
```

### Dependencies

```toml
[dependencies]
rmcp = { version = "1.2", features = ["server", "transport-io", "macros"] }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1"          # JsonSchema derive on param structs (rmcp tool macros require it)
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tokio-util = "0.7"
chrono = "0.4"
```

### Workspace Integration

```toml
# Root Cargo.toml
[workspace]
members = [
    "cynic-kernel",
    "cynic-node",
    "cynic-askesis",
    "cynic-mcp",        # NEW
]
```

## Source Mapping

| Source (kernel) | Destination (cynic-mcp) | Action |
|---|---|---|
| `api/mcp/proxy.rs` (514L) | `src/proxy.rs` | Move, replace `super::` with `crate::types::`, change `tool_router` vis from `pub(super)` to `pub(crate)` |
| `api/mcp/build_tools.rs` (314L) | `src/local_tools.rs` | Move `run_validate` + `run_git` (filesystem tools called by proxy) |
| `api/mcp/mod.rs` param structs (~120L) | `src/types.rs` | Copy (proxy's own contract, not shared) |
| `api/mcp/mod.rs` McpRateLimit (~55L) | `src/types.rs` | Copy |
| `api/mcp/mod.rs` validate_agent_id (~15L) | `src/types.rs` | Copy |
| `main.rs` MCP early exit block (~46L) | `src/main.rs` | Move + simplify (always proxy mode) + stdout guard |
| `cynic-kernel/build.rs` CYNIC_VERSION | `cynic-mcp/build.rs` | Replicate git version injection for `env!("CYNIC_VERSION")` |

## What Changes in the Kernel

1. **Remove** `api/mcp/proxy.rs` (after deprecation period)
2. **Remove** `pub mod proxy;` from `api/mcp/mod.rs` (after deprecation period)
3. **Remove** `rmcp` from `cynic-kernel/Cargo.toml` (after deprecation period — only if `CynicMcp` is also extracted or removed)
4. **Remove** `--mcp` early exit block from `main.rs` (after deprecation period)
5. **Keep** `api/mcp/mod.rs` (`CynicMcp`) — the in-process MCP server for testing/embedded use. Note: `CynicMcp` still needs `rmcp`, so rmcp stays in kernel deps until CynicMcp is also extracted or removed.

## cynic-node Integration (Follow-Up)

Not built in this PR. The library design (`lib.rs` exports) enables cynic-node to depend on `cynic-mcp` as an optional feature in a future PR. Details deferred to that spec.

## Migration Path

| Phase | Action | Timeline |
|---|---|---|
| 1 | Ship `cynic-mcp` binary, test against running kernel | This PR |
| 2 | Update `.claude/settings.json`: `cynic-kernel --mcp` → `cynic-mcp` | This PR |
| 3 | Kernel `--mcp` prints deprecation warning, still works | This PR |
| 4 | Remove `--mcp`, `proxy.rs`, `rmcp` from kernel | +2 weeks |
| 5 | Wire `cynic-mcp` into `cynic-node` as optional feature | Follow-up PR |

## Key Design Decisions

**D1: Duplicate types, no shared crate.** The param structs are the proxy's input contract (~120L of `#[derive(Deserialize)]` structs). A shared `cynic-mcp-types` crate adds coordination overhead for types that should diverge freely between proxy and in-process MCP.

**D2: Local tools stay local.** `validate` runs `cargo check`, `git` reads `.git/` — these are filesystem operations that must execute where the MCP client runs, not on the remote kernel.

**D3: Library + binary.** The proxy logic lives in `lib.rs` so `cynic-node` can depend on it as a library. The standalone binary is a thin `main.rs` wrapper.

**D4: Deprecation window for `--mcp`.** Existing `.claude/settings.json` configs reference `cynic-kernel --mcp`. Breaking them silently is hostile. The kernel keeps `--mcp` temporarily with a stderr warning.

## Success Criteria

- `cynic-mcp` binary starts in <100ms, connects to kernel, all 17 proxy tools work
- Tool list matches current proxy: judge, validate, git, observe, auth, coord (claim/release/who/batch), agent tasks, health, crystals, verdicts, list, audit
- `cargo build` of cynic-mcp pulls ~9 deps (not 31)
- Kernel builds without `rmcp` (after Phase 4)
- `.claude/settings.json` points to `cynic-mcp`, Claude Code works identically
- `cynic-mcp --help` documents env vars: `CYNIC_REST_ADDR`, `CYNIC_API_KEY`

## What This Enables (Federation)

- Any MCP proxy can point to any kernel (`CYNIC_REST_ADDR`)
- Multiple MCP proxies can share one kernel
- cynic-node on a remote machine can serve MCP without the full kernel
- Cross-kernel MCP routing becomes a config change, not a code change

## Risks

- **R1: Tool parity drift.** Proxy and in-process MCP could diverge. Mitigation: integration test that compares tool lists.
- **R2: Deprecation window confusion.** Two ways to start MCP during transition. Mitigation: clear warning message + CLAUDE.md update.
- **R3: Binary distribution.** New binary needs to be built and deployed. Mitigation: same `cargo build` workflow, workspace member.
- **R4: Stdout corruption.** MCP uses stdio for JSON-RPC. Any println! before the proxy starts corrupts the stream. Mitigation: `main.rs` sets a global stdout guard (like kernel's `MCP_MODE` AtomicBool) before any logging init. Tracing writes to stderr only.
- **R5: `cynic_infer` gap.** The in-process `CynicMcp` has a `cynic_infer` tool that the proxy does not forward. This is an existing gap (pre-extraction), not a regression. Document as known limitation.

## Epistemic Status

- Crate structure: **deduced** (proxy.rs has zero crate imports — confirmed by grep)
- Startup improvement: **conjecture** (~50ms estimate, needs measurement)
- Federation benefit: **inferred** (follows from decoupling, not yet tested)
- Type duplication cost: **observed** (~120L, all simple derive structs)
