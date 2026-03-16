# CYNIC Workflow — Development Process Reference

Use when in doubt about git steps, deploy process, multi-agent coordination, or workflow sequencing.

**Announce at start:** "I'm using cynic-workflow to verify the correct process."

## The Pipeline (always this order)

```
Edit code → make check → git commit → git push → make deploy → make e2e
```

Never skip a stage. Never deploy without a passing `make check`.

## Key Commands

| What | Command |
|------|---------|
| Build + test + lint | `make check` |
| Validated commit | `make commit m="type(scope): description"` |
| Commit + push | `make ship m="..."` |
| Full deploy | `make deploy m="..."` (builds, ships, deploys binary, verifies) |
| System status | `make status` |
| End-to-end test | `make e2e` |
| DB backup | `make backup` |
| Start ILC | `make scope SLUG=<name>` |
| End ILC | `make done SLUG=<name>` |
| Active agents | `make agents` |

## Session Lifecycle

1. START: `cynic_coord_register(agent_id, intent)`
2. BEFORE EDIT: `cynic_coord_who()` → `cynic_coord_claim(agent_id, target-file)`
3. WORK: in worktree (`make scope SLUG=<name>`)
4. VALIDATE: `make check`
5. SHIP: `make commit m="..." + make ship`
6. RELEASE: `cynic_coord_release(agent_id, target-file)`
7. END: `cynic_coord_release(agent_id)`

## ILC (Independent Logical Component)

Unit of work. One branch per ILC: `session/<agent>/<slug>`.
Git rejects duplicate branch names — hard enforcement against collision.
Use `make scope` to create. Use `make done` to clean up.

## Commit Format

```
type(scope): description

Examples:
feat(kernel): add circuit breaker to inference pipeline
fix(ccm): correct decay threshold calculation
docs(workflow): update session protocol
```

## If Something Is Broken

1. `make status` — full system dashboard
2. `systemctl --user status cynic-kernel surrealdb` — service state
3. Check logs: `ts_logs` MCP tool
4. Do NOT force-push. Do NOT skip hooks. Do NOT brute-force.
5. Max 2 fix attempts, then escalate.
