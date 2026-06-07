# CYNIC — Codex Adapter Shim

This file exists for discoverability. The shared protocol source of truth is `AGENTS.md`.

Codex must read, in order:

1. `AGENTS.md`
2. `.codex/CODEX-PROTOCOL.md`
3. `.handoff.md` as dated context only

Operational reality: Codex coordination is manual unless the `cynic` MCP tools are visible in-session. If MCP is absent, use REST against `${CYNIC_REST_ADDR}` with Bearer auth from `~/.cynic-env`.

Minimum lifecycle:

1. Register: `POST /coord/register`
2. Inspect: `GET /coord/who`
3. Claim every file before editing: `POST /coord/claim` or `/coord/claim-batch`
4. Treat returned conflicts as blockers
5. Release at session end: `POST /coord/release`

Do not put secrets, tailnet IPs, current sprint notes, or dirty-tree inventories in this file.
