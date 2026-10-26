# VCS Evolution — Agent-Native Surface

Date: 2026-10-25
Status: Planned (deferred)
Owner: hermes

## Problem

CYNIC agents still invoke raw `git` for routine operations. The wrapper
surface (`cynic source`, `cynic git`) covers PRs, worktrees, and topology
observation, but not commit, fetch, switch, push, cherry-pick, or read-only
inspection. This forces agents to bypass the rules the wrappers encode
(no `git add -A`, branch naming, worktree isolation, identity).

## Canonicalization

Collapse the two VCS families into one. `cynic git branch/reconcile/hygiene`
migrate under `cynic source` as deprecated aliases for one release cycle,
then removal.

## Tranches

### Tranche 1 — Core mutations (priority)

- `cynic source commit` — stage explicit paths + commit with agent identity.
  No `--all` flag exists; `git add -A` is impossible by construction.
- `cynic source fetch` — fetch origin, update remote refs.
- `cynic source switch <branch>` — refuse if working tree is dirty.

### Tranche 2 — Convergence and cleanup

- `cynic source push github` — push main to the public projection.
- `cynic source worktree remove <branch>` — manual worktree + local branch removal.
- `cynic source branch delete <branch>` — local + remote, guard `agent/*` only.

### Tranche 3 — Advanced

- `cynic source cherry-pick <commit>` — port with build verification.
- `cynic source log/diff/status` — read-only, replace raw `git` calls.

## Rules

- Every new command: observe -> plan -> apply -> read-back proof.
- Every tranche: branch, PR, merge via `cynic source pr merge`, test on builder.
- No agent uses raw `git` after tranche 1 ships.
- `--delete-branch` is the default on `pr merge` (already implemented, PR #384).

## Related

- PR #384: feat(source): delete head branch and local worktree after a PR merge
- PR #381: fix(freebox): declare freebox runtime secret, pair app_id, derive secret filenames from specs
- `docs/plans/c41-access-remediation-plan.md`
