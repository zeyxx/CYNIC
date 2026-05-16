#!/usr/bin/env bash
# CYNIC — PreToolUse hook: zone-level coordination gate
# Resolves file → zone from zones.json, claims zone, BLOCKS on zone conflict.
# K15 consumer: mempool dispatches + coord/claim → hard gate on Edit/Write.
set -euo pipefail

INPUT=$(cat)

block() {
    echo "BLOCKED: $*" >&2
    exit 2
}

is_truthy() {
    case "${1:-}" in
        1|true|TRUE|yes|YES) return 0 ;;
        *) return 1 ;;
    esac
}

coord_unavailable() {
    local reason="$1"
    if is_truthy "${CYNIC_COORD_ALLOW_DEGRADED:-}"; then
        echo "WARN: degraded coordination override — $reason" >&2
        exit 0
    fi
    block "$reason"
}

if ! command -v jq &>/dev/null; then
    block "jq is required for coord-claim.sh"
fi

FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')

source ~/.cynic-env 2>/dev/null || true
KERNEL_ADDR="${CYNIC_REST_ADDR:-127.0.0.1:3030}"
API_KEY="${CYNIC_API_KEY:-}"

AGENT_ID=""
if [[ -n "$SESSION_ID" ]]; then
    AGENT_ID="claude-${SESSION_ID:0:12}"
else
    SESSION_STATE_DIR="/tmp/cynic-sessions"
    if [[ -d "$SESSION_STATE_DIR" ]]; then
        RECENT_STATE=$(ls -t "$SESSION_STATE_DIR"/*.state 2>/dev/null | head -1)
        if [[ -n "$RECENT_STATE" ]]; then
            AGENT_ID=$(grep -oP 'agent_id=\K[^ ]+' "$RECENT_STATE" 2>/dev/null || true)
        fi
    fi
fi

if [[ -z "$AGENT_ID" ]]; then
    block "coordination requires valid AGENT_ID (SESSION_ID missing and no session state file found)"
fi

# ── Zone resolution: file path → zone name ──
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
ZONES_FILE="${PROJECT_DIR}/.claude/zones.json"
ZONE=""

if [[ -f "$ZONES_FILE" ]]; then
    # Make path relative to project root
    REL_PATH="${FILE_PATH#$PROJECT_DIR/}"

    # Match file to zone (longest prefix wins)
    ZONE=$(jq -r --arg path "$REL_PATH" '
        .zones | to_entries | map(
            select(.value.paths[] as $p | $path | startswith($p))
        ) | sort_by(.value.paths[0] | length) | reverse | .[0].key // ""
    ' "$ZONES_FILE" 2>/dev/null || echo "")
fi

# ── Zone conflict check (local, fast — no kernel round-trip) ──
ZONE_STATE_DIR="/tmp/cynic-zones"
mkdir -p "$ZONE_STATE_DIR"

if [[ -n "$ZONE" ]]; then
    ZONE_LOCK="${ZONE_STATE_DIR}/${ZONE}.claimed"

    if [[ -f "$ZONE_LOCK" ]]; then
        CLAIMED_BY=$(cat "$ZONE_LOCK" 2>/dev/null || echo "")
        if [[ -n "$CLAIMED_BY" && "$CLAIMED_BY" != "$AGENT_ID" ]]; then
            # Ask kernel: is the claimant still alive?
            ALIVE="false"
            if [[ -n "$API_KEY" ]]; then
                WHO_RESPONSE=$(curl -s --connect-timeout 2 --max-time 3 \
                    "http://${KERNEL_ADDR}/coord/who" \
                    -H "Authorization: Bearer $API_KEY" 2>/dev/null || echo "")
                if echo "$WHO_RESPONSE" | jq -e --arg id "$CLAIMED_BY" '.agents[]? | select(.agent_id == $id)' > /dev/null 2>&1; then
                    ALIVE="true"
                fi
            fi
            if [[ "$ALIVE" == "true" ]]; then
                block "ZONE CONFLICT: '${ZONE}' is claimed by ${CLAIMED_BY} (live). File: ${FILE_PATH##*/}. Wait or ask T. to reassign."
            else
                echo "WARN: zone '${ZONE}' claimed by dead session ${CLAIMED_BY} — reclaiming" >&2
            fi
        fi
    fi

    # Claim the zone for this agent
    echo "$AGENT_ID" > "$ZONE_LOCK"
fi

# ── Kernel-level file claim (secondary signal, requires API key + kernel) ──
if [[ -n "$API_KEY" ]]; then
    TARGET_FILE="${FILE_PATH#*cynic-kernel/src/}"
    CLAIM_TMP=$(mktemp /tmp/cynic-claim-XXXXXX)
    trap "rm -f '$CLAIM_TMP'" EXIT
    HTTP_CODE=$(curl -s -o "$CLAIM_TMP" -w '%{http_code}' \
        --connect-timeout 2 --max-time 3 \
        -X POST "http://${KERNEL_ADDR}/coord/claim" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d "{\"agent_id\":\"${AGENT_ID}\",\"target\":\"${TARGET_FILE}\",\"claim_type\":\"file\"}" \
        2>/dev/null || echo "000")

    if [[ "$HTTP_CODE" == "200" ]]; then
        CONFLICTS=$(jq -r '.conflicts // empty' "$CLAIM_TMP" 2>/dev/null)
        if [ -n "$CONFLICTS" ] && [ "$CONFLICTS" != "null" ] && [ "$CONFLICTS" != "[]" ]; then
            CONFLICT_AGENTS=$(jq -r '.conflicts[].agent_id' "$CLAIM_TMP" 2>/dev/null | tr '\n' ', ')
            echo "⚠ FILE OVERLAP: '$TARGET_FILE' also claimed by: ${CONFLICT_AGENTS%,}" >&2
        fi
    fi
fi

exit 0
