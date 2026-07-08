# Organ Exposure

`organ-exposure` owns the organism's universal capability exposure surface.

It exists because per-agent configuration drift is now a structural problem:
Codex, Claude, Antigravity, Hermes, organ activation helpers, and docs cannot
each maintain their own MCP/tool inventory.

## Role

- Read the organism exposure DNA from `infra/agent-exposure.json`.
- Model MCP servers and tools as domain entities.
- Render agent-specific projections:
  - Codex TOML with per-tool approval policies.
  - Claude `.mcp.json` compatible server map.
  - Antigravity `settings.json` compatible server map.
- Keep agent-local config files as projections, not sources of truth.
- Provide the future home for drift detection and config synchronization tasks.

## Architecture

- `src/core/` contains entities, ports, and projection rules.
- `src/adapters/` reads the exposure declaration from JSON.
- `src/entrypoints/` wires CLI commands to the core service.

## Rule

Root-level scripts must not own exposure logic. They may call this organ during
transition, but the organ is the boundary for rendering, drift detection, and
future mempool-backed synchronization.

Canonical operations:

- `render --agent <agent>` writes a single projection to stdout.
- `validate` checks MCP source/contract/exposure/projection alignment.
- `apply --check` fails when declared projection targets drift.
- `apply` writes every declared projection target from
  `infra/agent-exposure.json:projection_targets`.

JSON MCP clients use stable local aliases when their config format expects short
server names:

- `cynic-coord` -> `cynic`
- `cynic-browser` -> `browser`

The aliases are projection details only. The organism-wide server inventory
remains `infra/agent-exposure.json`.
