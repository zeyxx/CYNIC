# CYNIC Agent Protocol v1 — Design Spec

> "The kernel doesn't coordinate agents. It makes invalid states unrepresentable."

**Status:** DRAFT
**Author:** T. + Claude Opus 4.6
**Date:** 2026-05-15
**Scope:** v1 — 3-5 concurrent cortex on a single kernel. Designed for protocol compatibility with 10K-kernel federation (not implemented in v1).

---

## Problem Statement

Multi-cortex sessions (Claude Code, Gemini CLI, Codex) share a mutable filesystem and routinely contaminate each other. The current coordination (CLAUDE.md rules MC1-MC5, coord-claim on `cynic-kernel/src/*`) depends on LLM compliance — which fails at ~5% rate, enough to guarantee daily incidents at 3+ concurrent cortex.

**Observed today (2026-05-15):**
- PR #180 squash-merged while a cortex was still pushing commits to its branch
- Dirty files from cortex A inherited by cortex B (crystal-vivant → surrealdb-jwt)
- Cherry-pick required because rebase dropped commits from squash-merged PR
- Stale release artifacts from another cortex broke compilation
- 26 orphaned local branches, 14 with no PR

**Root cause:** The filesystem is shared and mutable. Rules are compliance-based. Coordination gates exist only for `cynic-kernel/src/*`.

---

## Design Principles (from Crystallize-Truth)

| # | Truth | Design consequence |
|---|-------|--------------------|
| T1 | Claim is a signal, not a lock | Mempool claims inform, don't block. Conflict resolution by quality (Dogs), not arrival order. |
| T2 | Mechanical Dogs (CI/tests) are valid for code v1 | No need for LLM code review. `make check` IS the first Dog. |
| T3 | Protocol is scale-invariant, implementation isn't | Design API verbs for 10K. Implement only local SurrealDB + hooks. |
| T4 | Worktree is fork(), necessary but insufficient | Combine: worktree (isolation) + mempool (intention) + CI gate (selection). |
| T5 | Must solve the 3-cortex problem in <1 week | Scope: mempool claim API + auto-worktree + pre-push kernel validation. Nothing more. |
| T6 | Human remains consensus mechanism for v1 | PRs stay human-reviewed. Kernel assists, doesn't decide. |
| T7 | Agent identity = `<cli>-<user>-<branch>` | Derived mechanically from environment. No persistent identity in v1. |
| T8 | Coordination cost < conflict cost | Budget: <10s overhead/session. Conflict cost: ~30min (observed). |

---

## Architecture

### The Protocol — 5 Verbs

Every agent lifecycle follows this sequence. The kernel enforces valid transitions.

```
REGISTER(intent)  →  kernel assigns agent_id, validates clean state
CLAIM(scope)      →  kernel records scope in mempool, returns conflicts if any
WORK              →  agent works in isolated worktree, no kernel interaction needed
PROPOSE           →  agent pushes branch, kernel validates (scope, freshness, CI)
EXIT              →  kernel releases claims, cleans worktree (or on crash/timeout)
```

**State machine:**

```
UNBORN → REGISTERED → CLAIMED → WORKING → PROPOSED → EXITED
                                    ↓          ↓
                                  DEAD    ABANDONED (TTL 2h)
                           (crash/timeout   (PR not merged
                            → auto-cleanup)  → claims released)
```

Invalid transitions are rejected. An agent cannot PROPOSE without having CLAIMED. An agent cannot CLAIM if its workspace has dirty tracked files. These are mechanically enforced — by the wrapper script (Component 1), hooks (Component 2), and pre-push gate (Component 3).

### Component Map

```
┌─────────────────────────────────────────────────────┐
│                   Claude Code Session                │
│                                                      │
│  session-init.sh ──→ REGISTER ──→ CLAIM             │
│  branch-guard.sh ──→ auto-worktree if on main       │
│  coord-claim.sh  ──→ CLAIM (extended to all files)  │
│  session-stop.sh ──→ EXIT                           │
│  pre-push hook   ──→ PROPOSE (validate via kernel)  │
└──────────────┬──────────────────────────────────────┘
               │ HTTP
               ▼
┌──────────────────────────────────────────────────────┐
│                    CYNIC Kernel                       │
│                                                       │
│  POST /coord/register  →  validate clean state        │
│  POST /coord/claim     →  record scope, check overlap │
│  GET  /coord/who       →  list active agents + scopes │
│  POST /coord/propose   →  validate freshness + scope  │
│  POST /coord/release   →  release claims              │
│                                                       │
│  SurrealDB: agent_session, work_claim (extended)       │
│             (status + intent fields added to existing) │
└───────────────────────────────────────────────────────┘
```

---

## V1 Implementation — 3 Components

### Component 1: Auto-Worktree via Wrapper Script

**What:** A wrapper script creates an isolated git worktree BEFORE launching Claude Code. The session runs entirely inside the worktree.

**Why wrapper, not hook:** Claude Code sets `CLAUDE_PROJECT_DIR` at launch time. `session-init.sh` runs AFTER the session starts — it cannot change the working directory. The worktree must exist before Claude Code launches.

**Wrapper script (`cynic-cortex.sh`):**
```bash
#!/bin/bash
# Usage: cynic-cortex.sh [optional-scope-hint]
SESSION_ID=$(head -c4 /dev/urandom | xxd -p)
WORKTREE_DIR="/tmp/cynic-worktrees/$SESSION_ID"
BRANCH="cortex/$SESSION_ID-$(date +%Y-%m-%d)"

# Gate: shared checkout must be clean (dirty = another cortex's uncommitted work)
# "Dirty" = tracked files modified relative to HEAD. Untracked files are ignored
# (they may be legitimate artifacts). This blocks the cortex from inheriting
# another session's uncommitted edits — not from starting entirely.
if [ -n "$(git -C "$CYNIC_ROOT" diff --name-only HEAD)" ]; then
    echo "BLOCKED: shared checkout has uncommitted tracked changes."
    echo "Another cortex left dirty files. Resolve before starting."
    exit 1
fi

git -C "$CYNIC_ROOT" worktree add "$WORKTREE_DIR" -b "$BRANCH" origin/main
cd "$WORKTREE_DIR"
claude  # launches Claude Code with CWD = worktree
```

**Worktree lifecycle:**
- Created by wrapper BEFORE session starts
- Agent works entirely inside it (all tools resolve relative to worktree)
- `session-init.sh` still runs inside the worktree — handles REGISTER, health probe, context injection
- On EXIT (`session-stop.sh`): if changes committed and pushed → delete worktree. If uncommitted → warn, preserve for 24h.
- On crash/timeout: cron cleans worktrees older than 24h with no commits ahead of main.

**Why worktree, not clone:**
- Worktrees share `.git/objects` — no disk/network cost
- `git worktree add` is instant (<100ms)
- Branch is automatically created and isolated
- `git worktree list` shows all active cortex workspaces

**Known limitation (v1):** Gemini CLI and Codex don't use this wrapper. They still operate in the shared checkout. V1 protects Claude Code sessions only. Concurrent Gemini sessions can still contaminate the shared checkout.

### Component 2: Extended Coord Claims

**What:** Extend `/coord/claim` from kernel-src-only to all files. Add intent + status tracking to existing `work_claim` table.

**Current state:** `coord-claim.sh` fires on `Edit|Write` matching `/cynic-kernel/src/**`. Claims stored in `work_claim` table with fields: `agent_id`, `target`, `claim_type`, `active`, `claimed_at`.

**Changes:**

1. **Remove `if` scope restriction** in `settings.json` — claim ALL Edit/Write targets, not just `cynic-kernel/src/*`.

2. **Extend `work_claim` table** (no new table — avoids violating R12 "one value, one source"):
   - Add `status` field: `claimed | working | proposed | merged | abandoned`
   - Add `intent` field: free-text scope description (from user's first message)
   - Add `session_id` field: links claims to the agent's session for batch operations
   - Existing `active` field becomes derived: `active = status IN ['claimed', 'working']`

3. **On REGISTER:** `session-init.sh` records the agent's intent via `/coord/register` (already exists, add `intent` param).

4. **On each CLAIM:** `coord-claim.sh` records the file target. `/coord/who` aggregates all targets per agent to show scope.

5. **PROPOSED state TTL:** When an agent pushes (PROPOSE), claims move to `proposed` status. A TTL of 2h applies — if the PR is not merged within 2h, claims auto-release to `abandoned`. This prevents orphan claims from blocking other agents indefinitely. The existing `expire_stale()` function handles this (extend with status-aware expiry).

**Conflict detection (scope overlap query):**

```sql
LET $my_targets = (SELECT VALUE target FROM work_claim 
  WHERE agent_id = $me AND status IN ['claimed', 'working']);
SELECT agent_id, target FROM work_claim 
  WHERE agent_id != $me 
  AND status IN ['claimed', 'working']
  AND target INSIDE $my_targets;
```

**Conflict response — signal, not lock:**

The current REST handler returns HTTP 409 on conflict, which `coord-claim.sh` treats as a block (exit 2). V1 changes this:
- HTTP 200 with `{"status": "claimed", "conflicts": [...]}` — claim succeeds, conflicts reported
- The hook outputs a WARNING to stderr (visible to the agent) but does NOT block
- The agent (or human) decides whether to proceed
- This implements T1: claim as signal. The cost of ignoring the signal (wasted tokens if rejected at PROPOSE) is the natural incentive.

### Component 3: Pre-Push Kernel Validation (PROPOSE)

**What:** The pre-push hook validates the proposed changes against kernel state before allowing push.

**Checks (all BLOCK, not warn — v1 must solve the problem, not report it):**
1. **Scope match:** Files in the push diff must be a subset of the agent's claimed targets. Unclaimed files → BLOCK with list of unclaimed files. The agent must claim them first (re-run the edit to trigger `coord-claim.sh`, or explicit `curl /coord/claim`).
2. **Freshness:** `git merge-base --is-ancestor origin/main HEAD` must be true. Stale base → BLOCK with "rebase onto origin/main first."
3. **CI gate:** `make check` must pass (already enforced by existing pre-push hook).
4. **PR status:** Check via `gh pr list --head <branch> --state merged`. If merged → BLOCK with "PR already merged, create new branch for new work."

**Implementation — client-side only (no new kernel endpoint in v1):**

The `/coord/propose` endpoint is deferred. The pre-push hook performs all checks client-side using existing tools — this avoids a new kernel endpoint that would need GitHub API access or server-side git operations:

```bash
# After make check passes:

# Check 1: Scope match (query kernel for agent's claims)
DIFF_FILES=$(git diff --name-only origin/main..HEAD)
CLAIMED=$(curl -s -H "Authorization: Bearer $API_KEY" \
  "http://${KERNEL_ADDR}/coord/who?agent_id=$AGENT_ID" \
  | jq -r '.claims[].target')
UNCLAIMED=$(comm -23 <(echo "$DIFF_FILES" | sort) <(echo "$CLAIMED" | sort))
if [ -n "$UNCLAIMED" ]; then
    echo "SCOPE VIOLATION: files not claimed: $UNCLAIMED"
    exit 1
fi

# Check 2: Freshness
if ! git merge-base --is-ancestor origin/main HEAD; then
    echo "STALE BASE: rebase onto origin/main first"
    exit 1
fi

# Check 3: PR not already merged
BRANCH=$(git rev-parse --abbrev-ref HEAD)
MERGED_PR=$(gh pr list --head "$BRANCH" --state merged --json number --jq '.[0].number' 2>/dev/null)
if [ -n "$MERGED_PR" ]; then
    echo "PR #$MERGED_PR already merged. Create new branch for new work."
    exit 1
fi
```

**Why client-side:** Keeps v1 simple. The pre-push hook already runs `make check` (2+ min). Adding 3 quick checks (curl + git merge-base + gh pr) adds <5s. No kernel changes needed for Component 3. The `/coord/propose` endpoint becomes a v2 optimization (server-side validation for federation scenarios where the client can't be trusted).

---

## What V1 Does NOT Include

- **LLM Dogs for code review** — mechanical gates only (T2)
- **Federation** — single kernel only (T5a)
- **Persistent agent identity** — session-derived only (T7)
- **Automatic merge/selection** — human reviews PRs (T6)
- **Punishment/reputation** — no agent scoring in v1
- **Distributed mempool** — local SurrealDB only (T3)

---

## Migration Path

### From current state to v1:

1. `cynic-cortex.sh` — wrapper script: clean check → worktree create → launch Claude Code
2. `settings.json` — remove `if` scope restriction on coord-claim (all files)
3. Kernel — extend `work_claim` table with `status`, `intent`, `session_id` fields
4. Kernel — extend `expire_stale()` with status-aware TTL (PROPOSED → ABANDONED after 2h)
5. `coord-claim.sh` — change conflict from 409-block to 200-with-warning
6. `pre-push` hook — add scope match + freshness + merged-PR checks (client-side)
7. `session-stop.sh` — add worktree cleanup + claim release

### From v1 to v2 (future):

- Dog code review (LLM judgment on PRs)
- Agent reputation (historical quality score)
- Persistent identity (cross-session agent profiles)
- Federated mempool (cross-kernel scope awareness)

---

## Falsification

| Claim | Falsification test |
|-------|--------------------|
| Auto-worktree prevents contamination | Run 3 cortex in parallel; check for zero dirty-file inheritance |
| Mempool claims detect scope overlap | Two agents claim overlapping files; verify conflict returned |
| Pre-push catches stale base | Merge PR A, then push from cortex B without rebase; verify rejection |
| Pre-push catches merged PR | Merge PR, then push new commit to same branch; verify block |
| Coordination overhead <10s/session | Measure total time in hooks across a typical session |
| V1 solves the "this morning" problem | Replay today's scenario; verify zero cross-contamination |

---

## Open Questions

1. **Should claims be per-file or per-module?** Per-file is precise but noisy (100+ claims per session). Per-module is coarse but practical. V1: per-file (hooks already fire per-file), with aggregation in `/coord/who` display.
2. **What happens when the kernel is down?** Current behavior: `CYNIC_COORD_ALLOW_DEGRADED=1` bypasses claims. V1 keeps this but logs degraded sessions for post-hoc audit.
3. **Gemini CLI / Codex in same workspace:** V1 only protects Claude Code sessions (wrapper script). Concurrent Gemini sessions can still dirty the shared checkout. Mitigation: the wrapper's clean-check at launch detects Gemini contamination and blocks until resolved. Full Gemini integration is v2.
4. **Worktree disk usage:** Each worktree is a lightweight checkout (~50MB for CYNIC). 5 concurrent cortex = ~250MB in `/tmp`. Acceptable. Cron cleanup prevents accumulation.

---

*"The organism doesn't prevent mutations. It selects for fitness."*
