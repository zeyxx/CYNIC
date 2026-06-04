#!/usr/bin/env bash
# Post a task item to the CYNIC mempool.
# Usage:
#   mempool-post.sh "target-slug" "Description of what needs doing" [RIPE|BLOCKED|MINED]
#   mempool-post.sh --mined "target-slug" "Why it's done / commit ref"
#
# Examples:
#   mempool-post.sh "wire-telegram-timer" "Add systemd timer for pipeline.py"
#   mempool-post.sh "wire-telegram-timer" "Done in commit abc123" MINED
#   mempool-post.sh --mined "wire-telegram-timer" "Closed by feat/telegram-pipeline PR#284"
#
# The latest observation per target wins. MINED items are hidden from agenda.ripe.
set -euo pipefail

source ~/.cynic-env 2>/dev/null || true

KERNEL_ADDR="${CYNIC_REST_ADDR:-localhost:3030}"
API_KEY="${CYNIC_API_KEY:-}"

if [[ -z "$API_KEY" ]]; then
    echo "ERROR: CYNIC_API_KEY not set" >&2
    exit 1
fi

# Parse --mined shorthand
STATE="RIPE"
if [[ "${1:-}" == "--mined" ]]; then
    STATE="MINED"
    shift
elif [[ "${1:-}" == "--blocked" ]]; then
    STATE="BLOCKED"
    shift
fi

TARGET="${1:-}"
CONTEXT="${2:-}"
if [[ -n "${3:-}" ]]; then
    STATE="$3"
fi

if [[ -z "$TARGET" || -z "$CONTEXT" ]]; then
    echo "Usage: $0 [--mined|--blocked] <target-slug> <context> [RIPE|BLOCKED|MINED]" >&2
    exit 1
fi

AGENT_ID="cli-$(whoami)"

PAYLOAD=$(jq -n \
    --arg tool "mempool_post" \
    --arg target "$TARGET" \
    --arg domain "mempool" \
    --arg context "$CONTEXT" \
    --arg agent_id "$AGENT_ID" \
    --argjson tags "[\"$STATE\"]" \
    '{tool: $tool, target: $target, domain: $domain, context: $context,
      agent_id: $agent_id, tags: $tags, consumer: "session-agenda"}')

RESP=$(curl -s --max-time 10 \
    -X POST "http://${KERNEL_ADDR}/observe" \
    -H "Authorization: Bearer ${API_KEY}" \
    -H "Content-Type: application/json" \
    -d "$PAYLOAD")

STATUS=$(echo "$RESP" | jq -r '.status // "error"' 2>/dev/null)
if [[ "$STATUS" == "observed" ]]; then
    echo "✓ mempool [$STATE] $TARGET"
else
    echo "✗ failed: $RESP" >&2
    exit 1
fi
