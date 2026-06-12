# Multi-Cortex Coordination v2 — Mechanical Isolation

**Date:** 2026-05-25
**Status:** APPROVED
**Scope:** Replace advisory coordination with mechanical branch isolation at session-init

---

## Problem

Multiple Claude Code sessions run simultaneously on the same repo. The current coordination system (L0: convention-only) fails because it depends on three unreliable actors:

1. **The LLM** should check `/coord/who` and create a branch before editing — it forgets.
2. **The human** should dispatch sessions with non-overlapping scope — they forget.
3. **The hooks** should enforce claims at edit-time — they're advisory (exit 0) or broken.

Concrete incident (2026-05-25): two sessions on the same branch, converging on overlapping files. The kernel had scope information but nothing acted on it. The `coord-claim.sh` hook is advisory-only. The `observe-tool.sh` hook (which includes heartbeats) is not wired in settings.json.

### Root Cause

The system puts the coordination burden on the wrong actor at the wrong time. Edit-time claims are bureaucratic overhead that doesn't prevent the actual failure mode (two sessions choosing the same work). By the time hooks fire on Edit, the scope overlap is already committed.

## Principle

**Coordination at dispatch, not at edit.** The human dispatches scope, session-init mechanically isolates, git resolves at merge. Zero mid-work bureaucracy.

## Falsification Criteria

- If merge conflicts exceed 30% of PRs after this change: branch isolation alone is insufficient, need scope-partitioning at dispatch
- If session-init latency exceeds 3s: the `/coord/who` or `git fetch` is too expensive, needs caching or timeout reduction
- If phantom session expiry still occurs after heartbeat wiring: PostToolUse hook is not firing

---

## Design

### Tier 1: Local Git (always works, zero dependencies)

`session-init.sh` creates a unique branch automatically before the LLM receives its first message.

```bash
# Injected into session-init.sh after AGENT_ID derivation (line ~190)
# NOTE: The CLAIMED_MODULES block at line ~100 runs BEFORE this point —
# that's fine because it's removed (broken jq). The only collision check
# that matters is the Tier 2 check below, which runs AFTER auto-branch.

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
CORTEX_BRANCH="$CURRENT_BRANCH"

if [[ "$CURRENT_BRANCH" == "main" || "$CURRENT_BRANCH" == "master" ]]; then
    # On main — always auto-branch
    CORTEX_BRANCH="cortex/${AGENT_ID}-$(date +%Y-%m-%d)-$(head -c4 /dev/urandom | xxd -p)"
    git checkout -b "$CORTEX_BRANCH" 2>/dev/null || true
    echo "AUTO-BRANCH: created ${CORTEX_BRANCH} (mechanical isolation)"
fi
```

**Known gap: sessions on existing feature branches.** If the user manually checks out a feature branch before launching Claude Code, no auto-branch occurs. Two sessions on the same feature branch have no mechanical protection — the Tier 2 collision check (below) will detect and resolve this when the kernel is available, but without kernel, both sessions work on the same branch. This is accepted: the human explicitly chose the branch, so they own the coordination. The random suffix on auto-branches makes accidental collision astronomically unlikely; deliberate branch sharing is a human decision.

The LLM finds itself on a unique branch when it starts. `branch-guard.sh` is the safety net — if the auto-branch somehow fails, main is still protected. The `|| true` prevents `set -euo pipefail` from aborting session-init on git errors.

### Tier 2: Kernel Coordination (enrichment, not enforcement)

When the kernel is reachable, session-init registers and checks for branch collisions:

```bash
# After auto-branch, if kernel is up:

if [[ "$KERNEL_STATUS" != "down" ]]; then
    # Register with branch in scope
    curl -s --max-time 2 -X POST "http://${KERNEL_ADDR}/coord/register" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"intent\":\"claude-code session\",\"agent_type\":\"claude\",\"scope\":\"branch:${CORTEX_BRANCH}\"}" \
        > /dev/null 2>&1 &

    # Check for branch collision (another session on same branch)
    WHO_JSON=$(curl -s --max-time 2 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/coord/who" 2>/dev/null || echo '{}')

    BRANCH_COLLISION=$(echo "$WHO_JSON" | jq -r --arg branch "$CORTEX_BRANCH" --arg self "$AGENT_ID" \
        '[.agents[]? | select(.active == true and .agent_id != $self and (.scope // "" | contains($branch)))] | length' \
        2>/dev/null || echo "0")

    if [[ "$BRANCH_COLLISION" -gt 0 ]]; then
        # Another session on same branch — create new branch
        CORTEX_BRANCH="cortex/${AGENT_ID}-$(date +%Y-%m-%d)-$(head -c4 /dev/urandom | xxd -p)"
        git checkout -b "$CORTEX_BRANCH" 2>/dev/null
        echo "BRANCH COLLISION: switched to ${CORTEX_BRANCH}"
    fi
fi
```

**If kernel is down:** auto-branch already happened (Tier 1). The random suffix makes collision extremely unlikely. No degradation.

**Consolidation with existing `/coord/who` at line ~400:** The existing MC4 "Active sessions check" block at line ~400 of session-init.sh already calls `/coord/who`. The Tier 2 collision check above is a REPLACEMENT, not an addition. The implementation must merge the two blocks into one: fetch `/coord/who` once, use the result for both branch collision detection and concurrent-session reporting.

### Tier 3: Heartbeat (keep-alive, prevents phantom expiry)

`observe-tool.sh` exists on disk but has **never been wired** in settings.json — the heartbeat capability has never fired in production. `heartbeat-tool.sh` is a new, stripped-down hook that adds heartbeat capability for the first time. No observation POST (K15: no consumer for tool observations).

**New `heartbeat-tool.sh`**:

```bash
#!/usr/bin/env bash
# CYNIC — PostToolUse: fire-and-forget heartbeat to keep session alive.
# Prevents 5-min TTL expiry during long tool sequences.
# No observation POST — K15: no consumer for tool observations.
set -uo pipefail

INPUT=$(cat)
source ~/.cynic-env 2>/dev/null || true

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
AGENT_ID="claude-${SESSION_ID:0:12}"
[[ "$SESSION_ID" == "" ]] && exit 0

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# Fire-and-forget heartbeat — async, 1s timeout, ignore result
curl -s --max-time 1 -X POST "http://${KERNEL_ADDR}/coord/heartbeat" \
    -H "Content-Type: application/json" \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    -d "{\"agent_id\":\"${AGENT_ID}\"}" \
    > /dev/null 2>&1 &

exit 0
```

Wire in settings.json as PostToolUse for Edit, Write, Bash (the tools that indicate active work). Async, fire-and-forget, ~0ms overhead.

### Scope Update

**No change needed.** `observe-prompt.sh` already fires the scope POST on every `UserPromptSubmit` unconditionally (lines 56-65). The dispatch marker at line 31 guards only the `human_dispatch` observation, not the scope update. The scope POST keeps `last_seen` fresh on every user message.

---

## What Changes

### settings.json

| Hook slot | Before | After |
|-----------|--------|-------|
| `PreToolUse (Edit\|Write)` | `branch-guard.sh`, `coord-claim.sh` | `branch-guard.sh` only |
| `PostToolUse (Edit\|Write\|Bash)` | `rustfmt-rs.sh` (Edit/Write only) | `rustfmt-rs.sh` (Edit/Write), `heartbeat-tool.sh` (Edit/Write/Bash) |

### session-init.sh

| Section | Before | After |
|---------|--------|-------|
| Branch creation | Not present — LLM convention | Auto-branch after AGENT_ID derivation |
| /coord/register | Fire-and-forget, no branch in scope | Includes `branch:<name>` in scope |
| MC4 check | Advisory warning on stderr | Branch collision → auto-create new branch |
| CLAIMED_MODULES jq | Broken path (`.agents[]?.claims[]?`) | Removed (claims are dead) |

### observe-prompt.sh

No changes needed. Scope update already fires on every prompt (verified in code).

### CLAUDE.md Multi-Cortex Rules

| Rule | Before | After |
|------|--------|-------|
| Rule 1 (branch before edit) | LLM convention | Deleted — session-init handles it mechanically |
| Rule 4 (check origin) | LLM convention | Partially mechanized: `git fetch` already in session-init (line 68), collision check added. **Rebase-before-PR discipline** (the non-mechanizable part of Rule 4) is preserved as a note under Rule 3b: "If another session pushed while you worked, rebase before PR. Never force-push." |
| Rule 5 (module-level ownership) | LLM convention | Deleted — no edit-time claims |

Rules 2 (scope = user's message), 3 (TODO.md read-only), 3b (hot files last-merger-wins), 6 (scope before launching second cortex) are unchanged. Rule 6 remains advisory — the LLM checks `/coord/who` before dispatching a second cortex. The mechanical isolation in session-init is the safety net if Rule 6 is ignored.

### Files

| File | Action |
|------|--------|
| `.cortex/mcp/session-init.sh` | MODIFY — add auto-branch logic after AGENT_ID, consolidate `/coord/who` calls |
| `.cortex/mcp/heartbeat-tool.sh` | CREATE — new heartbeat-only PostToolUse hook |
| `.claude/settings.json` | MODIFY — remove coord-claim.sh, add heartbeat-tool.sh |
| `CLAUDE.md` | MODIFY — update Multi-Cortex rules to reflect mechanical isolation |

### Files NOT touched

| File | Why |
|------|-----|
| `coord-claim.sh` | Left on disk for Codex MCP reference. Unhooked from settings.json. |
| `observe-tool.sh` | Left on disk. Never wired in settings.json. `heartbeat-tool.sh` is a new capability, not a replacement. |
| `observe-prompt.sh` | Already fires scope POST on every prompt — no changes needed. |
| Kernel `/coord/*` endpoints | Unchanged. Still used by MCP (Codex), session-init, session-stop. |
| `branch-guard.sh` | Unchanged. Safety net if auto-branch fails. |
| `session-stop.sh` | Zone claim release (lines 51-58, `/tmp/cynic-zones/`) becomes dead code — remove in same commit. The `/coord/release` call (line 43-47) stays (releases kernel session). |

---

## Degradation Modes

| Failure | Impact | Mitigation |
|---------|--------|------------|
| Kernel down | No branch collision detection, no heartbeat | Tier 1 auto-branch still works (local git). Random suffix prevents collision. |
| SurrealDB down (NullCoord) | All coord ops silently succeed | Same as kernel down — Tier 1 covers it. |
| git fetch fails (timeout) | Can't detect remote branch state | Auto-branch from local HEAD. Conflicts detected at push time. |
| heartbeat-tool.sh fails | Session expires in /coord/who after 5min | Work continues on local branch. Visibility lost, not capability. |
| session-init.sh fails entirely | No auto-branch, no registration | branch-guard.sh blocks main edits. LLM must manually branch (fallback to L0). |

---

## What Dies

1. **`coord-claim.sh` hook wiring** — removed from settings.json PreToolUse
2. **Edit-time claim checking** — no more advisory zone-activity queries
3. **CLAIMED_MODULES extraction** — broken jq removed from session-init
4. **CLAUDE.md Rule 1** ("branch before ANY edit") — mechanical, not convention
5. **CLAUDE.md Rule 5** ("module-level ownership per session") — no edit-time claims

## What Lives

1. **`branch-guard.sh`** — unchanged safety net
2. **Auto-branch at session-init** — new mechanical isolation
3. **Heartbeat via `heartbeat-tool.sh`** — new PostToolUse async hook
4. **`/coord/who` at session-init** — branch collision detection (enrichment)
5. **`/coord/register` + `/coord/release`** — session lifecycle
6. **`session-stop.sh`** — cleanup (zone claim release removed, `/coord/release` stays)
7. **`observe-prompt.sh`** — scope update already on every prompt (no change needed)

---

## Compliance Resistance

| Actor | Failure mode | System response |
|-------|-------------|----------------|
| LLM forgets to branch | Already on a unique branch (session-init) |
| Human dispatches overlapping scope | Each session on its own branch, git merges |
| LLM ignores /coord/who warning | Doesn't matter — isolation is a fait accompli |
| Human runs 5 sessions simultaneously | 5 auto-branches, 5 PRs, human merges in order |
| Kernel down during dispatch | Tier 1 (local git) covers — no coordination needed |
| Session idle >5min (TTL expiry) | Heartbeat hook prevents if tools are active. If truly idle (waiting for human), expiry is correct — session is inactive. |
