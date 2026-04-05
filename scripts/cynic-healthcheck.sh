#!/usr/bin/env bash
# cynic-healthcheck.sh — Health check with Telegram alerting.
# The kernel returns HTTP 200 (healthy) or 503 (degraded/critical).
# Alerts on STATE CHANGE only — not every 5 min for a known condition.
# State file tracks last known status. Telegram fires on transition.
# Append-only log records every check (immutable facts, not cache).
set -euo pipefail

source "${HOME}/.cynic-env" 2>/dev/null || { logger -t cynic-healthcheck -p user.err "FATAL: cannot source ~/.cynic-env"; exit 1; }

ADDR="${CYNIC_REST_ADDR:-127.0.0.1:3030}"
STATE_DIR="${HOME}/.local/share/cynic"
LOG="${STATE_DIR}/healthcheck.log"
LAST_STATE="${STATE_DIR}/healthcheck.last"
mkdir -p "$STATE_DIR"

telegram() {
    curl -s -X POST \
        "https://api.telegram.org/bot${CYNIC_TELEGRAM_BOT_TOKEN}/sendMessage" \
        -d chat_id="${CYNIC_TELEGRAM_CHAT_ID}" \
        -d text="$1" \
        -d disable_notification=false \
        > /dev/null 2>&1 || logger -t cynic-healthcheck -p user.warning "Telegram send failed"
}

record() {
    local status="$1" msg="$2"
    local ts
    ts=$(date -Iseconds)
    local prev
    prev=$(cat "$LAST_STATE" 2>/dev/null || echo "UNKNOWN")

    # Append-only log (every check, always)
    echo "${ts} ${status} ${msg}" >> "$LOG"

    # Syslog
    if [ "$status" = "PASS" ]; then
        logger -t cynic-healthcheck -p user.info "$msg"
    else
        logger -t cynic-healthcheck -p user.err "$msg"
    fi

    # Telegram on state CHANGE only
    if [ "$status" != "$prev" ]; then
        if [ "$status" = "PASS" ]; then
            telegram "🟢 CYNIC RECOVERED — ${msg}"
        else
            telegram "🔴 CYNIC ${msg} (was: ${prev})"
        fi
    fi

    # Update state
    echo "$status" > "$LAST_STATE"
}

# Authenticated health check — returns full details + correct HTTP status
if curl -sf --max-time 10 \
    ${CYNIC_API_KEY:+-H "Authorization: Bearer ${CYNIC_API_KEY}"} \
    "http://${ADDR}/health" > /dev/null 2>&1; then
    record "PASS" "healthy"
else
    HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 \
        ${CYNIC_API_KEY:+-H "Authorization: Bearer ${CYNIC_API_KEY}"} \
        "http://${ADDR}/health" 2>/dev/null || echo "000")
    case "$HTTP_CODE" in
        503) record "DEGRADED" "FAIL: DEGRADED (503)" ;;
        000) record "UNREACHABLE" "FAIL: UNREACHABLE" ;;
        *)   record "ERROR" "FAIL: HTTP $HTTP_CODE" ;;
    esac
    exit 1
fi
