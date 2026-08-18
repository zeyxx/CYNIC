# Cortex Proof-of-History Schema

**Version:** 1.1  
**Purpose:** Multi-cortex coordination. Prevents feat/k15 vs feat/agent-logging divergence.  
**Location:** `.cortex/session-proof.json` (git-tracked, updated by hooks)

## Overview

Each Claude Code session (cortex) emits a proof of what it did and what state it left behind. The proof is:
- **Written at session START** by `session-init.sh` (AT_START section)
- **Updated at session END** by `session-stop.sh` (AT_END section)
- **Read by NEXT session** to verify continuity and detect violations

This prevents divergence because the next session sees:
1. What branches the previous session left open
2. What commits are unpushed
3. What modules other active agents are claiming
4. Whether previous session had coordination debt (MC violations)

## Schema

```json
{
  "session_id": "claude-6e22996b-89a",
  "schema_version": "1.1",
  "recorded_at": "2026-04-30T02:30:00Z",
  
  "AT_START": {
    "branch_name": "feat/agent-logging-...",
    "last_commit_hash": "39de9ae",
    "origin_commit_hash": "657df06",
    "local_behind_origin": 1,
    "local_ahead_of_origin": 18,
    "dirty_files": 2,
    "open_branches": ["feat/k15-loop-closure-2026-04-30", "fix/qwen35-gpu-config-2026-04-27"],
    "open_prs": [],
    "stashes": [],
    "claimed_by_active_agents": ["cynic-kernel/src/tasks.rs"],
    "kernel_status": "degraded",
    "dogs_active": 3
  },
  
  "AT_END": {
    "final_commit_hash": "9858966",
    "branches_deleted": 1,
    "duration_minutes": 47,
    "commits_produced": 10,
    "compliance_score": 0.618,
    "work_lost": "none",
    "completed_at": "2026-04-30T02:47:30Z"
  }
}
```

## Violation Detection Rules

### V1: Unpushed commits on main (VIOLATION)
- **Rule:** If `branch_name == "main"` AND `local_ahead_of_origin > 0`
- **Why:** Work on main could be lost if not pushed
- **Remediation:** `git push origin main`

### V2: Multiple open branches (COORDINATION DEBT)
- **Rule:** If `open_branches.length > 1`
- **Why:** MC2 violation — PR before new work not followed
- **Remediation:** Check if PR#N is merged; if yes, delete the branch locally

### V3: Stale stashes (INCOMPLETE WORK)
- **Rule:** If `stashes.length > 0`
- **Why:** Previous session left work on the shelf
- **Remediation:** `git stash pop` and decide: complete or drop?

### V4: Module claims conflict (MC4 CONSTRAINT)
- **Rule:** If `claimed_by_active_agents` contains modules you're about to edit
- **Why:** Another cortex is actively editing that file
- **Remediation:** Check `/coord/who`; wait for other session to release or re-partition work

## Next Session's Consumption

At session start, `session-init.sh` automatically:

1. **Reads** `.cortex/session-proof.json` (if it exists from prior session)
2. **Detects** any violations (V1-V4)
3. **Warns** the human in the session-init output
4. **Prevents divergence** by flagging what needs to be resolved

Example output:
```
⚠ GIT STATE VIOLATIONS (Rule MC2/MC5 — coordinate before proceeding):
  7 branches remain open (MC2: review if PRs are merged)
  → Review .cortex/session-proof.json for previous session state

⚠ ACTIVE MODULE CLAIMS (Rule MC4 — other cortices are editing):
  → cynic-kernel/src/tasks.rs
  Before editing these modules, verify other sessions are done (check /coord/who).
```

## Falsification Test

**Can this schema prevent feat/k15 vs feat/agent-logging divergence?**

Scenario:
1. Session A creates `feat/k15-loop-closure` and commits to `cynic-kernel/src/tasks.rs`
2. Session A ends; AT_END proof records: `branch_name="feat/k15-loop-closure"`, `commits_produced=5`
3. Session A claims module via `/coord/claim` → proof records in `claimed_by_active_agents`

4. Session B starts on `main` (from origin, where A's branch isn't yet merged)
5. Session-init.sh reads A's AT_END proof
6. It sees: `open_branches=["feat/k15-loop-closure"]` and `claimed_by_active_agents=["tasks.rs"]`
7. **V4 blocks:** Session B is warned "tasks.rs is claimed by feat/k15 session"
8. Session B either waits or chooses a different module

**Result:** No divergence. The feat/k15 and feat/agent-logging branches never both edit tasks.rs.

## Current Weaknesses (Known Gaps)

1. **V4 blocking is advisory, not enforced** — session-init.sh warns but doesn't prevent file edits. Mechanical enforcement requires MCP tool integration (future).

2. **claimed_modules comes from `/coord/who`** — only accurate if kernel is running and agents properly claim/release. If kernel is down, this field is empty.

3. **Proof is local-only until next session commits it** — if this session crashes before committing changes, the AT_END proof may not be written.

4. **No immutable chain yet** — currently `.cortex/session-proof.json` is a single file, overwritten by each session. Future: append-only `.jsonl` or kernel-stored ledger.

## Implementation Checklist

- [x] AT_START proof capture in session-init.sh
- [x] AT_END proof update in session-stop.sh
- [x] V1 violation detection (unpushed commits on main)
- [x] V2 violation detection (multiple branches)
- [x] V3 violation detection (stale stashes)
- [x] V4 constraint warning (module claims from `/coord/who`)
- [ ] V4 enforcement (mechanical block, not advisory)
- [ ] Append-only proof ledger (immutable chain)
- [ ] Kernel-stored proof endpoint (`GET /cortex-proofs`)
- [ ] CLI tool to verify proof chain (`cynic-kernel proof verify <session_id>`)
