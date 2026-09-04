#!/bin/bash
# Deploy Synapse DNS to Proxmox PVE
# Source: infra/network/20-synapse-dnsmasq.conf
# Target: /etc/dnsmasq.d/synapse.conf on PVE

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONF_SRC="$REPO_ROOT/infra/network/20-synapse-dnsmasq.conf"
PVE_HOST="${1:-10.10.10.1}"
DNSMASQ_CONF="/etc/dnsmasq.d/synapse.conf"

echo "=== Deploying Synapse DNS to PVE ($PVE_HOST) ==="
echo "Source: $CONF_SRC"
echo "Target: $DNSMASQ_CONF"

# Copy config to PVE
scp "$CONF_SRC" "root@$PVE_HOST:$DNSMASQ_CONF.bak"
ssh "root@$PVE_HOST" "cp $DNSMASQ_CONF.bak $DNSMASQ_CONF && systemctl restart dnsmasq && sleep 1 && systemctl is-active dnsmasq"

# Verify
echo "=== Verification ==="
ssh "root@$PVE_HOST" "dig +short kernel.synapse.cynic @127.0.0.1"
ssh "root@$PVE_HOST" "dig +short forgejo.synapse.cynic @127.0.0.1"
ssh "root@$PVE_HOST" "dig +short builder.synapse.cynic @127.0.0.1"

echo "=== DNS deployed successfully ==="
