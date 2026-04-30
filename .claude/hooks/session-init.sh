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
    OPEN_PRS=$(gh pr list --json number,title,headRefName 2>/dev/null | jq -c '.' || echo "[]")
fi

# Collect stashes
STASHES=$(git -C "$PROJECT_DIR" stash list 2>/dev/null | jq -R -s -c 'split("\n") | map(select(length > 0))' || echo "[]")

# K15: Collect claimed modules from active agents (MC4 constraint detection)
# If kernel is down, skip — don't block session start on kernel availability
CLAIMED_MODULES="[]"
if [[ "$KERNEL_STATUS" != "down" ]]; then
    WHO_JSON=$(curl -s --max-time 2 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/coord/who" 2>/dev/null || echo '{}')
    # Extract all claims from all active agents and flatten into single array
    CLAIMED_MODULES=$(echo "$WHO_JSON" | jq -c '[.agents[]? | select(.active == true) | .claims[]? // empty] | unique' 2>/dev/null || echo "[]")
fi

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

# Agent ID from Claude session_id (stable across compactions) ──
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    AGENT_ID="claude-$(date +%s)"
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

# ── Auto-register agent via REST (hard enforcement, not hope) ──
REGISTER_STATUS="skipped"
if [[ "$KERNEL_STATUS" != "down" ]]; then
    REGISTER_RESPONSE=$(curl -s --max-time 3 -X POST "http://${KERNEL_ADDR}/coord/register" \
        -H "Content-Type: application/json" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"intent\":\"claude-code session\",\"agent_type\":\"claude\"}" \
        2>/dev/null || echo '{}')
    if echo "$REGISTER_RESPONSE" | jq -e '.status == "registered"' > /dev/null 2>&1; then
        REGISTER_STATUS="registered"
    else
        REGISTER_STATUS="failed"
    fi
fi

# Mask real IP — session context must never contain real IPs
[ -n "${CYNIC_REST_ADDR:-}" ] && ADDR_STATUS="SET" || ADDR_STATUS="NOT SET"

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
EOF

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

# ── TODO injection (continuity: the Dog knows what needs doing) ──
TODO_FILE="${PROJECT_DIR}/TODO.md"
if [[ -f "$TODO_FILE" ]]; then
    # Extract P1 items (unchecked only)
    P1_ITEMS=$(grep -A1 '## P1' "$TODO_FILE" | grep '^\- \[ \]' | head -5 || true)
    if [[ -n "$P1_ITEMS" ]]; then
        echo ""
        echo "TODO (P1 — do first):"
        echo "$P1_ITEMS"
    fi
    TODO_AGE_DAYS=$(( ( $(date +%s) - $(stat -c %Y "$TODO_FILE") ) / 86400 ))
    if [[ "$TODO_AGE_DAYS" -gt 3 ]]; then
        echo "WARNING: TODO.md last updated ${TODO_AGE_DAYS} days ago — review and update."
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

# ── Active sessions check (MC4: auto-partition with escalade) ──
if [[ "$KERNEL_STATUS" != "down" ]]; then
    WHO_JSON=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/coord/who" 2>/dev/null)
    ACTIVE_AGENTS=$(echo "$WHO_JSON" | jq '[.agents[]? | select(.active == true)] | length' 2>/dev/null || echo 0)
    if [[ "$ACTIVE_AGENTS" -gt 1 ]]; then
        echo ""
        echo "⚠ CONCURRENT SESSIONS: ${ACTIVE_AGENTS} active cortex detected."
        echo "  Rule MC4: check /coord/who before touching shared files."
        echo "$WHO_JSON" | jq -r '.agents[]? | select(.active == true) | "  → \(.agent_id) claiming: \(.claims // [] | join(", "))"' 2>/dev/null | head -5 || true
    fi
fi

# ── Inter-agent bus: read prior session distillations (domain=session) ──
# Cortex POST rich handoffs (up to 2000 chars) at session end via self-distill.
# Reading them here gives continuity across agents and sessions.
if [[ "$KERNEL_STATUS" != "down" ]]; then
    SESSION_OBS=$(curl -s --max-time 3 \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        "http://${KERNEL_ADDR}/observations?domain=session&limit=3" 2>/dev/null)
    if [[ -n "$SESSION_OBS" ]] && echo "$SESSION_OBS" | jq -e 'type == "array" and length > 0' >/dev/null 2>&1; then
        echo ""
        echo "RECENT SESSIONS (inter-agent bus):"
        echo "$SESSION_OBS" | jq -r '.[] | "  [\(.agent_id // "?")] \(.context // "no context") (\(.created_at // "?" | split("T")[0] // "?"))"' 2>/dev/null | head -5 || true
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

# ── Domain wisdom injection (D1-D6 curated signals, high-confidence only) ──
# No REST endpoint for wisdom — read curation files directly from disk.
# Budget: 3 signals max, ~400 chars total. Strength threshold >= 0.8.
CURATION_DIR="${PROJECT_DIR}/cynic-python/curation"
if [[ -d "$CURATION_DIR" ]]; then
    WISDOM_COUNT=0
    WISDOM_OUT=""
    for f in "$CURATION_DIR"/D*_curated.jsonl; do
        [[ -f "$f" ]] || continue
        # Stream each line (JSONL format), filter by strength >= 0.8, take first match
        SIGNAL=$(jq -r 'select(.strength >= 0.8) |
            "[" + .domain + "] " + (.pattern[:80] | gsub("\n"; " ")) + " (strength: " + (.strength | tostring | .[0:4]) + ")"' \
            "$f" 2>/dev/null | head -1 || true)
        if [[ -n "$SIGNAL" ]]; then
            WISDOM_OUT="${WISDOM_OUT}  ${SIGNAL}\n"
            WISDOM_COUNT=$((WISDOM_COUNT + 1))
        fi
        [[ $WISDOM_COUNT -ge 3 ]] && break
    done
    if [[ $WISDOM_COUNT -gt 0 ]]; then
        echo ""
        echo "DOMAIN WISDOM (${WISDOM_COUNT} high-strength signals from D1-D6):"
        printf "%b" "$WISDOM_OUT"
    fi
fi
