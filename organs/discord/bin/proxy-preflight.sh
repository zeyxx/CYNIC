#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
PAC="$ROOT/organs/discord/state/proxy-policy.pac"
DB="${DISCORD_ORGAN_DB:-$HOME/.cynic/organs/discord/messages.db}"
MITM_CA="$HOME/.mitmproxy/mitmproxy-ca-cert.pem"

fail=0

check() {
  local name="$1"
  shift
  if "$@"; then
    printf 'ok %s\n' "$name"
  else
    printf 'fail %s\n' "$name"
    fail=1
  fi
}

check "mitmproxy_listener_127.0.0.1_8890" bash -c "ss -ltn 2>/dev/null | grep -Eq '127\\.0\\.0\\.1:8890|localhost:8890'"
check "pac_file_exists" test -f "$PAC"
check "discord_config_exists" test -f "$HOME/.cynic/organs/discord/config.yaml"
check "sqlite_db_parent_exists" test -d "$(dirname "$DB")"

if [ -f "$MITM_CA" ]; then
  printf 'ok mitmproxy_ca_exists %s\n' "$MITM_CA"
else
  printf 'warn mitmproxy_ca_missing %s\n' "$MITM_CA"
fi

if command -v google-chrome >/dev/null 2>&1; then
  printf 'ok google_chrome_exists %s\n' "$(command -v google-chrome)"
else
  printf 'warn google_chrome_missing\n'
fi

if [ "$fail" -eq 0 ]; then
  printf 'preflight=pass\n'
else
  printf 'preflight=fail\n'
fi

exit "$fail"
