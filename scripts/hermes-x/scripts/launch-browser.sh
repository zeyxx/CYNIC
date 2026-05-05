#!/usr/bin/env bash
# CYNIC Hermes Browser — launches Chrome with proxy + CDP for passive capture.
#
# - Uses dedicated profile (X accounts persist across restarts)
# - Routes through mitmproxy (hermes-proxy.service must be running)
# - Exposes CDP on a fixed port for Hermes agent / Playwright
# - Writes CDP URL to state file for service discovery
#
# Usage:
#   ./launch-browser.sh              # foreground (for systemd)
#   ./launch-browser.sh --headless   # headless (no display needed)
#
# R1: no hardcoded paths — uses env vars with fallbacks.
set -euo pipefail

CHROME="${CHROME_BIN:-/opt/google/chrome/chrome}"
CDP_PORT="${HERMES_CDP_PORT:-40769}"
PROXY_PORT="${HERMES_PROXY_PORT:-8888}"
PROFILE_DIR="${HERMES_CHROME_PROFILE:-${HOME}/.cache/ms-playwright/mcp-chrome-cc49c36}"
STATE_DIR="${CYNIC_ORGANS_DIR:-${HOME}/.cynic/organs}/hermes"
STATE_FILE="${STATE_DIR}/browser-state.json"
START_URL="${HERMES_START_URL:-https://x.com/home}"

mkdir -p "${STATE_DIR}"

# Pre-flight: proxy must be listening
if ! ss -tln | grep -q ":${PROXY_PORT} "; then
    echo "WARN: mitmproxy not listening on port ${PROXY_PORT}" >&2
    echo "      Start hermes-proxy.service first for passive capture." >&2
    echo "      Launching without proxy — no capture will occur." >&2
    PROXY_FLAG=""
else
    PROXY_FLAG="--proxy-server=http://127.0.0.1:${PROXY_PORT}"
fi

CHROME_FLAGS=(
    "--remote-debugging-port=${CDP_PORT}"
    "--user-data-dir=${PROFILE_DIR}"
    "--no-first-run"
    "--no-default-browser-check"
    "--disable-sync"
    "--disable-background-timer-throttling"
    "--disable-backgrounding-occluded-windows"
)

if [ -n "${PROXY_FLAG:-}" ]; then
    CHROME_FLAGS+=("${PROXY_FLAG}")
fi

if [[ "${1:-}" == "--headless" ]]; then
    CHROME_FLAGS+=("--headless=new")
fi

# Write state before launch
cat > "${STATE_FILE}" <<EOF
{
  "cdp_port": ${CDP_PORT},
  "cdp_url": "ws://127.0.0.1:${CDP_PORT}",
  "proxy_port": ${PROXY_PORT},
  "profile_dir": "${PROFILE_DIR}",
  "pid": $$,
  "started_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo "Hermes browser starting — CDP :${CDP_PORT}, proxy :${PROXY_PORT}"

# exec replaces the shell — systemd tracks the Chrome PID directly
exec "${CHROME}" "${CHROME_FLAGS[@]}" --new-window "${START_URL}"
