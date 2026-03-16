# CYNIC Worktrees — Isolated Scope + Coordination Protocol

*Isolation without coordination is incomplete. Register, claim, work, release.*

Use this skill when starting any non-trivial change to CYNIC. One ILC = one worktree = one branch = one set of claims.

## Step 1 — Register intent

Before touching any file, register this session with the kernel:

```
cynic_coord_register(agent_id="<your-agent-id>", intent="<what you're building>")
```

Agent ID is shown in the session banner (`Agent: claude-XXXXXXXXXX`).

## Step 2 — Check active claims

```
cynic_coord_who()
```

Read the output. If another agent holds a claim on a file you need, coordinate before proceeding. Do not override silently.

## Step 3 — Claim your target

For each file you intend to edit:

```
cynic_coord_claim(agent_id="<your-agent-id>", target="<filename>")
```

Expected response: `CLAIMED`. If `CONFLICT`, the file is held by another agent — coordinate or wait.

## Step 4 — Create isolated worktree

```bash
make scope SLUG=<descriptive-name>
# e.g. make scope SLUG=rest-audit
```

This creates:
- Branch: `session/<username>/<slug>`
- Worktree: `../cynic-<slug>`

Work in the worktree directory. Main checkout stays clean.

## Step 5 — Validate and ship

```bash
make check           # build + test + clippy (must pass)
git add <files>
git commit -m "type(scope): description"
git push origin session/<username>/<slug>
```

L0 gates (gitleaks pre-commit, pre-push build+test+clippy) run automatically.

## Step 6 — Release claims

After shipping each file:

```
cynic_coord_release(agent_id="<your-agent-id>", target="<filename>")
```

Release one file at a time as work completes. Do not hold claims after merging.

## Step 7 — Clean up scope

After merging:

```bash
make done SLUG=<name>
```

Removes the worktree and deletes the branch.

## Step 8 — Release session (SessionEnd)

```
cynic_coord_release(agent_id="<your-agent-id>")
```

No target = releases all remaining claims for this agent. Call at end of every session.

---

## Quick Reference

| Stage | Command |
|-------|---------|
| Register | `cynic_coord_register(agent_id, intent)` |
| Check state | `cynic_coord_who()` |
| Claim file | `cynic_coord_claim(agent_id, target)` |
| Create worktree | `make scope SLUG=<name>` |
| Validate | `make check` |
| Release file | `cynic_coord_release(agent_id, target)` |
| Clean up | `make done SLUG=<name>` |
| End session | `cynic_coord_release(agent_id)` |
| View all agents | `make agents` |

## Conflict protocol

If `cynic_coord_claim` returns `CONFLICT`:
1. Run `cynic_coord_who()` to identify the holder
2. Check if that session is still active (5-min TTL auto-expires idle claims)
3. If expired, claim proceeds automatically on next try
4. If active, coordinate: split scope, serialize work, or handoff

## Why this matters

Without coordination, two agents editing the same file produce merge conflicts at push time — caught late. `cynic_coord_claim` catches this before a single line is written.

Git branch uniqueness is hard enforcement. Coord claims are soft enforcement (visibility layer). Both are necessary.
