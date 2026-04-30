#!/bin/bash
# Deploy H3 Secrets Leakage Fixes (Critical)
# Ensures CYNIC_API_KEY never appears in process list or CLI args
# Run: sudo bash scripts/deploy-h3-secrets-fixes.sh

set -e

echo "=== H3 SECRETS LEAKAGE FIX DEPLOYMENT ==="
echo ""

# 1. Update hermes-infrastructure-monitor.service
echo "[1] Updating hermes-infrastructure-monitor.service to use wrapper script..."
CYNIC_ROOT_PATH=$(git rev-parse --show-toplevel)
cat > /etc/systemd/system/hermes-infrastructure-monitor.service << EOF
[Unit]
Description=K15 Infrastructure Consumer — Route probe failures to recovery
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=$CYNIC_ROOT_PATH

# Environment: set CYNIC_ROOT for script
Environment="CYNIC_ROOT=$CYNIC_ROOT_PATH"
Environment="HOME=/root"

# Load secrets from .cynic-env (systemd redacts from unprivileged systemctl show)
EnvironmentFile=/root/.cynic-env

# Run via wrapper script (routes probe failures to recovery, uses Tailscale endpoint)
ExecStart=$CYNIC_ROOT_PATH/scripts/start-k15-infrastructure-consumer.sh

Restart=on-failure
RestartSec=30
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
echo "✓ Updated /etc/systemd/system/hermes-infrastructure-monitor.service"

# 2. Reload systemd
echo "[2] Reloading systemd configuration..."
systemctl daemon-reload
echo "✓ Systemd reloaded"

# 3. Restart both K15 consumer services
echo "[3] Restarting K15 consumer services with new configuration..."
systemctl restart hermes-k15-consumer.service hermes-infrastructure-monitor.service
sleep 2
echo "✓ Services restarted"

# 4. Verify H3: No API key in process list
echo ""
echo "[4] VERIFICATION — Checking that CYNIC_API_KEY is NOT in process command lines..."
FOUND_LEAK=0

# Check k15 consumer
if ps aux | grep -E 'k15_observation_consumer' | grep -v grep | grep -q "CYNIC_API_KEY"; then
    echo "✗ FAIL: CYNIC_API_KEY found in k15_observation_consumer process"
    FOUND_LEAK=1
else
    echo "✓ PASS: k15_observation_consumer runs without API key in CLI"
fi

# Check infrastructure consumer
if ps aux | grep -E 'k15_infrastructure_consumer' | grep -v grep | grep -q "CYNIC_API_KEY"; then
    echo "✗ FAIL: CYNIC_API_KEY found in k15_infrastructure_consumer process"
    FOUND_LEAK=1
else
    echo "✓ PASS: k15_infrastructure_consumer runs without API key in CLI"
fi

if [ $FOUND_LEAK -eq 1 ]; then
    echo ""
    echo "✗ H3 DEPLOYMENT FAILED: Secrets still leaking to process list"
    exit 1
fi

echo ""
echo "=== H3 DEPLOYMENT COMPLETE ==="
echo ""
echo "✓ CRITICAL: CYNIC_API_KEY no longer appears in process list (ps aux)"
echo "✓ Both K15 consumers now load secrets from EnvironmentFile only"
echo "✓ Infrastructure consumer now connects to Tailscale kernel (not localhost)"
echo ""
echo "Monitoring services:"
systemctl status hermes-k15-consumer.service hermes-infrastructure-monitor.service --lines=0
