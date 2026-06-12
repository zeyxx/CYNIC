#!/bin/bash
set -euo pipefail
export LC_ALL=C

PROJECT_DIR="${CYNIC_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
STATE_DIR="$PROJECT_DIR/infra/organ-docker"
STATE_FILE="$STATE_DIR/state.json"
AUDIT_FILE="$STATE_DIR/audit.jsonl"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

mkdir -p "$STATE_DIR"

systemd_active="$(systemctl --user is-active cynic-portal.service 2>/dev/null || echo unknown)"
systemd_substate="$(systemctl --user show -p SubState --value cynic-portal.service 2>/dev/null || echo unknown)"
systemd_result="$(systemctl --user show -p Result --value cynic-portal.service 2>/dev/null || echo unknown)"
compose_json='[]'
compose_status='unavailable'
compose_error=''
if command -v docker >/dev/null 2>&1; then
  compose_err_file="$(mktemp)"
  if compose_json_raw="$(docker compose -f "$PROJECT_DIR/infra/docker/cynic-portal/docker-compose.yml" ps --format json 2>"$compose_err_file")"; then
    compose_json="${compose_json_raw:-[]}"
    compose_status='ok'
  else
    compose_status='error'
    compose_error="$(sed -n '1p' "$compose_err_file" 2>/dev/null || echo 'docker compose ps failed')"
  fi
  rm -f "$compose_err_file"
else
  compose_error='docker cli unavailable'
fi

container_count="$(printf '%s' "$compose_json" | jq 'length' 2>/dev/null || echo 0)"
compose_services="$(printf '%s' "$compose_json" | jq 'map(.Service // .ServiceName // .Name // "unknown")' 2>/dev/null || echo '[]')"

drift_notes=$(jq -n \
  --arg systemd_active "$systemd_active" \
  --arg systemd_substate "$systemd_substate" \
  --arg systemd_result "$systemd_result" \
  --arg compose_status "$compose_status" \
  --arg compose_error "$compose_error" \
  '[
    (if $systemd_active != "active" then "cynic-portal service is not active" else empty end),
    (if $systemd_substate != "running" then ("cynic-portal substate=" + $systemd_substate) else empty end),
    (if $systemd_result != "success" then ("cynic-portal result=" + $systemd_result) else empty end),
    (if $compose_status != "ok" then ($compose_error // "docker compose ps could not be collected") else empty end)
  ]')

alerts=$(jq -n \
  --arg systemd_active "$systemd_active" \
  --arg systemd_result "$systemd_result" \
  --arg compose_status "$compose_status" \
  '[
    (if $systemd_active != "active" then "fallback portal service unhealthy" else empty end),
    (if $systemd_result != "success" then ("service result=" + $systemd_result) else empty end),
    (if $compose_status != "ok" then "container listing unavailable" else empty end)
  ]')

jq -n \
  --arg timestamp "$TIMESTAMP" \
  --arg systemd_active "$systemd_active" \
  --arg systemd_substate "$systemd_substate" \
  --arg systemd_result "$systemd_result" \
  --arg compose_status "$compose_status" \
  --arg compose_error "$compose_error" \
  --arg observed_url 'http://192.168.0.12:5000/' \
  --argjson compose_services "$compose_services" \
  --argjson container_count "${container_count:-0}" \
  --argjson drift_notes "$drift_notes" \
  --argjson alerts "$alerts" \
  '{version:"1.0.0",updated:$timestamp,source:"organ-docker-state",systemd_active:$systemd_active,systemd_substate:$systemd_substate,systemd_result:$systemd_result,compose_status:$compose_status,compose_services:$compose_services,container_count:$container_count,observed_url:$observed_url,drift_notes:$drift_notes,alerts:$alerts}' \
  | tee "$STATE_FILE" >/dev/null

jq -c -n --arg timestamp "$TIMESTAMP" --argjson state "$(cat "$STATE_FILE")" '{timestamp:$timestamp,action:"observation",outcome:"success",details:$state}' >> "$AUDIT_FILE"
cat "$STATE_FILE"
