# CYNIC — Codex Adapter

> Codex-specific execution notes for the shared protocol in `AGENTS.md`.
> This file is an adapter, not the repo-wide source of truth.

## Scope

Read in this order:

1. `AGENTS.md` — shared protocol
2. this file — how Codex executes that protocol

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
cd <repo>
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
2. `cynic_coord_who`
3. `cynic_list_pending_agent_tasks(kind="cortex")` when looking for human-dispatched structural work

Before editing:

1. `cynic_coord_claim`
2. if conflict: stop and coordinate

At session end:

1. `cynic_observe` for material discoveries, decisions, or blockers
2. `cynic_coord_release`

### Fallback path: MCP unavailable

Use direct kernel REST calls with:

- `POST /coord/register`
- `GET /coord/who`
- `POST /coord/claim`
- `POST /coord/release`
- `POST /observe`
- `GET /agent-tasks`

This fallback is valid. It is better than pretending MCP worked.

## Supply Chain

Codex must treat `infra/registry.json` as the source of truth for infrastructure placement.

- Build runner: node resolved from `lifecycle.kernel_release.build_runner_role`.
- Kernel host: node resolved from `lifecycle.kernel_release.deploy_target_role` and its deploy flag.
- Proxmox is a hypervisor only.
- Cortex hosts are operator/editing surfaces only.
- `make lifecycle-diagnose` is the non-mutating way to see redacted lifecycle readiness.
- `python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers` is the concise machine-readable readiness view; `make lifecycle-blockers` is the operator wrapper.
- `make deploy-request` is the canonical Makefile entrypoint for release build/deploy requests.
- `make gate-2-request` and `make deploy-request` resolve lifecycle intents from `lifecycle.kernel_release.request_profiles`.
- Do not conditionally add or skip release-validation intents from cortex-local service probes.
- Machine clients should use `python3 organs/forge/src/entrypoints/cli.py delegate-profile <profile> --json` for a single parseable JSON object. Treat Makefile request targets as human-facing wrappers.

Canonical release build/deploy is MCP-first after the active kernel passes `make e2e`, including the supply-chain mempool step:

1. dispatch task via `cynic_dispatch_agent_task` or `POST /agent-tasks`
2. list it pending by kind/domain
3. claim it via `cynic_claim_agent_task` or `POST /agent-tasks/{id}/claim`
4. complete it via `cynic_update_agent_task_result` or `POST /agent-tasks/{id}/result`
5. verify it appears in completed tasks

Until that e2e is green on the active kernel, Codex may run local checks, but must label them transitional and must not present local shell build/deploy as the organism's canonical supply chain.

## Codex Limits That Matter

- `apply_patch` does not provide the same hook surface as Claude/Gemini
- sandbox/network settings may prevent transparent access to the kernel
- session automation is weaker than Claude's hook-backed lifecycle

Design implication:

- Codex instructions must explicitly name manual steps and fallback paths
- do not describe Claude-grade automation as if Codex had it

## Stable Expectations

Before touching `crates/cynic-kernel/`:

- verify who is active
- claim the file explicitly
- record material discoveries

Before ending the session:

- record material observations in the kernel
- release all claims

If you cannot do these through MCP, do them through REST.

## What Does Not Belong Here

This file must not contain:

- current sprint state
- cleanup status snapshots
- “next tasks” backlog
- historical dirty-tree inventories

Those belong in:

- the kernel mempool (`/agent-tasks`)
- structured observations (`cynic_observe` / `/observe`)
- domain files only when they are stable protocol, specs, or source code
