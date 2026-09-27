# infra/ — Runtime Infrastructure

Canonical placement, authority, and projection files for the CYNIC low-layer runtime.

## Authority chain (8 levels, canonical)

1. Observed runtime state
2. `contracts/foundation.json`
3. `infra/runtime-authority.json`
4. `infra/directory-tree.json`
5. `infra/registry.json`
6. `Cargo.toml` workspace
7. Canonical Rust crates under `crates/`
8. `infra/agent-exposure.json`

## Keep-list (deploy, secure, configure, or inspect the active runtime)

- `registry.json` — node topology, transport URIs, credential references, lifecycle. Placement SSOT.
- `runtime-authority.json` — operational authority map: surfaces, installed state, service projections.
- `directory-tree.json` — canonical filesystem arborescence. Path SSOT.
- `agent-exposure.json` — agent-facing MCP projection surface (109 tools, 17 domains).
- `agent-roles.json` — RBAC role definitions.
- `agent-source-protocol.json` — source governance: remotes, commit convention, branch policy.
- `surface-registry.json` — which crate owns which CLI surface, commands, entrypoints.
- `runtime-basket.json` — canonical runtime artifact catalog.
- `storage-taxonomy.json` — data classification K1-K8 with storage homes and retention.
- `systemd/` — installed service units and operator projections.
- `surrealdb/`, `compose/`, `firewall/`, `sudoers.d/`, `wireguard/`, `network/` — provider projections.
- `repo-access-profile.json`, `repo-remotes.json` — repository access governance.
- `RUST_COMPILER_ALPHAS.md` — compiler and build policy.
- `CONTEXT_PLACEMENT.md` — context file placement rules (SOUL/AGENTS/MEMORY/USER).

## Architecture & taxonomy SSOT

- `CANONICAL_CRATE_MAP.md` — 47-crate taxonomy, LOC census, extraction verdicts (v3, 2026-09-28).
- `ARCHITECTURE_CONSTELLATIONS.md` — architectural invariants, boundary rules, extraction slices.
- `GOVERNANCE_MAP.md` — governance surface map.

## Audits (read-only evidence)

- `CLI_SURFACE_AUDIT.md` — CLI surface audit (62 orphan source files).
- `INFERENCE_SURFACE_AUDIT.md` — inference surface audit (91 unreachable commands).
- `WORKSPACE_ORPHAN_DECISIONS.md` — per-crate orphan decision matrix (390 production orphans).
- `SPOF_AUDIT.md`, `ACCOUNTABILITY_AUDIT.md` — resilience and accountability evidence.

Retention rule: keep files that deploy, secure, configure, or inspect the active low-layer runtime. Delete committed runtime snapshots, UX containers, organ remnants, and provider experiments unless required by registry or systemd.
