#!/usr/bin/env bash
set -euo pipefail

DB="${DISCORD_ORGAN_DB:-$HOME/.cynic/organs/discord/messages.db}"

printf 'discord_proxy_control_status\n'
printf 'mitmproxy_listener='
if ss -ltn 2>/dev/null | grep -Eq '127\.0\.0\.1:8890|localhost:8890'; then
  printf 'listening\n'
else
  printf 'missing\n'
fi

printf 'system_proxy_mode='
if command -v gsettings >/dev/null 2>&1; then
  gsettings get org.gnome.system.proxy mode 2>/dev/null || printf 'unknown\n'
else
  printf 'gsettings_missing\n'
fi

printf 'db_path=%s\n' "$DB"
if [ -f "$DB" ]; then
  printf 'recent_raw_events='
  sqlite3 "$DB" "SELECT COALESCE(count(*),0) FROM raw_events WHERE captured_at > datetime('now','-10 minutes');" 2>/dev/null || printf 'schema_missing'
  printf '\n'
  printf 'recent_messages='
  sqlite3 "$DB" "SELECT COALESCE(count(*),0) FROM messages WHERE created_at > datetime('now','-10 minutes');" 2>/dev/null || printf 'schema_missing'
  printf '\n'
  printf 'latest_channel_activity='
  sqlite3 "$DB" "SELECT COALESCE(channel_id || ':' || max(date), 'none') FROM messages;" 2>/dev/null || printf 'schema_missing'
  printf '\n'
else
  printf 'db_status=missing\n'
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
printf 'pac_file=%s/organs/discord/state/proxy-policy.pac\n' "$ROOT"
