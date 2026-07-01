#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# cynic-node-propagate.sh — Mycélium spread script
# Compile cynic-node locally, then deploy it to every declared node
# in infra/registry.json that has a reachable network.local address.
#
# Usage: ./infra/scripts/cynic-node-propagate.sh [--dry-run]
# ─────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
REGISTRY="$REPO_ROOT/infra/registry.json"
DRY_RUN="${1:-}"
BINARY_NAME="cynic-node"
INSTALL_PATH="/usr/local/bin/cynic-node"
SUDOERS_SRC="$REPO_ROOT/infra/sudoers.d/cynic-node"
SUDOERS_DST="/etc/sudoers.d/cynic-node"
SERVICE_SRC="$REPO_ROOT/infra/systemd/cynic-node.service"
SERVICE_DST="/etc/systemd/system/cynic-node.service"
BUILD_TARGET="x86_64-unknown-linux-gnu"

log() { echo "[cynic-propagate] $*"; }
info() { log "INFO  $*"; }
warn() { log "WARN  $*"; }
err()  { log "ERROR $*" >&2; }

# ── 1. Build release binary ───────────────────────────────────────
info "Building $BINARY_NAME release binary..."
if [[ "$DRY_RUN" != "--dry-run" ]]; then
    cargo build --release -p cynic-node --target "$BUILD_TARGET" 2>&1
    BINARY="$REPO_ROOT/target/$BUILD_TARGET/release/$BINARY_NAME"
    [[ -f "$BINARY" ]] || { err "Binary not found after build: $BINARY"; exit 1; }
    info "Binary built: $(du -sh "$BINARY" | cut -f1) at $BINARY"
else
    BINARY="$REPO_ROOT/target/$BUILD_TARGET/release/$BINARY_NAME"
    info "[DRY-RUN] Would build $BINARY"
fi

# ── 2. Extract target nodes from registry ────────────────────────
# We deploy to every node with a non-placeholder local IP
# that is not a pure-VM guest (i.e. the hypervisor host or bare metal nodes)
TARGET_NODES=$(python3 -c "
import json, sys, re
registry = json.load(open('$REGISTRY'))
placeholder = re.compile(r'<.*?>')
for node in registry.get('nodes', []):
    ip = node.get('network', {}).get('local', '')
    if ip and not placeholder.match(ip):
        name = node['name']
        role = node.get('role', 'unknown')
        print(f'{name}|{ip}|{role}')
")

if [[ -z "$TARGET_NODES" ]]; then
    err "No resolved target nodes found in $REGISTRY"
    exit 1
fi

info "Target nodes:"
echo "$TARGET_NODES" | while IFS='|' read -r name ip role; do
    info "  → $name ($role) @ $ip"
done

# ── 3. Deploy to each node ────────────────────────────────────────
DEPLOYED=0
FAILED=0

echo "$TARGET_NODES" | while IFS='|' read -r name ip role; do
    info "─── Deploying to $name ($ip)..."

    if [[ "$DRY_RUN" == "--dry-run" ]]; then
        info "[DRY-RUN] Would scp binary + sudoers + service to root@$ip"
        info "[DRY-RUN] Would systemctl daemon-reload && systemctl enable --now cynic-node"
        continue
    fi

    # Check reachability
    if ! ssh -o ConnectTimeout=5 -o BatchMode=yes -o StrictHostKeyChecking=no "root@$ip" true 2>/dev/null; then
        warn "Cannot reach $name @ $ip — skipping (node may be offline)"
        FAILED=$((FAILED + 1))
        continue
    fi

    # Deploy binary
    scp -o StrictHostKeyChecking=no "$BINARY" "root@$ip:$INSTALL_PATH"
    ssh -o StrictHostKeyChecking=no "root@$ip" "chmod +x $INSTALL_PATH"
    info "  ✓ Binary deployed to $INSTALL_PATH"

    # Deploy sudoers rule (atomic)
    scp -o StrictHostKeyChecking=no "$SUDOERS_SRC" "root@$ip:/tmp/cynic-node.sudoers"
    ssh -o StrictHostKeyChecking=no "root@$ip" \
        "visudo -c -f /tmp/cynic-node.sudoers && mv /tmp/cynic-node.sudoers $SUDOERS_DST && chmod 440 $SUDOERS_DST"
    info "  ✓ Sudoers deployed to $SUDOERS_DST"

    # Deploy systemd service
    scp -o StrictHostKeyChecking=no "$SERVICE_SRC" "root@$ip:$SERVICE_DST"
    ssh -o StrictHostKeyChecking=no "root@$ip" \
        "systemctl daemon-reload && systemctl enable --now cynic-node.service && systemctl status cynic-node.service --no-pager -l"
    info "  ✓ Service enabled on $name"

    DEPLOYED=$((DEPLOYED + 1))
done

info "Done. Deployed=$DEPLOYED Failed=$FAILED"
