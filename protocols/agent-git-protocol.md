# CYNIC Agent Git Protocol — protocols/agent-git-protocol.md

## Purpose

This document specifies the **executable protocol** for multi-agent Git collaboration
on the CYNIC repository. It eliminates chaos from concurrent agent mutations
by formalizing branch ownership, scope claims, sync workflows, and merge procedures.

**Authority:** This protocol is normative. All agents (Hermes, Codic, Claude, native CYNIC)
must follow it. Where this document disagrees with informal habits, this document wins.

**Design principle:** Git is a coordination substrate. CYNIC provides the coordination
protocol (claims, heartbeat, release). Git provides the transport. Neither replaces the other.

## Part 1 — Branch Topology

### Branch hierarchy

```
main                    ← production runtime authority (protected)
├── agent/<name>/<scope> ← individual agent working branches
├── staging/constitution ← integration branches per subsystem
├── staging/coordination
├── staging/build
└── hotfix/<issue>       ← emergency fixes only
```

### Branch naming

| Pattern | Owner | Lifetime |
|---|---|---|
| `agent/<agent>/<scope>` | One agent | One session/task |
| `staging/<subsystem>` | Integration | Until merged |
| `hotfix/<issue>` | Any agent | Until merged |

**Agent names:** `hermes`, `codex`, `claude`, `cynic-native`, or the MCP `principal`.

**Scope examples:**
- `agent/hermes/constitution` — constitution work by Hermes
- `agent/codex/coordination-protocol` — coordination work by Codex
- `agent/claude/build-runner` — build runner work by Claude

### Branch creation

```bash
# 1. Always start from latest main
git fetch origin
git checkout -b agent/<agent>/<scope> origin/main

# 2. Immediately claim the scope (see Part 2)
cynic agent claim --agent-id "<agent>:<scope>" --target "<scope>" --json
```

## Part 2 — Scope Claims (CYNIC Coordination)

Before any mutation, the agent **must** hold a valid claim on the scope.

### Claim via CLI

```bash
cynic agent claim \
  --agent-id "hermes:constitution" \
  --target "protocols/,crates/cynic-protocol/" \
  --claim-type crate \
  --ttl-seconds 3600 \
  --budget-actions 100 \
  --json
```

### Claim via MCP

```json
{
  "tool": "cynic_coord_claim",
  "arguments": {
    "agent_id": "hermes:constitution",
    "target": "protocols/",
    "claim_type": "crate",
    "ttl_seconds": 3600,
    "budget_actions": 100
  }
}
}

### Claim conflict resolution

| Situation | Resolution |
|---|---|
| Same scope, same mode | First-wins. Second picks another scope or waits. |
| Cynic vs baseline | Cynic preempts. Baseline gets grace period (300s). |
| Overlapping scopes | Most specific wins. `File` beats `Crate` beats `Fleet`. |

### Claim lifecycle

```
UNCLAIMED → CLAIMED → HEARTBEATING → MUTATING → RELEASED
              ↓                        ↓
           EXPIRED                 PREEMPTED
```

## Part 3 — Sync Workflow

### Session start

```bash
# 1. Fetch latest state
git fetch origin --prune

# 2. If branch exists: rebase on origin/main
git checkout agent/<agent>/<scope>
git rebase origin/main

# 3. If conflict: resolve or re-create branch
# (see Part 6 — Conflict Resolution)

# 4. Verify claim is still active
cynic agent status --agent-id "<agent>:<scope>" --json

# 5. If expired: re-claim
cynic agent claim --agent-id "<agent>:<scope>" --target "<scope>" --json
```

### During session (every ~15 min)

```bash
# 1. Heartbeat the claim
cynic agent claim --agent-id "<agent>:<scope>" --target "<scope>" --json

# 2. Check for conflicting claims
cynic runtime coord --json | jq '.active_claims'

# 3. If preemption imminent: release cleanly, stash work, wait
```

### Session end

```bash
# 1. Commit all work
git add -A
git commit -m "<type>(<scope>): <description>"

# 2. Push branch to origin
git push origin agent/<agent>/<scope>

# 3. Release claim
cynic agent release \
  --agent-id "<agent>:<scope>" \
  --target "<scope>" \
  --lease-id "<from-claim>" \
  --fencing-token <from-claim> \
  --json

# 4. Create PR via MCP or CLI
cynic git create-pr \
  --branch "agent/<agent>/<scope>" \
  --title "<type>(<scope>): <description>" \
  --json
```

## Part 4 — Merge Protocol

### Rule: No direct push to `main`

All changes must go through a PR (Pull Request) on Forgejo.

### PR creation

```bash
# Via CLI
cynic git create-pr \
  --branch "agent/<agent>/<scope>" \
  --base main \
  --title "feat(constitution): formalize agent constitution" \
  --body "## Changes\n- protocols/agent-constitution.md\n- cynic-protocol::harness_authority\n- cynic-cli agent_surface\n\n## Claims Released\n- agent/hermes/constitution\n\n## Verification\n- cargo check green\n- All tests pass"

# Via MCP
{
  "tool": "cynic_git",
  "arguments": {
    "action": "create_pr",
    "branch": "agent/<agent>/<scope>",
    "base": "main",
    "title": "feat(constitution): formalize agent constitution"
  }
}
}
```

### PR review requirements

| Requirement | Enforcement |
|---|---|
| Build passes (`make check`) | CI webhook on Forgejo |
| No conflicting claims | CYNIC coordination check |
| Scope documented in claim | Automatic via claim metadata |
| Migration path documented | Manual review |

### Merge procedure

1. PR opened → CI webhook triggers build
2. Build green → reviewer approves
3. Merge via `git merge --no-ff` (preserves branch history)
4. Delete branch after merge
5. Release claim (if not already done)

## Part 5 — Sync with Origin (Anti-Chaos)

### Regular sync (every ~30 min or before any push)

```bash
# 1. Fetch without merging
git fetch origin

# 2. Check if origin/main has diverged
git log --oneline HEAD..origin/main

# 3. If diverged: rebase
git rebase origin/main

# 4. If conflicts: resolve (see Part 6)
```

### Pre-push check

```bash
# NEVER push without this check
git fetch origin
git diff --stat origin/main

# If more than ~50 files changed: STOP, rebase first
# If conflicting files: STOP, coordinate with other agent
```

## Part 6 — Conflict Resolution

### Scenario A: Rebase conflict during sync

```bash
# 1. Abort current rebase
git rebase --abort

# 2. Create backup
git branch backup/<agent>-<scope>-$(date +%s)

# 3. Stash your changes
git stash push -m "WIP: <scope> before rebase"

# 4. Reset to origin/main
git fetch origin
git reset --hard origin/main

# 5. Apply stash
git stash pop

# 6. Resolve conflicts manually
# 7. Commit and push
```

### Scenario B: Another agent modified your files

```bash
# 1. Check who modified what
git log --oneline origin/main -- <conflicting-files>

# 2. Check their claim status
cynic runtime coord --json | jq '.active_claims'

# 3. If they released: cherry-pick or re-apply your changes
# 4. If they still claim: coordinate (chat, MCP, or wait)
```

### Scenario C: Destructive rebase by another agent

```bash
# 1. DON'T force push — you'd destroy their work
# 2. Create backup of your work
git branch backup/<agent>-<scope>-emergency
git push origin backup/<agent>-<scope>-emergency

# 3. Observe the new origin/main
git fetch origin
git log --oneline origin/main -20

# 4. Rebase your branch onto the new origin/main
git checkout agent/<agent>/<scope>
git rebase origin/main

# 5. If hundreds of conflicts: re-create branch from scratch
git checkout origin/main -b agent/<agent>/<scope>-v2
git cherry-pick <your-commits-from-backup>

# 6. Push new branch
git push origin agent/<agent>/<scope>-v2
```

## Part 7 — Emergency Procedures

### When to use emergency branch

- Critical production fix needed immediately
- Another agent's work is blocking yours after 30+ minutes
- Destructive rebase detected

```bash
# 1. Create hotfix branch from origin/main
git checkout -b hotfix/<issue> origin/main

# 2. Apply minimal fix
# 3. Push
git push origin hotfix/<issue>

# 4. Create PR with emergency label
# 5. Merge after CI green (skip review if critical)
```

### Claim emergency release

If an agent dies (no heartbeat for 2x TTL), its claims expire automatically.
Other agents can then claim the same scope:

```bash
# 1. Check expired claims
cynic runtime coord --json | jq '.expired_claims'

# 2. Claim the freed scope
cynic agent claim --agent-id "<agent>" --target "<scope>" --json
```

## Part 8 — Current Incident Response

The repository is currently in a chaotic state due to a destructive rebase
on origin/main. Here's the recovery plan:

### Phase 1 — Stabilize (this session)

1. Reset `main` to `origin/main` (accept the other agent's rebase)
2. Create `agent/hermes/constitution` branch from `origin/main`
3. Apply constitution work as a single commit
4. Push branch, create PR

### Phase 2 — Coordinate

1. Verify all other agents have switched to branch-based workflow
2. Establish claim discipline
3. Document any remaining conflicts

### Phase 3 — Merge

1. Once PR approved and CI green, merge to main
2. Delete working branch
3. Release all claims

## Files

| File | Role |
|---|---|
| `.agent-claims.json` | Local claim cache (gitignored) |
| `infra/registry.json` | Node topology (discover endpoints) |
| `cynic-protocol/src/agent_coordination.rs` | Canonical claim types |
| `cynic-mcp/src/routers/coord.rs` | MCP tools for claims |
| `cynic-cli/src/agent_surface/` | CLI commands for claims |

## Verification

```bash
# Check current branch
git branch --show-current
# Expected: agent/<agent>/<scope> (NOT main)

# Check claim status
cynic agent status --agent-id "<agent>:<scope>" --json
# Expected: status=active, remaining_seconds > 0

# Check sync state
git fetch origin
git log --oneline HEAD..origin/main
# Expected: 0 commits behind (or resolve with rebase)
```

---

*This protocol is executable prose. Law lives in the Rust types and the Git workflow.
Transpose, compile, test, merge.*
