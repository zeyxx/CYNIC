# Agent Protocol v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mechanical multi-cortex coordination — worktree isolation + extended claims + pre-push validation — solving the contamination problem observed 2026-05-15.

**Architecture:** Wrapper script (`cynic-cortex.sh`) creates isolated git worktrees per session. Extended coord claims track all files (not just kernel/src). Pre-push hook validates scope, freshness, and merged-PR status client-side. No new kernel endpoints.

**Tech Stack:** Bash (wrapper + hooks), Rust (kernel coord extensions), SurrealDB (work_claim schema), Claude Code hooks API.

**Spec:** `docs/superpowers/specs/2026-05-15-agent-protocol-v1-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `scripts/cynic-cortex.sh` | **Create** | Wrapper: clean check → worktree create → launch Claude Code |
| `scripts/cynic-cortex-cleanup.sh` | **Create** | Cron: prune worktrees older than 24h with no unpushed commits |
| `.cortex/mcp/coord-claim.sh` | **Modify** | Change conflict response from block to warning |
| `.cortex/mcp/session-stop.sh` | **Modify** | Add worktree cleanup + claim status update |
| `.claude/settings.json` | **Modify** | Remove `if` scope restriction on coord-claim |
| `scripts/git-hooks/pre-push` | **Modify** | Add scope match + freshness + merged-PR checks |
| `cynic-kernel/src/storage/mod.rs` | **Modify** | Add `status`, `intent`, `session_id` fields to work_claim bootstrap |
| `cynic-kernel/src/storage/surreal/coord.rs` | **Modify** | Status-aware expire_stale, claim-with-conflicts response |
| `cynic-kernel/src/api/rest/coord.rs` | **Modify** | Return 200+conflicts instead of 409 on claim conflict |
| `cynic-kernel/src/domain/coord.rs` | **Modify** | Add `ClaimWithConflicts` result variant |
| `cynic-kernel/tests/integration_storage.rs` | **Modify** | Tests for status-aware claims and expiry |

---

## Task 1: Wrapper Script (`cynic-cortex.sh`)

**Files:**
- Create: `scripts/cynic-cortex.sh`
- Create: `scripts/cynic-cortex-cleanup.sh`

- [ ] **Step 1: Write `cynic-cortex.sh`**

```bash
#!/usr/bin/env bash
# CYNIC Agent Protocol v1 — cortex launcher
# Creates an isolated git worktree per session, then launches Claude Code inside it.
# This is the ONLY way to start a coordinated cortex session.
set -euo pipefail

CYNIC_ROOT="${CYNIC_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null)}"
if [ -z "$CYNIC_ROOT" ]; then
    echo "ERROR: not in a git repository. Run from CYNIC project root." >&2
    exit 1
fi

SESSION_ID=$(head -c4 /dev/urandom | xxd -p)
WORKTREE_DIR="/tmp/cynic-worktrees/$SESSION_ID"
BRANCH="cortex/${SESSION_ID}-$(date +%Y-%m-%d)"

# Gate: shared checkout must be clean (tracked files only)
DIRTY=$(git -C "$CYNIC_ROOT" diff --name-only HEAD 2>/dev/null)
if [ -n "$DIRTY" ]; then
    echo "BLOCKED: shared checkout has uncommitted tracked changes:" >&2
    echo "$DIRTY" >&2
    echo "" >&2
    echo "Another cortex left dirty files. Resolve before starting:" >&2
    echo "  cd $CYNIC_ROOT && git stash  # or git checkout -- <files>" >&2
    exit 1
fi

# Fetch latest main
git -C "$CYNIC_ROOT" fetch origin main --quiet 2>/dev/null || true

# Create worktree
mkdir -p /tmp/cynic-worktrees
git -C "$CYNIC_ROOT" worktree add "$WORKTREE_DIR" -b "$BRANCH" origin/main --quiet
echo "Worktree created: $WORKTREE_DIR (branch: $BRANCH)"

# Launch Claude Code inside worktree
cd "$WORKTREE_DIR"
exec claude "$@"
```

- [ ] **Step 2: Make executable and test manually**

Run:
```bash
chmod +x scripts/cynic-cortex.sh
# Test 1: creates worktree and prints path
CYNIC_ROOT=$(pwd) scripts/cynic-cortex.sh --help 2>&1 | head -5
# Verify: /tmp/cynic-worktrees/<id> exists
ls /tmp/cynic-worktrees/
# Test 2: dirty tree blocks launch
echo "dirty" >> /tmp/test-dirty && git add /tmp/test-dirty 2>/dev/null; touch Cargo.lock.bak
# (simulate by modifying a tracked file temporarily)
```

- [ ] **Step 3: Write `cynic-cortex-cleanup.sh`**

```bash
#!/usr/bin/env bash
# CYNIC — Prune stale worktrees (cron: daily)
# Deletes worktrees older than 24h that have no unpushed commits.
set -euo pipefail

CYNIC_ROOT="${CYNIC_ROOT:-$HOME/Bureau/CYNIC}"
WORKTREE_BASE="/tmp/cynic-worktrees"

[ -d "$WORKTREE_BASE" ] || exit 0

NOW=$(date +%s)
CLEANED=0

for wt in "$WORKTREE_BASE"/*/; do
    [ -d "$wt" ] || continue
    # Age check: >24h
    CREATED=$(stat -c '%Y' "$wt" 2>/dev/null || echo "$NOW")
    AGE=$(( (NOW - CREATED) / 3600 ))
    [ "$AGE" -lt 24 ] && continue

    # Safety: check for unpushed commits
    BRANCH=$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
    if [ -n "$BRANCH" ]; then
        AHEAD=$(git -C "$wt" log --oneline "origin/main..$BRANCH" 2>/dev/null | wc -l)
        if [ "$AHEAD" -gt 0 ]; then
            PUSHED=$(git -C "$wt" log --oneline "origin/$BRANCH..$BRANCH" 2>/dev/null | wc -l)
            if [ "$PUSHED" -gt 0 ]; then
                echo "SKIP: $wt has $PUSHED unpushed commits (branch: $BRANCH)"
                continue
            fi
        fi
    fi

    # Remove worktree
    git -C "$CYNIC_ROOT" worktree remove "$wt" --force 2>/dev/null && CLEANED=$((CLEANED+1))
done

echo "Cleaned $CLEANED stale worktrees"
```

- [ ] **Step 4: Make executable**

Run: `chmod +x scripts/cynic-cortex-cleanup.sh`

- [ ] **Step 5: Commit**

```bash
git add scripts/cynic-cortex.sh scripts/cynic-cortex-cleanup.sh
git commit -m "feat(coord): cynic-cortex.sh wrapper — worktree isolation per session

Creates isolated git worktree per cortex session. Gates on clean
shared checkout. Cleanup script for stale worktrees (24h TTL).
Agent Protocol v1 Component 1."
```

---

## Task 2: Extend work_claim Schema

**Files:**
- Modify: `cynic-kernel/src/storage/mod.rs:288-294` (bootstrap schema)

- [ ] **Step 1: Add `status`, `intent`, `session_id` fields to bootstrap SQL**

In `cynic-kernel/src/storage/mod.rs`, after the existing `work_claim` field definitions, add:

```rust
// After: DEFINE INDEX IF NOT EXISTS work_claim_target_idx ON work_claim FIELDS target;\
// Add:
            DEFINE FIELD IF NOT EXISTS status ON work_claim TYPE string DEFAULT 'claimed';\
            DEFINE FIELD IF NOT EXISTS intent ON work_claim TYPE string DEFAULT '';\
            DEFINE FIELD IF NOT EXISTS session_id ON work_claim TYPE string DEFAULT '';\
```

- [ ] **Step 2: Run `cargo check`**

Run: `cargo check --workspace --all-targets`
Expected: compiles with no errors.

- [ ] **Step 3: Verify schema applies to live DB**

Run:
```bash
cargo test -p cynic-kernel --test integration_storage -- coord_claim_batch --exact
```
Expected: PASS (new fields are additive, existing tests unaffected).

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/storage/mod.rs
git commit -m "feat(coord): add status/intent/session_id fields to work_claim schema

Additive schema change — existing claims get defaults. Enables
status-aware expiry (PROPOSED→ABANDONED TTL) and intent tracking.
Agent Protocol v1 Component 2a."
```

---

## Task 3: Status-Aware Expiry in Kernel

**Files:**
- Modify: `cynic-kernel/src/storage/surreal/coord.rs:398-408` (expire_stale)
- Modify: `cynic-kernel/tests/integration_storage.rs` (new test)

- [ ] **Step 1: Write the failing integration test**

Add to `cynic-kernel/tests/integration_storage.rs`:

```rust
#[tokio::test]
async fn coord_proposed_claims_expire_after_ttl() {
    let Some(db) = common::setup_test_db("coord_proposed_ttl").await else {
        return;
    };
    db.register_agent("ttl-agent", "claude", "test proposed ttl")
        .await
        .unwrap();
    db.claim("ttl-agent", "file.rs", "file").await.unwrap();

    // Manually set status to 'proposed' and claimed_at to 3h ago
    db.query_one(
        "UPDATE work_claim SET status = 'proposed', claimed_at = time::now() - 3h WHERE agent_id = 'ttl-agent' AND active = true;"
    ).await.unwrap();

    // expire_stale should move proposed claims older than 2h to abandoned
    db.expire_stale().await.expect("expire_stale failed");

    let snap = db.who(None).await.unwrap();
    assert!(
        snap.claims.iter().all(|c| !c.active),
        "proposed claims older than 2h should be expired"
    );

    common::teardown_test_db(&db).await;
}
```

- [ ] **Step 2: Run test — verify it fails**

Run: `cargo test -p cynic-kernel --test integration_storage -- coord_proposed_claims_expire_after_ttl --exact`
Expected: FAIL (expire_stale doesn't check status yet).

- [ ] **Step 3: Extend `expire_stale()` with status-aware TTL**

In `cynic-kernel/src/storage/surreal/coord.rs`, in the `expire_stale` method, add after the audit TTL line:

```rust
        // Agent Protocol v1: expire proposed claims after 2h (prevents orphan claims)
        self.query_one("UPDATE work_claim SET active = false, status = 'abandoned' WHERE status = 'proposed' AND active = true AND (time::now() - claimed_at) > 2h;").await
            .map_err(|e| CoordError::StorageFailed(format!("expire proposed: {e}")))?;
```

- [ ] **Step 4: Run test — verify it passes**

Run: `cargo test -p cynic-kernel --test integration_storage -- coord_proposed_claims_expire_after_ttl --exact`
Expected: PASS.

- [ ] **Step 5: Run full clippy + test suite**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo test -p cynic-kernel --test integration_storage`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/storage/surreal/coord.rs cynic-kernel/tests/integration_storage.rs
git commit -m "feat(coord): status-aware expiry — PROPOSED claims auto-abandon after 2h

Prevents orphan claims from blocking other agents indefinitely.
Agent Protocol v1 Component 2b."
```

---

## Task 4: Claim Returns Conflicts Instead of Blocking

**Files:**
- Modify: `cynic-kernel/src/domain/coord.rs` (add ClaimResultWithConflicts)
- Modify: `cynic-kernel/src/api/rest/coord.rs:162-175` (change 409 → 200+conflicts)
- Modify: `.cortex/mcp/coord-claim.sh` (warning instead of block)

- [ ] **Step 1: Modify REST handler — return 200 with conflicts**

In `cynic-kernel/src/api/rest/coord.rs`, replace the `Conflict` arm (around line 162):

```rust
        // Before: returned 409 CONFLICT which blocked the hook
        // After: return 200 with conflicts listed — claim succeeds, agent warned (T1: signal not lock)
        Ok(ClaimResult::Conflict(conflicts)) => {
            let conflict_list: Vec<_> = conflicts
                .iter()
                .map(|c| serde_json::json!({"agent_id": &c.agent_id, "claimed_at": &c.claimed_at}))
                .collect();
            let _ = state.coord.store_audit(
                "cynic_coord_claim", &req.agent_id,
                &serde_json::json!({"target": req.target, "claim_type": claim_type, "conflicts": &conflict_list, "source": "rest"}).to_string(),
            ).await;
            Ok(Json(serde_json::json!({
                "status": "claimed_with_conflicts",
                "agent_id": req.agent_id,
                "target": req.target,
                "conflicts": conflict_list,
            })))
        }
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --workspace --all-targets`

- [ ] **Step 3: Update `coord-claim.sh` — warn instead of block on conflict**

Replace the `409)` case in `.cortex/mcp/coord-claim.sh`:

```bash
    200)
        # Check for conflicts in response (signal, not lock)
        CONFLICTS=$(jq -r '.conflicts // empty' "$CLAIM_TMP" 2>/dev/null)
        if [ -n "$CONFLICTS" ] && [ "$CONFLICTS" != "null" ] && [ "$CONFLICTS" != "[]" ]; then
            CONFLICT_AGENTS=$(jq -r '.conflicts[].agent_id' "$CLAIM_TMP" 2>/dev/null | tr '\n' ', ')
            echo "⚠ SCOPE OVERLAP: '$TARGET_FILE' also claimed by: ${CONFLICT_AGENTS%,}" >&2
            echo "  Your work may conflict. Check /coord/who for details." >&2
        fi
        exit 0
        ;;
    409)
        # Legacy: should not happen after REST change, but handle gracefully
        CONFLICT_MSG=$(jq -r '.error // "conflict"' "$CLAIM_TMP" 2>/dev/null || echo "conflict")
        echo "⚠ CLAIM CONFLICT (legacy 409): $CONFLICT_MSG" >&2
        exit 0  # Changed from exit 2 (block) to exit 0 (warn)
        ;;
```

- [ ] **Step 4: Run integration tests**

Run: `cargo test -p cynic-kernel --test integration_storage`
Expected: 25/25 pass (REST behavior change is tested via rest_routes, not integration_storage).

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/api/rest/coord.rs .cortex/mcp/coord-claim.sh
git commit -m "feat(coord): claim returns 200+conflicts instead of 409 block

Implements T1: claim as signal, not lock. The claim succeeds and
the agent is warned about overlapping scopes. The cost of ignoring
the warning (wasted tokens at PROPOSE) is the natural incentive.
Agent Protocol v1 Component 2c."
```

---

## Task 5: Remove Scope Restriction on Claims

**Files:**
- Modify: `.claude/settings.json:70-78`

- [ ] **Step 1: Remove `if` restriction on coord-claim hooks**

In `.claude/settings.json`, remove the `"if"` field from both coord-claim entries:

```json
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.cortex/mcp/branch-guard.sh"
          },
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.cortex/mcp/coord-claim.sh"
          },
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.cortex/mcp/coord-claim.sh"
          }
        ]
      }
```

Note: Two entries because the original had one for `Edit(/cynic-kernel/src/**)` and one for `Write(...)`. With the `if` removed, both fire on all Edit/Write. Merge them into one entry to avoid duplicate claims.

- [ ] **Step 2: Test that claims fire for non-kernel files**

Manual test: Edit a file outside `cynic-kernel/src/` and verify `coord-claim.sh` fires (check kernel logs or `/coord/who`).

- [ ] **Step 3: Commit**

```bash
git add .claude/settings.json
git commit -m "feat(coord): extend claims to all files, not just kernel/src

Removes if-scope restriction on coord-claim.sh. All Edit/Write
operations now register claims with the kernel.
Agent Protocol v1 Component 2d."
```

---

## Task 6: Pre-Push Validation (PROPOSE Gate)

**Files:**
- Modify: `scripts/git-hooks/pre-push:86-98` (add checks after make check)

- [ ] **Step 1: Add scope + freshness + merged-PR checks to pre-push**

In `scripts/git-hooks/pre-push`, add BEFORE the final success message (before line 96 "All gates passed"):

```bash
# ── Agent Protocol v1: PROPOSE validation ──
echo "▶ Agent Protocol checks..."

source ~/.cynic-env 2>/dev/null || true
KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

# Derive agent_id from session state (same logic as coord-claim.sh)
AGENT_ID=""
SESSION_STATE_DIR="/tmp/cynic-sessions"
if [ -d "$SESSION_STATE_DIR" ]; then
    RECENT_STATE=$(ls -t "$SESSION_STATE_DIR"/*.state 2>/dev/null | head -1)
    [ -n "$RECENT_STATE" ] && AGENT_ID=$(grep -oP 'agent_id=\K[^ ]+' "$RECENT_STATE" 2>/dev/null || true)
fi

BRANCH=$(git rev-parse --abbrev-ref HEAD)

# Check 1: PR not already merged (prevents orphan commits)
MERGED_PR=$(gh pr list --head "$BRANCH" --state merged --json number --jq '.[0].number' 2>/dev/null || true)
if [ -n "$MERGED_PR" ]; then
    echo ""
    echo "✗ PR #$MERGED_PR already merged for branch '$BRANCH'."
    echo "  Create a new branch for new work:"
    echo "    git checkout -b <type>/<scope>-$(date +%Y-%m-%d)-$(head -c4 /dev/urandom | xxd -p)"
    exit 1
fi

# Check 2: Freshness (branch must include origin/main)
git fetch origin main --quiet 2>/dev/null || true
if ! git merge-base --is-ancestor origin/main HEAD 2>/dev/null; then
    echo ""
    echo "✗ Branch is not rebased on origin/main."
    echo "  Run: git rebase origin/main"
    exit 1
fi

# Check 3: Scope match (files in diff ⊆ claimed targets) — requires kernel + agent_id
if [ -n "$API_KEY" ] && [ -n "$AGENT_ID" ]; then
    DIFF_FILES=$(git diff --name-only origin/main..HEAD | sort)
    CLAIMED_JSON=$(curl -s --max-time 3 \
        -H "Authorization: Bearer $API_KEY" \
        "http://${KERNEL_ADDR}/coord/who?agent_id=$AGENT_ID" 2>/dev/null || echo '{}')
    CLAIMED=$(echo "$CLAIMED_JSON" | jq -r '.claims[]?.target // empty' 2>/dev/null | sort)

    if [ -n "$DIFF_FILES" ] && [ -n "$CLAIMED" ]; then
        UNCLAIMED=$(comm -23 <(echo "$DIFF_FILES") <(echo "$CLAIMED"))
        if [ -n "$UNCLAIMED" ]; then
            echo ""
            echo "✗ SCOPE VIOLATION: files modified but not claimed:"
            echo "$UNCLAIMED" | sed 's/^/    /'
            echo "  Edit these files to trigger coord-claim, or claim manually."
            exit 1
        fi
    fi
    echo "  ✓ Scope, freshness, PR status verified"
else
    echo "  ⚠ Scope check skipped (no CYNIC_API_KEY or AGENT_ID)"
fi
```

- [ ] **Step 2: Test merged-PR check**

Run:
```bash
# Create a test: push a branch, merge the PR, try pushing again
# This is manual — verify the pre-push hook blocks with the right message
```

- [ ] **Step 3: Test freshness check**

Run:
```bash
# Test: create a branch from an old commit, try pushing
# git checkout -b test/stale HEAD~5 && git push origin test/stale
# Should block with "not rebased on origin/main"
```

- [ ] **Step 4: Commit**

```bash
git add scripts/git-hooks/pre-push
git commit -m "feat(coord): pre-push PROPOSE gate — scope, freshness, merged-PR

Client-side validation before push:
1. PR not already merged (prevents orphan commits)
2. Branch rebased on origin/main (prevents stale base)
3. All modified files must be claimed (scope enforcement)
Agent Protocol v1 Component 3."
```

---

## Task 7: Session Stop Cleanup

**Files:**
- Modify: `.cortex/mcp/session-stop.sh`

- [ ] **Step 1: Add worktree cleanup to session-stop.sh**

Add at the END of `session-stop.sh`, before the final exit:

```bash
# ── Agent Protocol v1: worktree cleanup ──
WORKTREE_DIR=$(pwd)
if [[ "$WORKTREE_DIR" == /tmp/cynic-worktrees/* ]]; then
    CYNIC_ROOT="${CYNIC_ROOT:-$HOME/Bureau/CYNIC}"
    BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

    # Check for uncommitted changes
    DIRTY=$(git diff --name-only HEAD 2>/dev/null)
    if [ -n "$DIRTY" ]; then
        echo "⚠ Worktree has uncommitted changes — preserving for 24h:" >&2
        echo "$DIRTY" >&2
    else
        # Check if branch was pushed
        if [ -n "$BRANCH" ] && git rev-parse "origin/$BRANCH" &>/dev/null; then
            echo "Worktree cleanup: branch '$BRANCH' pushed, removing worktree." >&2
            cd "$CYNIC_ROOT"
            git worktree remove "$WORKTREE_DIR" --force 2>/dev/null || true
        else
            AHEAD=$(git log --oneline "origin/main..$BRANCH" 2>/dev/null | wc -l)
            if [ "$AHEAD" -eq 0 ]; then
                echo "Worktree cleanup: no commits ahead, removing." >&2
                cd "$CYNIC_ROOT"
                git worktree remove "$WORKTREE_DIR" --force 2>/dev/null || true
            else
                echo "⚠ Worktree has $AHEAD unpushed commits — preserving." >&2
            fi
        fi
    fi
fi
```

- [ ] **Step 2: Commit**

```bash
git add .cortex/mcp/session-stop.sh
git commit -m "feat(coord): session-stop worktree cleanup

Auto-removes worktree on session end if branch is pushed or has
no commits. Preserves worktrees with uncommitted/unpushed work
for 24h (cleaned by cynic-cortex-cleanup.sh cron).
Agent Protocol v1 Component 1b."
```

---

## Task 8: Integration Smoke Test

- [ ] **Step 1: Run full `make check`**

Run: `make check`
Expected: all gates pass.

- [ ] **Step 2: Manual end-to-end test**

```bash
# Terminal 1: launch cortex via wrapper
CYNIC_ROOT=$(pwd) ./scripts/cynic-cortex.sh

# Inside Claude Code: edit a file, verify claim fires
# Push, verify pre-push checks run

# Terminal 2: launch second cortex
CYNIC_ROOT=$(pwd) ./scripts/cynic-cortex.sh

# Edit the SAME file — verify conflict warning appears
# Check /coord/who shows both agents + their scopes
```

- [ ] **Step 3: Verify worktree isolation**

```bash
# In terminal 1: create a file, don't commit
# In terminal 2: verify the file does NOT exist
# This proves worktree isolation works
```

- [ ] **Step 4: Final commit + push + PR**

```bash
git push -u origin HEAD
gh pr create --base main --title "feat: Agent Protocol v1 — worktree isolation + extended claims + PROPOSE gate"
```

---

## Execution Order & Dependencies

```
Task 1 (wrapper)     ─── independent, can start immediately
Task 2 (schema)      ─── independent, can start immediately
Task 3 (expiry)      ─── depends on Task 2
Task 4 (claim 200)   ─── independent, can start immediately
Task 5 (scope remove) ── depends on Task 4 (new claim behavior must be in place first)
Task 6 (pre-push)    ─── independent, can start immediately
Task 7 (cleanup)     ─── depends on Task 1 (worktree must exist)
Task 8 (smoke test)  ─── depends on all
```

**Parallelizable:** Tasks 1, 2, 4, 6 can run in parallel. Tasks 3, 5, 7 are sequential after their dependencies. Task 8 is last.
