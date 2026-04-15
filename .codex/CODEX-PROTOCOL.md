# CYNIC — Codex Adapter

> Codex-specific execution notes for the shared protocol in `AGENTS.md`.
> This file is an adapter, not the repo-wide source of truth.

## Scope

Read in this order:

1. `AGENTS.md` — shared protocol
2. this file — how Codex executes that protocol
3. `.handoff.md` — current historical context only

Do not read roadmap or journal files as protocol.

## Reality Check

Codex does **not** currently have the same deterministic lifecycle automation as Claude.

What is real today:

- Codex reads `AGENTS.md` when launched in this repo
- Codex loads `.codex/config.toml`
- Codex can use the `cynic-coord` MCP proxy when it is actually available in-session
- Codex can fall back to direct kernel REST calls when MCP is absent or unreliable

What is **not** guaranteed:

- automatic claim hooks on every write
- automatic session-start register/read/who
- automatic session-end handoff/release

Treat Codex as “manual by default, automated when verified”.

## Quick Start

```bash
cd /home/user/Bureau/CYNIC
source ~/.cynic-env
codex
```

Then verify whether MCP is actually live in the session:

- expected: `cynic-coord` is available
- if not available: use direct REST fallback to the kernel

## Codex Session Workflow

### Preferred path: MCP available

At session start:

1. `cynic_coord_register`
2. `cynic_handoff(action="read")`
3. `cynic_coord_who`

Before editing:

1. `cynic_coord_claim`
2. if conflict: stop and coordinate

At session end:

1. `cynic_handoff(action="append", ...)`
2. `cynic_coord_release`

### Fallback path: MCP unavailable

Use direct kernel REST calls with:

- `POST /coord/register`
- `GET /agents`
- `POST /coord/claim`
- `POST /coord/release`
- local `.handoff.md` read/append

This fallback is valid. It is better than pretending MCP worked.

## Codex Limits That Matter

- `apply_patch` does not provide the same hook surface as Claude/Gemini
- sandbox/network settings may prevent transparent access to the kernel
- session automation is weaker than Claude's hook-backed lifecycle

Design implication:

- Codex instructions must explicitly name manual steps and fallback paths
- do not describe Claude-grade automation as if Codex had it

## Stable Expectations

Before touching `cynic-kernel/`:

- verify who is active
- claim the file explicitly
- record material discoveries

Before ending the session:

- append a short handoff
- release all claims

If you cannot do these through MCP, do them through REST.

## What Does Not Belong Here

This file must not contain:

- current sprint state
- cleanup status snapshots
- “next tasks” backlog
- historical dirty-tree inventories

Those belong in:

- `TODO.md` for live work
- archived TODO snapshots for history
- `.handoff.md` for dated session context
