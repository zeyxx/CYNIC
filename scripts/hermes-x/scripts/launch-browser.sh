#!/bin/bash
# CYNIC Hermes — Launch Chrome with account-specific profile and CDP
#
# Launched by systemd (hermes-browser@.service) with:
#   HERMES_ACCOUNT=%i             (e.g. "cynic" or "personal")
#   HERMES_CHROME_PROFILE=%h/...  (e.g. ~/.cynic/organs/hermes/x/chrome-profiles/cynic)
#   HERMES_CDP_PORT=40769
#   HERMES_PROXY_PORT=8888
#   DISPLAY=:0
#
set -euo pipefail

ACCOUNT="${HERMES_ACCOUNT:-cynic}"
PROFILE="${HERMES_CHROME_PROFILE:-$HOME/.cynic/organs/hermes/x/chrome-profiles/cynic}"
CDP_PORT="${HERMES_CDP_PORT:-40769}"
PROXY_PORT="${HERMES_PROXY_PORT:-8888}"
DISPLAY="${DISPLAY:-:0}"

echo "[launch-browser.sh] Starting Chrome for account: $ACCOUNT"
echo "  Profile: $PROFILE"
echo "  CDP port: $CDP_PORT"
echo "  Proxy: 127.0.0.1:$PROXY_PORT"
echo "  Display: $DISPLAY"

# Ensure profile directory exists
mkdir -p "$PROFILE"

# Launch Chrome with:
# - Account-specific user data directory
# - Remote debugging port for CDP
# - HTTP proxy pointing to our mitmproxy instance
# - Disable first-run experience
# - Allow insecure localhost (proxy uses self-signed cert)
exec /usr/bin/google-chrome \
    --user-data-dir="$PROFILE" \
    --remote-debugging-port=$CDP_PORT \
    --proxy-server="127.0.0.1:$PROXY_PORT" \
    --no-first-run \
    --no-default-browser-check \
    --disable-component-update \
    --allow-insecure-localhost \
    --ignore-certificate-errors \
    about:blank
