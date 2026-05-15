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
                                    ↓
                                  DEAD (crash/timeout → auto-cleanup)
```

Invalid transitions are rejected. An agent cannot PROPOSE without having CLAIMED. An agent cannot CLAIM if its workspace is dirty. These are kernel-enforced, not LLM-requested.

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
│  SurrealDB: agent_session, work_claim (existing)      │
│             mempool_item (new: scope as hyperedge)     │
└───────────────────────────────────────────────────────┘
```

---

## V1 Implementation — 3 Components

### Component 1: Auto-Worktree on Session Start

**What:** `session-init.sh` creates an isolated git worktree for each session instead of working in the shared checkout.

**How:**
1. Session starts on `main` (or any branch)
2. `session-init.sh` detects dirty tree → **BLOCK** (don't clean it — that's another cortex's work)
3. If clean: create worktree at `/tmp/cynic-worktrees/<session-id>/` with a new branch
4. Set `CLAUDE_PROJECT_DIR` (or equivalent) to the worktree path
5. All subsequent tool calls operate in the worktree, not the shared checkout

**Worktree lifecycle:**
- Created at REGISTER
- Agent works entirely inside it
- On EXIT: if changes committed and pushed → delete worktree. If uncommitted → warn, preserve for 24h, then auto-delete.
- On crash: TTL-based cleanup (session-stop or cron)

**Why worktree, not clone:**
- Worktrees share `.git/objects` — no disk/network cost
- `git worktree add` is instant
- Branch is automatically created and isolated

**Edge case — shared checkout for read-only:**
The shared `main` checkout remains for tools that only READ (Grep, Glob, Read). Write operations (Edit, Write) are redirected to the worktree. This prevents "I need to read the whole project to understand context" from requiring a full copy.

**Constraint:** Claude Code's `CLAUDE_PROJECT_DIR` is set at session start. Changing the working directory mid-session requires the hook to output a `cwd` directive. If Claude Code doesn't support this, the worktree must be created BEFORE the session starts (wrapper script).

### Component 2: Mempool Claim API (Extended Coord)

**What:** Extend `/coord/claim` from kernel-src-only to all files. Add scope-as-hyperedge storage.

**Current state:** `coord-claim.sh` fires on `Edit|Write` matching `/cynic-kernel/src/**`. Claims stored in `work_claim` table.

**Changes:**
1. Remove the `if` scope restriction in `settings.json` — claim ALL Edit/Write targets
2. Add `mempool_item` table in SurrealDB for scope tracking:

```sql
DEFINE TABLE IF NOT EXISTS mempool_item;
-- Fields: agent_id, intent (text), targets (array of file paths),
--         status (claimed|working|proposed|merged|abandoned),
--         created_at, updated_at
```

3. On REGISTER: agent declares intent (from user's first message, via `observe-prompt.sh`)
4. On first CLAIM: kernel creates `mempool_item` with the agent's targets
5. Subsequent CLAIMs add targets to the same `mempool_item`
6. `/coord/who` returns active agents WITH their scope (list of claimed targets)

**Conflict detection (hyperedge intersection):**

```sql
LET $my_targets = (SELECT VALUE targets FROM mempool_item WHERE agent_id = $me AND status = 'working');
SELECT agent_id, targets FROM mempool_item 
  WHERE agent_id != $me 
  AND status IN ['claimed', 'working']
  AND targets ANYINSIDE $my_targets;
```

**Conflict response:** Return the conflict info to the agent. The agent (or human) decides whether to proceed. The claim is NOT blocked — it's flagged. This is T1: claim as signal, not lock.

### Component 3: Pre-Push Kernel Validation (PROPOSE)

**What:** The pre-push hook validates the proposed changes against kernel state before allowing push.

**Checks:**
1. **Scope match:** Files in the push diff must be a subset of the agent's claimed targets. Unclaimed files = scope violation → WARN (not block in v1, block in v2).
2. **Freshness:** The branch must be rebased on current `origin/main`. Stale base = rejection.
3. **CI gate:** `make check` must pass (already enforced by pre-push hook).
4. **PR status:** If a PR exists for this branch and is already merged → BLOCK push (prevents the orphan commit problem from today).

**Implementation:** Add kernel call to existing `scripts/git-hooks/pre-push`:

```bash
# After make check passes:
DIFF_FILES=$(git diff --name-only origin/main..HEAD)
HTTP_CODE=$(curl -s -o /tmp/propose.json -w '%{http_code}' \
  -X POST "http://${KERNEL_ADDR}/coord/propose" \
  -H "Authorization: Bearer $API_KEY" \
  -d "{\"agent_id\":\"$AGENT_ID\",\"files\":$DIFF_FILES_JSON,\"base_sha\":\"$BASE_SHA\"}")
# 200 = proceed, 409 = scope conflict, 412 = stale base
```

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

1. `session-init.sh` — add worktree creation (or wrapper script if CWD can't change)
2. `settings.json` — remove `if` scope restriction on coord-claim
3. Kernel — add `mempool_item` table to bootstrap schema
4. Kernel — add `POST /coord/propose` endpoint
5. `pre-push` hook — add kernel validation call
6. `session-stop.sh` — add worktree cleanup

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

1. **Can Claude Code change CWD mid-session?** If not, the worktree must be created by a wrapper script that launches Claude Code inside the worktree. This is a blocker for Component 1.
2. **Should claims be per-file or per-module?** Per-file is precise but noisy (100+ claims). Per-module is coarse but practical. V1: per-file, with aggregation in `/coord/who` display.
3. **What happens when the kernel is down?** Current behavior: `CYNIC_COORD_ALLOW_DEGRADED=1` bypasses claims. V1 keeps this but logs degraded sessions for post-hoc audit.
4. **Gemini CLI / Codex integration:** The hooks are Claude Code-specific. Gemini has its own hook system (`BeforeTool`). Codex has none. V1 covers Claude Code only. V2 abstracts the hook layer.

---

*"The organism doesn't prevent mutations. It selects for fitness."*
