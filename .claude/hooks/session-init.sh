#!/usr/bin/env bash
# CYNIC — SessionStart hook
# Verifies environment, auto-registers agent, injects critical context.
# This is the pipeline entry point — every session starts clean.
set -euo pipefail

INPUT=$(cat)
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"

# Source env for variable resolution
source ~/.cynic-env 2>/dev/null || true

# ── Verify working directory ──
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')
if [[ -n "$CWD" && "$CWD" != "$PROJECT_DIR" ]]; then
    echo "WARNING: CWD is $CWD — expected $PROJECT_DIR" >&2
fi

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AUTH_HEADER=""
[ -n "$API_KEY" ] && AUTH_HEADER="Authorization: Bearer $API_KEY"

# ── Quick health probe (authenticated — HTTP 200=sovereign, 503=degraded) ──
HEALTH_CODE=$(curl -s -o /dev/null -w '%{http_code}' --max-time 5 \
    ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
    "http://${KERNEL_ADDR}/health" 2>/dev/null || echo "000")

if [[ "$HEALTH_CODE" == "200" ]]; then
    KERNEL_STATUS="sovereign"
elif [[ "$HEALTH_CODE" == "503" ]]; then
    KERNEL_STATUS="degraded"
else
    KERNEL_STATUS="down"
fi

# ── Dog count from authenticated response ──
EXPECTED_DOGS=0
BACKENDS_FILE="${HOME}/.config/cynic/backends.toml"
if [[ -f "$BACKENDS_FILE" ]]; then
    # deterministic-dog is in-kernel (no backends.toml entry) but the /health
    # endpoint already includes it in the dogs array, so no +1 needed here.
    EXPECTED_DOGS=$(grep -c '^\[backend\.' "$BACKENDS_FILE" || echo 0)
fi
# Count dogs from authenticated /health (dogs array length)
ACTIVE_DOGS=0
if [[ "$KERNEL_STATUS" != "down" ]]; then
    ACTIVE_DOGS=$(curl -s --max-time 5 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/health" 2>/dev/null \
        | jq '.dogs | length' 2>/dev/null || echo 0)
fi
DOG_DRIFT=""
if [[ "$EXPECTED_DOGS" -gt 0 && "$ACTIVE_DOGS" -lt "$EXPECTED_DOGS" ]]; then
    DOG_DRIFT="WARNING: ${ACTIVE_DOGS}/${EXPECTED_DOGS} Dogs active — check backend health"
fi

SURREAL_STATUS="down"
if surreal is-ready --endpoint http://localhost:8000 2>/dev/null; then
    SURREAL_STATUS="ok"
fi

GIT_BRANCH=$(git -C "$PROJECT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git -C "$PROJECT_DIR" status --porcelain 2>/dev/null | wc -l)

# ── Auto-prune: clean up local branches whose remote was deleted (post-merge) ──
git -C "$PROJECT_DIR" fetch --prune origin 2>/dev/null || true
GONE_BRANCHES=$(git -C "$PROJECT_DIR" branch -vv 2>/dev/null | grep '\[gone\]' | awk '{print $1}' || true)
PRUNED_COUNT=0
if [[ -n "$GONE_BRANCHES" ]]; then
    while IFS= read -r gone; do
        [[ -z "$gone" ]] && continue
        git -C "$PROJECT_DIR" branch -D "$gone" 2>/dev/null && PRUNED_COUNT=$((PRUNED_COUNT+1)) || true
    done <<< "$GONE_BRANCHES"
fi

# ── K15: Capture AT_START cortex proof (multi-cortex coordination) ──
# Prevents divergence: captures git state at session start for next session to verify continuity.
# Stored in .claude/session-proof.json (git-tracked), also sent to kernel /observe as fallback.
PROOF_FILE="${PROJECT_DIR}/.claude/session-proof.json"
LAST_COMMIT=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")
ORIGIN_COMMIT=$(git -C "$PROJECT_DIR" rev-parse --short origin/main 2>/dev/null || echo "unknown")
ORIGIN_AHEAD=$(git -C "$PROJECT_DIR" rev-list HEAD..origin/main --count 2>/dev/null || echo 0)
ORIGIN_BEHIND=$(git -C "$PROJECT_DIR" rev-list origin/main..HEAD --count 2>/dev/null || echo 0)

# Collect open LOCAL branches only (exclude HEAD, exclude remote branches)
# Filter: only lines that DON'T start with 'remotes/' (git branch --list shows local only, but be explicit)
OPEN_BRANCHES=$(git -C "$PROJECT_DIR" branch --list 2>/dev/null | grep -v '^\*' | sed 's/^[[:space:]]*//' | grep -v '^remotes/' | jq -R -s -c 'split("\n") | map(select(length > 0))' || echo "[]")

# Collect open PRs (if gh is available)
OPEN_PRS="[]"
if command -v gh &>/dev/null && [[ "$KERNEL_STATUS" != "down" ]]; then
    OPEN_PRS=$(gh pr list --json number,title,headRefName,isDraft,state 2>/dev/null | jq -c '.' || echo "[]")
fi

# Collect stashes
STASHES=$(git -C "$PROJECT_DIR" stash list 2>/dev/null | jq -R -s -c 'split("\n") | map(select(length > 0))' || echo "[]")

# MC4 claimed modules extraction: REMOVED (jq path was broken — .agents[]?.claims[]?
# never resolved because claims are a separate array in CoordSnapshot, not nested in agents).
# Branch isolation at session-init replaces edit-time claim checking.
CLAIMED_MODULES="[]"

# Agent ID from Claude session_id (stable across compactions) ──
# MUST be computed early — used in PROOF_FILE heredoc below.
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    AGENT_ID="claude-$(date +%s)"
fi

# NOW_TS — used in stash age calculation below
NOW_TS=$(date +%s)

# Write AT_START proof (includes coordination state for MC4 blocking)
cat > "$PROOF_FILE" << PROOF_EOF
{
  "session_id": "${AGENT_ID}",
  "schema_version": "1.1",
  "recorded_at": "$(date -u +'%Y-%m-%dT%H:%M:%SZ')",
  "AT_START": {
    "branch_name": "${GIT_BRANCH}",
    "last_commit_hash": "${LAST_COMMIT}",
    "origin_commit_hash": "${ORIGIN_COMMIT}",
    "local_behind_origin": ${ORIGIN_AHEAD},
    "local_ahead_of_origin": ${ORIGIN_BEHIND},
    "dirty_files": ${GIT_DIRTY},
    "open_branches": ${OPEN_BRANCHES},
    "open_prs": ${OPEN_PRS},
    "stashes": ${STASHES},
    "claimed_by_active_agents": ${CLAIMED_MODULES},
    "kernel_status": "${KERNEL_STATUS}",
    "dogs_active": ${ACTIVE_DOGS}
  }
}
PROOF_EOF

# ── Git Hygiene Sense: emit raw metrics to /observe (data-centric, no thresholds) ──
# K15 consumer: CCM crystallization pipeline learns git hygiene patterns over time.
# All values are integers; all derived from data already collected above.
BRANCHES_LOCAL=$(echo "$OPEN_BRANCHES" | jq 'length')
BRANCHES_MERGED=$(git -C "$PROJECT_DIR" branch --merged main 2>/dev/null | grep -v '^\*' | grep -v 'main' | wc -l || echo 0)
BRANCHES_UNPUSHED=$(git -C "$PROJECT_DIR" branch -vv 2>/dev/null | grep -v '\[origin/' | grep -v '^\*' | wc -l || echo 0)
STASHES_TOTAL=$(echo "$STASHES" | jq 'length')
OPEN_PRS_COUNT=$(echo "$OPEN_PRS" | jq 'length')

# Stash max age: parse oldest stash timestamp
STASH_MAX_AGE_DAYS=0
if [[ "$STASHES_TOTAL" -gt 0 ]]; then
    OLDEST_STASH_TS=$(git -C "$PROJECT_DIR" stash list --date=unix 2>/dev/null | tail -1 | grep -oP '@\{\K[0-9]+' || echo 0)
    if [[ "$OLDEST_STASH_TS" -gt 0 ]]; then
        STASH_MAX_AGE_DAYS=$(( (NOW_TS - OLDEST_STASH_TS) / 86400 ))
    fi
fi

# Stale PRs: open PRs with no update in >3 days
STALE_PRS=0
if [[ "$OPEN_PRS_COUNT" -gt 0 ]]; then
    THREE_DAYS_AGO=$(date -u -d '3 days ago' +'%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || echo "")
    if [[ -n "$THREE_DAYS_AGO" ]]; then
        STALE_PRS=$(gh pr list --json updatedAt 2>/dev/null | jq "[.[] | select(.updatedAt < \"${THREE_DAYS_AGO}\")] | length" 2>/dev/null || echo 0)
    fi
fi

# Git hygiene context string (POST deferred until after AGENT_ID is computed)
GIT_HYGIENE_CTX="branches_local=${BRANCHES_LOCAL} branches_merged=${BRANCHES_MERGED} branches_unpushed=${BRANCHES_UNPUSHED} stashes_total=${STASHES_TOTAL} stash_max_age_days=${STASH_MAX_AGE_DAYS} open_prs=${OPEN_PRS_COUNT} stale_prs=${STALE_PRS} pruned=${PRUNED_COUNT}"

# Check for violations from last session (AT_END that became AT_START of this session)
# Rule: violation = work that could be lost or coordination debt from previous session
GIT_VIOLATIONS=""

# V1: If on main AND have unpushed commits → violation (could lose work on main)
if [[ "${GIT_BRANCH}" == "main" && ${ORIGIN_BEHIND} -gt 0 ]]; then
    GIT_VIOLATIONS="On main with ${ORIGIN_BEHIND} unpushed commits (risk: could be lost)"
fi

# V2: Multiple open branches → coordination debt (MC2 violation: PR before new work)
OPEN_BRANCH_COUNT=$(echo "$OPEN_BRANCHES" | jq 'length')
if [[ ${OPEN_BRANCH_COUNT} -gt 1 ]]; then
    GIT_VIOLATIONS="${GIT_VIOLATIONS:+$GIT_VIOLATIONS; }${OPEN_BRANCH_COUNT} branches remain open (MC2: review if PRs are merged)"
fi

# V3: Stale stashes → incomplete work from previous session
if [[ $(echo "$STASHES" | jq 'length') -gt 0 ]]; then
    GIT_VIOLATIONS="${GIT_VIOLATIONS:+$GIT_VIOLATIONS; }$(echo "$STASHES" | jq 'length') stashes exist (previous session work not completed)"
fi

# ── Git Hygiene Sense: POST to /observe (fire-and-forget, non-blocking) ──
# Deferred from metric computation above — needs AGENT_ID which is derived from session_id.
if [[ "$KERNEL_STATUS" != "down" ]]; then
    curl -s --connect-timeout 2 --max-time 3 -X POST "http://${KERNEL_ADDR}/observe" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"tool\":\"git_hygiene_sense\",\"target\":\"session_start\",\"domain\":\"git-hygiene\",\"context\":\"${GIT_HYGIENE_CTX}\",\"tags\":[\"git-hygiene\",\"sense\"],\"consumer\":\"ccm\",\"action\":\"crystallize git hygiene patterns\"}" \
        > /dev/null 2>&1 &
fi

# ── Session cost tracking (Rule 3: measure baseline) ──
SESSION_STATE_DIR="/tmp/cynic-sessions"
mkdir -p "$SESSION_STATE_DIR"
SESSION_STATE_FILE="${SESSION_STATE_DIR}/${AGENT_ID}.state"
cat > "$SESSION_STATE_FILE" << STATE_EOF
agent_id=${AGENT_ID}
session_start=$(date +%s)
project_dir=${PROJECT_DIR}
STATE_EOF

# ── Tier 1: Mechanical branch isolation (local git, zero dependencies) ──
# Creates a unique branch BEFORE the LLM receives its first message.
# The LLM cannot forget to branch — it's already on one.
CURRENT_BRANCH=$(git -C "$PROJECT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
CORTEX_BRANCH="$CURRENT_BRANCH"

if [[ "$CURRENT_BRANCH" == "main" || "$CURRENT_BRANCH" == "master" ]]; then
    CORTEX_BRANCH="cortex/${AGENT_ID}-$(date +%Y-%m-%d)-$(head -c4 /dev/urandom | xxd -p)"
    git -C "$PROJECT_DIR" checkout -b "$CORTEX_BRANCH" 2>/dev/null || true
    echo "AUTO-BRANCH: created ${CORTEX_BRANCH} (mechanical isolation)"
fi

# ── Auto-register agent via REST + branch collision check ──
# Tier 2: kernel enrichment. If kernel is down, Tier 1 (auto-branch) already covers.
REGISTER_STATUS="skipped"
if [[ "$KERNEL_STATUS" != "down" ]]; then
    # Register with branch in scope
    REGISTER_RESPONSE=$(curl -s --max-time 3 -X POST "http://${KERNEL_ADDR}/coord/register" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"intent\":\"claude-code session\",\"agent_type\":\"claude\",\"scope\":\"branch:${CORTEX_BRANCH}\"}" \
        2>/dev/null || echo '{}')
    if echo "$REGISTER_RESPONSE" | jq -e '.status == "registered"' > /dev/null 2>&1; then
        REGISTER_STATUS="registered"
    else
        REGISTER_STATUS="failed"
    fi
fi

# Mask real IP — session context must never contain real IPs
[ -n "${CYNIC_REST_ADDR:-}" ] && ADDR_STATUS="SET" || ADDR_STATUS="NOT SET"

# ── Temporal Consciousness: Chronos/Kairos/Aion anchors ──
CURRENT_HOUR=$(date +%H)
CURRENT_DAY=$(date +%A)
CURRENT_DATE=$(date +%Y-%m-%d)

# Peak hours: 19-22h observed (user behavioral data)
PEAK_HOURS="false"
if [[ "$CURRENT_HOUR" -ge 19 && "$CURRENT_HOUR" -le 22 ]]; then
    PEAK_HOURS="true"
fi

# Gap since last session (from session state files)
LAST_SESSION_TS=0
LATEST_STATE=$(ls -t "${SESSION_STATE_DIR}"/*.state 2>/dev/null | head -1 || true)
if [[ -n "$LATEST_STATE" && -f "$LATEST_STATE" ]]; then
    LAST_SESSION_TS=$(grep '^session_start=' "$LATEST_STATE" | cut -d= -f2 || echo 0)
fi
# NOW_TS already defined earlier (line ~115)
if [[ "$LAST_SESSION_TS" -gt 0 ]]; then
    GAP_HOURS=$(( (NOW_TS - LAST_SESSION_TS) / 3600 ))
else
    GAP_HOURS="unknown"
fi

# Known deadlines
TALARIA_ICO_CLOSE="2026-06-09"
DAYS_TO_ICO=$(( ( $(date -d "$TALARIA_ICO_CLOSE" +%s 2>/dev/null || echo "$NOW_TS") - NOW_TS ) / 86400 ))
if [[ "$DAYS_TO_ICO" -lt 0 ]]; then
    DAYS_TO_ICO="past"
fi

# ── Mempool scan: query items from kernel observations ──
MEMPOOL_RIPE=""
MEMPOOL_COUNT=0
if [[ "$KERNEL_STATUS" != "down" ]]; then
    MEMPOOL_OBS=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/observations?domain=mempool&limit=10" 2>/dev/null)
    if [[ -n "$MEMPOOL_OBS" ]] && echo "$MEMPOOL_OBS" | jq -e 'type == "array" and length > 0' >/dev/null 2>&1; then
        MEMPOOL_COUNT=$(echo "$MEMPOOL_OBS" | jq 'length')
        MEMPOOL_RIPE=$(echo "$MEMPOOL_OBS" | jq -r '.[] | "  → \(.target // "?") — \(.context // "no context" | .[0:100])"' 2>/dev/null | head -5 || true)
    fi
fi

# ── Generate session-agenda.json (versioned, timestamped — replaces TODO.md tactical layer) ──
# Schema v1: ripe (actionable), blocked, in_flight (PRs), done_recent (git log)
# Source of truth: mempool API + gh pr + git log — never hand-edited.
AGENDA_FILE="${PROJECT_DIR}/.claude/session-agenda.json"
DONE_RECENT=$(git -C "$PROJECT_DIR" log --oneline -7 \
    --pretty=format:'{"sha":"%h","msg":"%s","date":"%as"}' 2>/dev/null \
    | jq -s '.' 2>/dev/null || echo "[]")

# Dedup by target (latest per target wins). Filter: dispatch noise, agent-ID targets, MINED items.
if [[ -n "$MEMPOOL_OBS" ]] && echo "$MEMPOOL_OBS" | jq -e 'type == "array"' >/dev/null 2>&1; then
    AGENDA_RIPE=$(echo "$MEMPOOL_OBS" | jq -c '
      group_by(.target) | map(sort_by(.created_at) | last) |
      map(select(
        ((.tags // []) | (contains(["MINED"]) or contains(["BLOCKED"]) or contains(["dispatch"])) | not) and
        (.target | startswith("claude-") | not)
      )) |
      map({id: (.id // .target), context: (.context // "" | .[0:120]), tags: (.tags // []), age_hours: 0})
    ' 2>/dev/null || echo "[]")
    AGENDA_BLOCKED=$(echo "$MEMPOOL_OBS" | jq -c '
      group_by(.target) | map(sort_by(.created_at) | last) |
      map(select(
        ((.tags // []) | contains(["BLOCKED"])) and
        (.target | startswith("claude-") | not)
      )) |
      map({id: (.id // .target), context: (.context // "" | .[0:120]), tags: (.tags // [])})
    ' 2>/dev/null || echo "[]")
else
    AGENDA_RIPE="[]"
    AGENDA_BLOCKED="[]"
fi

IN_FLIGHT=$(echo "$OPEN_PRS" | jq -c \
    '[.[] | {pr: .number, title: .title, branch: .headRefName, draft: (.isDraft // false)}]' \
    2>/dev/null || echo "[]")

cat > "$AGENDA_FILE" << AGENDA_EOF
{
  "v": 1,
  "generated_at": "$(date -u +'%Y-%m-%dT%H:%M:%SZ')",
  "ripe": ${AGENDA_RIPE},
  "blocked": ${AGENDA_BLOCKED},
  "in_flight": ${IN_FLIGHT},
  "done_recent": ${DONE_RECENT},
  "strategy_ref": "TODO.md (phases/milestones only — not tactical)"
}
AGENDA_EOF

# ── Output context (injected into conversation) ──
cat <<EOF
CYNIC SESSION — Pipeline initialized.
Kernel: ${KERNEL_STATUS} | DB: ${SURREAL_STATUS} | Git: ${GIT_BRANCH} (${GIT_DIRTY} dirty files)
Dogs: ${ACTIVE_DOGS}/${EXPECTED_DOGS}${DOG_DRIFT:+ — $DOG_DRIFT}
Env: CYNIC_REST_ADDR=${ADDR_STATUS}
Agent: ${AGENT_ID} (${REGISTER_STATUS})

WORKFLOW: Use /build after edits, /deploy for production, /status for full dashboard.
COORD: Agent auto-registered. Claim → cynic_coord_who + cynic_coord_claim | Release → cynic_coord_release
RULES: Public repo — no secrets, no real IPs, no names. Use skills before acting.
TEMPORAL: ${CURRENT_DATE} ${CURRENT_DAY} ${CURRENT_HOUR}h | Gap: ${GAP_HOURS}h | Peak: ${PEAK_HOURS} | Talaria ICO close: J-${DAYS_TO_ICO}
EOF

# ── Mempool injection (temporal consciousness) ──
if [[ "$MEMPOOL_COUNT" -gt 0 ]]; then
    echo ""
    echo "MEMPOOL (${MEMPOOL_COUNT} items):"
    echo "$MEMPOOL_RIPE"
fi

# K15: Warn about git state violations from previous session
if [[ -n "$GIT_VIOLATIONS" ]]; then
    echo ""
    echo "⚠ GIT STATE VIOLATIONS (Rule MC2/MC5 — coordinate before proceeding):"
    echo "  $GIT_VIOLATIONS"
    echo "  → Review .claude/session-proof.json for previous session state"
fi

# K15: MC4 — Warn about module claims from other active agents
CLAIMED_COUNT=$(echo "$CLAIMED_MODULES" | jq 'length')
if [[ ${CLAIMED_COUNT} -gt 0 ]]; then
    echo ""
    echo "⚠ ACTIVE MODULE CLAIMS (Rule MC4 — other cortices are editing):"
    echo "$CLAIMED_MODULES" | jq -r '.[] | "  → \(.)"' 2>/dev/null | head -10 || true
    echo "  Before editing these modules, verify other sessions are done (check /coord/who)."
fi

# ── Zone awareness: inject claimed zones so cortex self-partitions ──
# Agnostic: reads /tmp/cynic-zones/ (shared across Claude/Gemini/Codex).
# The cortex sees what's taken and avoids it. Hard gate in coord-claim.sh catches mistakes.
ZONE_STATE_DIR="/tmp/cynic-zones"
ZONES_FILE="${PROJECT_DIR}/.claude/zones.json"
if [[ -d "$ZONE_STATE_DIR" ]] && ls "$ZONE_STATE_DIR"/*.claimed &>/dev/null; then
    echo ""
    echo "ZONE CLAIMS (hard gate — Edit/Write blocked on conflict):"
    for lock in "$ZONE_STATE_DIR"/*.claimed; do
        [[ -f "$lock" ]] || continue
        ZONE_NAME=$(basename "$lock" .claimed)
        CLAIMED_BY=$(cat "$lock" 2>/dev/null || echo "?")
        if [[ "$CLAIMED_BY" == "$AGENT_ID" ]]; then
            printf "  %-20s → %s (YOU)\n" "$ZONE_NAME" "$CLAIMED_BY"
        else
            printf "  %-20s → %s ← BLOCKED for you\n" "$ZONE_NAME" "$CLAIMED_BY"
        fi
    done
    # Show free zones
    if [[ -f "$ZONES_FILE" ]]; then
        FREE_ZONES=$(jq -r '.zones | keys[]' "$ZONES_FILE" 2>/dev/null | while read z; do
            [[ ! -f "$ZONE_STATE_DIR/${z}.claimed" ]] && echo -n "$z "
        done)
        [[ -n "$FREE_ZONES" ]] && echo "  FREE: $FREE_ZONES"
    fi
fi

# ── Last session compliance (K15: session-stop produces, this consumes) ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    LAST_COMPLIANCE=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/compliance?limit=1" 2>/dev/null)
    if [[ -n "$LAST_COMPLIANCE" ]] && echo "$LAST_COMPLIANCE" | jq -e 'type == "array" and length > 0' >/dev/null 2>&1; then
        LAST_SCORE=$(echo "$LAST_COMPLIANCE" | jq -r '.[0].score // empty' 2>/dev/null)
        LAST_AGENT=$(echo "$LAST_COMPLIANCE" | jq -r '.[0].agent_id // empty' 2>/dev/null)
        if [[ -n "$LAST_SCORE" ]]; then
            # φ⁻² = 0.382 — below this, compliance is degraded
            BELOW_THRESHOLD=$(LC_ALL=C awk "BEGIN {print ($LAST_SCORE < 0.382) ? 1 : 0}" 2>/dev/null || echo 0)
            if [[ "$BELOW_THRESHOLD" == "1" ]]; then
                echo ""
                echo "COMPLIANCE: Last session (${LAST_AGENT}) scored ${LAST_SCORE}/0.618 — BELOW φ⁻² threshold. Review warnings."
            fi
        fi
    fi
fi

# ── Handoff injection (inter-agent context bus) ──
HANDOFF_FILE="${PROJECT_DIR}/.handoff.md"
if [[ -f "$HANDOFF_FILE" ]]; then
    # Inject last agent's handoff entry (most recent ## section)
    LAST_HANDOFF=$(awk '/^## \[/{found++; if(found==2) exit} found{print}' "$HANDOFF_FILE" | head -30)
    if [[ -n "$LAST_HANDOFF" ]]; then
        echo ""
        echo "LAST HANDOFF:"
        echo "$LAST_HANDOFF"
    fi
fi

# ── Agenda injection (from session-agenda.json — live sources, not TODO.md) ──
if [[ -f "$AGENDA_FILE" ]]; then
    RIPE_COUNT=$(jq '.ripe | length' "$AGENDA_FILE" 2>/dev/null || echo 0)
    BLOCKED_COUNT=$(jq '.blocked | length' "$AGENDA_FILE" 2>/dev/null || echo 0)
    IN_FLIGHT_COUNT=$(jq '.in_flight | length' "$AGENDA_FILE" 2>/dev/null || echo 0)

    if [[ "$RIPE_COUNT" -gt 0 || "$IN_FLIGHT_COUNT" -gt 0 ]]; then
        echo ""
        echo "AGENDA (generated from mempool + PRs + git — see .claude/session-agenda.json):"
        if [[ "$RIPE_COUNT" -gt 0 ]]; then
            echo "  RIPE (${RIPE_COUNT}):"
            jq -r '.ripe[] | "    → \(.context | .[0:100])"' "$AGENDA_FILE" 2>/dev/null | head -5
        fi
        if [[ "$IN_FLIGHT_COUNT" -gt 0 ]]; then
            echo "  IN FLIGHT (${IN_FLIGHT_COUNT} PRs):"
            jq -r '.in_flight[] | "    PR#\(.pr) \(if .draft then "[draft] " else "" end)\(.title | .[0:70])"' \
                "$AGENDA_FILE" 2>/dev/null | head -5
        fi
        if [[ "$BLOCKED_COUNT" -gt 0 ]]; then
            echo "  BLOCKED (${BLOCKED_COUNT}):"
            jq -r '.blocked[] | "    ⊘ \(.context | .[0:100])"' "$AGENDA_FILE" 2>/dev/null | head -3
        fi
    fi
fi

# ── Dream consolidation check (K15: dream-trigger produces, this consumes) ──
# When threshold met, output DREAM_REQUIRED directive.
# Workflow rule: LLM must dispatch dream-consolidator agent in background.
DREAM_STATE="${HOME}/.claude/projects/-home-user-Bureau-CYNIC/memory/.dream-state"
if [[ -f "$DREAM_STATE" ]]; then
    SESSIONS_SINCE=$(grep '^sessions_since=' "$DREAM_STATE" | cut -d= -f2 || echo 0)
    if [[ "$SESSIONS_SINCE" -ge 5 ]]; then
        echo ""
        echo "DREAM_REQUIRED: ${SESSIONS_SINCE} sessions since last consolidation — dispatch dream-consolidator agent in background"
    fi
fi

# ── Active sessions + branch collision check (single /coord/who call) ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    WHO_JSON=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/coord/who" 2>/dev/null || echo '{}')
    ACTIVE_AGENTS=$(echo "$WHO_JSON" | jq '[.agents[]? | select(.active == true)] | length' 2>/dev/null || echo 0)

    # Branch collision: if another active session is on the same branch, auto-branch away
    if [[ "$CORTEX_BRANCH" != "unknown" ]]; then
        BRANCH_COLLISION=$(echo "$WHO_JSON" | jq -r --arg branch "$CORTEX_BRANCH" --arg self "$AGENT_ID" \
            '[.agents[]? | select(.active == true and .agent_id != $self and (.scope // "" | contains($branch)))] | length' \
            2>/dev/null || echo "0")
        if [[ "$BRANCH_COLLISION" -gt 0 ]]; then
            OLD_BRANCH="$CORTEX_BRANCH"
            CORTEX_BRANCH="cortex/${AGENT_ID}-$(date +%Y-%m-%d)-$(head -c4 /dev/urandom | xxd -p)"
            git -C "$PROJECT_DIR" checkout -b "$CORTEX_BRANCH" 2>/dev/null || true
            echo "BRANCH COLLISION on ${OLD_BRANCH}: switched to ${CORTEX_BRANCH}"
        fi
    fi

    if [[ "$ACTIVE_AGENTS" -gt 1 ]]; then
        echo ""
        echo "CONCURRENT SESSIONS: ${ACTIVE_AGENTS} active cortex (each on its own branch)."
        echo "$WHO_JSON" | jq -r '.agents[]? | select(.active == true) |
            "  → " + .agent_id +
            (if (.scope // "") != "" then " [" + .scope[0:80] + "]" else "" end)
        ' 2>/dev/null | head -5 || true
    fi
fi

# ── Inter-agent bus: read prior session distillations (domain=session) ──
# Cortex POST rich handoffs (up to 2000 chars) at session end via self-distill.
# Reading them here gives continuity across agents and sessions.
if [[ "$KERNEL_STATUS" != "down" ]]; then
    SESSION_OBS=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/observations?domain=session&limit=20" 2>/dev/null)
    if [[ -n "$SESSION_OBS" ]] && echo "$SESSION_OBS" | jq -e 'type == "array" and length > 0' >/dev/null 2>&1; then
        # Extract NEXT + BLOCKED from the most recent session_distill (target=handover)
        LAST_CTX=$(echo "$SESSION_OBS" | jq -r '[.[] | select(.target == "handover")] | .[0].context // ""' 2>/dev/null)
        LAST_TS=$(echo "$SESSION_OBS" | jq -r '[.[] | select(.target == "handover")] | .[0].created_at // ""' 2>/dev/null)
        if [[ -n "$LAST_CTX" && -n "$LAST_TS" ]]; then
            LAST_EPOCH=$(date -d "$LAST_TS" +%s 2>/dev/null || echo 0)
            HANDOFF_AGE_H=$(( (NOW_TS - LAST_EPOCH) / 3600 ))
            if [[ "$HANDOFF_AGE_H" -ge 48 ]]; then
                HANDOFF_LABEL="${LAST_TS:0:10} — $(( HANDOFF_AGE_H / 24 ))d ago ⚠ may be stale"
            elif [[ "$HANDOFF_AGE_H" -ge 1 ]]; then
                HANDOFF_LABEL="${HANDOFF_AGE_H}h ago"
            else
                HANDOFF_LABEL="this session"
            fi
            echo ""
            echo "LAST SESSION HANDOFF (${HANDOFF_LABEL}):"
            echo "$LAST_CTX" | grep -oP '(?<=NEXT: ).*?(?= BLOCKED:|$)' | fold -s -w 100 | sed 's/^/  NEXT: /' || true
            echo "$LAST_CTX" | grep -oP '(?<=BLOCKED: ).*' | fold -s -w 100 | sed 's/^/  BLOCKED: /' || true
        fi
    fi
fi

# ── Inject top CCM crystals as learnings (CYNIC remembers) ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    CRYSTALS=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/crystals?limit=5" 2>/dev/null)
    if [[ -n "$CRYSTALS" ]] && echo "$CRYSTALS" | jq -e 'type == "array" and length > 0' >/dev/null 2>&1; then
        echo ""
        echo "CYNIC MEMORY (top crystallized patterns):"
        echo "$CRYSTALS" | jq -r '.[] | select(.state == "crystallized" or .state == "canonical") | "  [\(.state)] \(.content) (confidence: \(.confidence | tostring | .[0:4]), \(.observations) obs)"' 2>/dev/null | head -5 || true
    fi
fi

# ── Organism topology injection (discoverable infrastructure) ──
TOPOLOGY_FILE="${PROJECT_DIR}/TOPOLOGY.md"
if [[ -f "$TOPOLOGY_FILE" ]]; then
    TOPOLOGY_AGE_DAYS=$(( ( $(date +%s) - $(stat -c %Y "$TOPOLOGY_FILE") ) / 86400 ))
    echo ""
    echo "TOPOLOGY (organism map — ${TOPOLOGY_AGE_DAYS}d old):"
    # Inject Active Modules table only (compact — full file available as TOPOLOGY.md)
    awk '/^## Active Modules/{found=1; next} found && /^\|/{print "  "$0} found && /^$/{exit}' "$TOPOLOGY_FILE" | head -15
    # Inject K15 unfulfilled
    UNFULFILLED=$(awk '/^## Unfulfilled/{found=1; next} found && /^-/{print "  "$0} found && /^$/{exit}' "$TOPOLOGY_FILE")
    if [[ -n "$UNFULFILLED" ]]; then
        echo "  K15 UNFULFILLED:"
        echo "$UNFULFILLED"
    fi
    if [[ "$TOPOLOGY_AGE_DAYS" -gt 7 ]]; then
        echo "  WARNING: TOPOLOGY.md is ${TOPOLOGY_AGE_DAYS} days old — run 'python3 scripts/topology.py' to refresh"
    fi
fi

# ── Domain wisdom injection (high-strength curated signals) ──
CURATION_DIR="${PROJECT_DIR}/cynic-python/curation"
if [[ -d "$CURATION_DIR" ]]; then
    WISDOM_COUNT=0
    WISDOM_OUT=""
    for f in "$CURATION_DIR"/D*_curated.jsonl; do
        [[ -f "$f" ]] || continue
        SIGNAL=$(jq -r 'select(.strength >= 0.8) |
            "[" + .domain + "] " + (.pattern[:80] | gsub("\n";" ")) + " (s=" + (.strength|tostring) + ")"' \
            "$f" 2>/dev/null | head -1 || true)
        [[ -n "$SIGNAL" ]] && WISDOM_OUT="${WISDOM_OUT}  ${SIGNAL}\n" && WISDOM_COUNT=$((WISDOM_COUNT+1))
        [[ $WISDOM_COUNT -ge 3 ]] && break
    done
    if [[ $WISDOM_COUNT -gt 0 ]]; then
        echo ""
        echo "DOMAIN WISDOM (${WISDOM_COUNT} high-strength signals):"
        printf "%b" "$WISDOM_OUT"
    fi
fi
