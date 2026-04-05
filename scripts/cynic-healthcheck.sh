#!/usr/bin/env bash
# cynic-healthcheck.sh — Health check with Telegram alerting.
# The kernel returns HTTP 200 (healthy) or 503 (degraded/critical).
# Two detection modes:
#   EDGE — alert on state transition (PASS→DEGRADED, DEGRADED→PASS)
#   LEVEL — reminder every REMINDER_INTERVAL while non-PASS persists
# State file: "STATUS LAST_ALERT_EPOCH". Not a cache — the live check
# is authoritative. The file only tracks when we last notified.
# Append-only log records every check (immutable facts).
set -euo pipefail

source "${HOME}/.cynic-env" 2>/dev/null || { logger -t cynic-healthcheck -p user.err "FATAL: cannot source ~/.cynic-env"; exit 1; }

ADDR="${CYNIC_REST_ADDR:-127.0.0.1:3030}"
STATE_DIR="${HOME}/.local/share/cynic"
LOG="${STATE_DIR}/healthcheck.log"
STATE_FILE="${STATE_DIR}/healthcheck.last"
REMINDER_SECS=3600  # 1 hour
mkdir -p "$STATE_DIR"

now_epoch() { date +%s; }

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
    local ts now prev prev_epoch
    ts=$(date -Iseconds)
    now=$(now_epoch)

    # Read previous state
    if [ -f "$STATE_FILE" ]; then
        prev=$(awk '{print $1}' "$STATE_FILE")
        prev_epoch=$(awk '{print $2}' "$STATE_FILE")
    else
        prev="UNKNOWN"
        prev_epoch=0
    fi

    # Append-only log (every check, always)
    echo "${ts} ${status} ${msg}" >> "$LOG"

    # Syslog
    if [ "$status" = "PASS" ]; then
        logger -t cynic-healthcheck -p user.info "$msg"
    else
        logger -t cynic-healthcheck -p user.err "$msg"
    fi

    # EDGE: state changed → alert
    if [ "$status" != "$prev" ]; then
        if [ "$status" = "PASS" ]; then
            telegram "🟢 CYNIC RECOVERED — ${msg}"
        else
            telegram "🔴 CYNIC ${msg} (was: ${prev})"
        fi
        echo "$status $now" > "$STATE_FILE"

    # LEVEL: same non-PASS state, reminder interval elapsed → remind
    elif [ "$status" != "PASS" ] && [ $(( now - prev_epoch )) -ge $REMINDER_SECS ]; then
        local hours=$(( (now - prev_epoch) / 3600 ))
        telegram "⚠️ CYNIC still ${status} (${hours}h) — ${msg}"
        echo "$status $now" > "$STATE_FILE"

    # Same state, within reminder window → log only, no alert
    else
        echo "$status $prev_epoch" > "$STATE_FILE"
    fi
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
