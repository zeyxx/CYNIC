# docs/ — Historical and non-authoritative material

**Status: quarantined historical corpus. Not runtime authority.**

## Agent rule

When working on the living CYNIC runtime, **do not treat files under `docs/` as law**.

Canonical authority order (see `AGENTS.md` and `contracts/foundation.json`):

1. observed runtime state
2. `contracts/foundation.json`
3. `infra/runtime-authority.json`
4. `infra/registry.json`
5. `Cargo.toml` + canonical Rust crates
6. `infra/agent-exposure.json`

## Why this directory still exists

Many Markdown files here describe earlier organism/product, legal, token, and
topology ideas. Some still say "Status: normative" or reference deleted paths
such as `axioms/` and `organs/`. Those statements are **historical prose**, not
current runtime contracts.

Deleted authorities:

- `axioms/`
- `organs/`
- Python lifecycle gates
- docs-as-runtime-law

## What agents should use instead

| Concern | Living surface |
|---------|----------------|
| Framework identity and layers | `cynic dev framework --json` / `contracts/framework.json` |
| Runtime harness and metrics | `cynic runtime harness|conformance|metrics --json` |
| Operator command grammar | `cynic commands --json` / `cynic dev surface-audit --json` |
| Agent MCP tool projection | `infra/agent-exposure.json` / MCP tool `cynic_mcp_surface` |
| Kernel boundary extraction debt | `cynic dev kernel-audit --json` |

## Safe use of these files

Allowed:

- historical research
- product/legal narrative drafting when the user explicitly asks
- comparison against current Rust surfaces

Forbidden as authority:

- inventing new runtime paths because a doc names them
- recreating `axioms/` or `organs/` forests from prose
- treating token/legal papers as host/node/kernel execution contracts

`make check` and `cynic dev surface-audit --json` enforce that this quarantine
README remains present with the markers above.
