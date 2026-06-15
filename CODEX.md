# CYNIC — Codex Adapter Shim

This file exists for discoverability. The shared protocol source of truth is `AGENTS.md`.

Codex must read, in order:

1. `AGENTS.md`
2. `.codex/CODEX-PROTOCOL.md`

Operational reality: Codex coordination is manual unless the `cynic` MCP tools are visible in-session. If MCP is absent, use REST against `${CYNIC_REST_ADDR}` with Bearer auth from `~/.cynic-env`.

Minimum lifecycle:

1. Register: `POST /coord/register`
2. Inspect: `GET /coord/who`
3. Claim every file before editing: `POST /coord/claim` or `/coord/claim-batch`
4. Treat returned conflicts as blockers
5. Release at session end: `POST /coord/release`

Supply-chain rule:

- Read `infra/registry.json` before build/deploy decisions.
- Build target comes from `infra/registry.json:lifecycle.kernel_release.build_runner_role`.
- Kernel deploy target comes from `infra/registry.json:lifecycle.kernel_release.deploy_target_role` plus its deploy flag.
- Use `make lifecycle-diagnose` to inspect redacted lifecycle resolution when registry targets are not active yet.
- Use `python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers` for concise machine-readable blockers; use `make lifecycle-blockers` only as the operator wrapper.
- Do not mark lifecycle nodes `active` while `activation_evidence_gaps` is non-empty.
- Use `make deploy-request` for the canonical registry/MCP deploy request.
- Request contents come from `lifecycle.kernel_release.request_profiles`; do not rebuild intent lists from local probes.
- Dry-run a full request profile with `python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads <profile> <domain> <agent_id>` before dispatch when checking compliance.
- For machine-readable output, call `python3 organs/forge/src/entrypoints/cli.py delegate-profile <profile> --json` directly instead of parsing Makefile output.
- After `make e2e` passes the supply-chain mempool step, release build/deploy requests must go through `cynic_dispatch_agent_task` / `/agent-tasks` for `organ-forge`.
- Before that e2e is green on the active kernel, local build/deploy commands are transitional checks only, not canonical supply-chain execution.

Do not put secrets, tailnet IPs, current sprint notes, or dirty-tree inventories in this file.
