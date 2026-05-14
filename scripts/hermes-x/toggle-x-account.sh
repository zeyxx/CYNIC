#!/usr/bin/env bash
# CYNIC Hermes — Toggle between X accounts (cynic / personal)
#
# Stops all services for current account, updates symlinks, starts services for target account.
#
# Usage:
#   ./toggle-x-account.sh cynic     # Switch to CYNIC account
#   ./toggle-x-account.sh personal  # Switch to Personal account
#
# Requirements:
# - systemctl --user available (user-level systemd)
# - Accounts configured in ~/.config/cynic/accounts.toml
#
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 {cynic|personal}" >&2
    exit 1
fi

TARGET_ACCOUNT="$1"
VALID_ACCOUNTS=(cynic personal)

# Validate account
if [[ ! " ${VALID_ACCOUNTS[@]} " =~ " ${TARGET_ACCOUNT} " ]]; then
    echo "ERROR: Invalid account '${TARGET_ACCOUNT}'. Valid accounts: ${VALID_ACCOUNTS[*]}" >&2
    exit 1
fi

STATE_DIR="${CYNIC_ORGANS_DIR:-${HOME}/.cynic/organs}/hermes/x"

# List of services to toggle (in correct order — reverse for stop, normal for start)
SERVICES=(
    "hermes-browser"
    "hermes-proxy"
    "hermes-browser-hub"
    "hermes-x-ingest"
    "hermes-x-recovery"
)

echo "Toggling Hermes X account: ${TARGET_ACCOUNT}"
echo "State directory: ${STATE_DIR}"

# Step 1: Find currently active account
ACTIVE_ACCOUNT=""
for service in "${SERVICES[@]}"; do
    # Check if any instance of this service is running
    if systemctl --user is-active "${service}@cynic.service" &>/dev/null; then
        ACTIVE_ACCOUNT="cynic"
        break
    elif systemctl --user is-active "${service}@personal.service" &>/dev/null; then
        ACTIVE_ACCOUNT="personal"
        break
    fi
done

if [ -z "${ACTIVE_ACCOUNT}" ]; then
    echo "WARN: No active account detected. Assuming target account will start fresh." >&2
    ACTIVE_ACCOUNT="unknown"
else
    echo "Current active account: ${ACTIVE_ACCOUNT}"
fi

# Step 2: Stop all services for active account (if found)
if [ "${ACTIVE_ACCOUNT}" != "unknown" ]; then
    echo "Stopping services for account: ${ACTIVE_ACCOUNT}"
    for service in "${SERVICES[@]}"; do
        unit="${service}@${ACTIVE_ACCOUNT}.service"
        if systemctl --user is-active "$unit" &>/dev/null; then
            echo "  Stopping $unit"
            systemctl --user stop "$unit" || true
        fi
    done

    # Wait a moment for services to shut down cleanly
    sleep 2
fi

# Step 3: Update symlinks for backward compatibility
echo "Updating symlinks for account: ${TARGET_ACCOUNT}"
mkdir -p "${STATE_DIR}/datasets/${TARGET_ACCOUNT}"

# Symlink legacy single-account paths to active account
ln -sf "datasets/${TARGET_ACCOUNT}/dataset.jsonl" "${STATE_DIR}/dataset.jsonl"
ln -sf "datasets/${TARGET_ACCOUNT}/ingest_cursor.txt" "${STATE_DIR}/ingest_cursor.txt"
ln -sf "datasets/${TARGET_ACCOUNT}/verdicts" "${STATE_DIR}/verdicts"
ln -sf "datasets/${TARGET_ACCOUNT}/observations" "${STATE_DIR}/observations"

echo "Symlinks updated:"
ls -la "${STATE_DIR}/dataset.jsonl" | sed 's/^/  /'
ls -la "${STATE_DIR}/ingest_cursor.txt" | sed 's/^/  /'

# Step 4: Start all services for target account
echo "Starting services for account: ${TARGET_ACCOUNT}"
for service in "${SERVICES[@]}"; do
    unit="${service}@${TARGET_ACCOUNT}.service"
    echo "  Starting $unit"
    systemctl --user start "$unit" || {
        echo "ERROR: Failed to start $unit" >&2
        exit 1
    }
done

# Step 5: Wait for services to be ready
echo "Waiting for services to stabilize..."
sleep 3

# Step 6: Verify health
echo "Service status:"
for service in "${SERVICES[@]}"; do
    unit="${service}@${TARGET_ACCOUNT}.service"
    status=$(systemctl --user is-active "$unit" 2>/dev/null || echo "unknown")
    printf "  %-40s %s\n" "$unit" "$status"
done

# Step 7: Update browser-state.json
STATE_FILE="${STATE_DIR}/browser-state.json"
if [ -f "${STATE_FILE}" ]; then
    echo "Updating browser-state.json with account: ${TARGET_ACCOUNT}"
    # Read current state and update account_id
    python3 <<EOF
import json
with open("${STATE_FILE}", "r") as f:
    state = json.load(f)
state["account_id"] = "${TARGET_ACCOUNT}"
state["switched_at"] = "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
with open("${STATE_FILE}", "w") as f:
    json.dump(state, f, indent=2)
print(f"Updated: {json.dumps(state, indent=2)}")
EOF
fi

echo "✓ Successfully switched to account: ${TARGET_ACCOUNT}"
