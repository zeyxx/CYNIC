# CYNIC — Codex Adapter Shim

This file exists for discoverability. The shared protocol source of truth is `AGENTS.md`.

Codex must read, in order:

1. `AGENTS.md`
2. `.codex/CODEX-PROTOCOL.md`

Operational reality: Codex coordination is manual unless the `cynic` MCP tools are visible in-session. If MCP is absent, use REST against `${CYNIC_REST_ADDR}` with Bearer auth from `~/.cynic-env`.

Minimum lifecycle:

1. Register: `POST /coord/register`
2. Inspect: `GET /coord/who`
3. Read startup attention via `cynic_attention` when MCP is available, or inspect `/health` + `/agent-tasks` through REST fallback
4. Claim every file before editing: `POST /coord/claim` or `/coord/claim-batch`
5. Treat returned conflicts as blockers
6. Release at session end: `POST /coord/release`

Supply-chain rule:

- Read `infra/registry.json` before build/deploy decisions.
- Build target comes from `infra/registry.json:lifecycle.kernel_release.build_runner_role`.
- Kernel deploy target comes from `infra/registry.json:lifecycle.kernel_release.deploy_target_role` plus its deploy flag.
- Use `make lifecycle-diagnose` to inspect redacted lifecycle resolution when registry targets are not active yet.
- Use `cynic activate --dry-run` for concise machine-readable blockers plus dry-run activation action summaries; require `activation_schema_version == "activation.v1"` before consuming those summaries; use `python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers` or `make lifecycle-blockers` only as fallback transports.
- Do not mark lifecycle nodes `active` while `activation_evidence_gaps` is non-empty.
- Use `cynic_lifecycle_dispatch(profile="deploy")` for the canonical registry/MCP deploy request when MCP is visible.
- Request contents come from `lifecycle.kernel_release.request_profiles`; do not rebuild intent lists from local probes.
- Dry-run a full request profile with `cynic_lifecycle_payloads(profile="<profile>")` before dispatch when checking compliance. This must not write to the mempool.
- Use `cynic_lifecycle_dispatch(profile="gate-2")`, `cynic_lifecycle_dispatch(profile="mcp-reload")`, or `cynic_lifecycle_dispatch(profile="deploy")` for mutating requests; success must prove read-back visibility in pending or completed agent tasks.
- When completing a mempool task through `cynic_update_agent_task_result`, pass `kind` and `domain` when known; verified success means the exact task id is visible in completed tasks.
- If `make mcp-freshness` reports a missing or stale MCP binary, request `cynic mcp-reload` through the organism build runner; do not present a cortex-local MCP build as canonical validation.
- The MCP wrapper exports `CYNIC_PROJECT_ROOT` after resolving the repo from `CYNIC_PROJECT_ROOT`, its own path, or git, so Codex/Claude/Antigravity/Hermes can share the same launcher contract across checkouts.
- Use `cynic gate-2`, `cynic mcp-reload`, and `cynic deploy` as the native operator transports when MCP is unavailable. Use `make gate-2-request`, `make mcp-reload-request`, `make deploy-request`, or the Python lifecycle CLI only as legacy fallbacks when the `cynic` CLI is unavailable.
- Local build/deploy commands are transitional checks only, not canonical supply-chain execution.

Do not put secrets, tailnet IPs, current sprint notes, or dirty-tree inventories in this file.
