#!/bin/bash
# Install cynic-kernel systemd unit with hardening
# Run: sudo bash scripts/install-cynic-kernel-systemd.sh

set -e

SERVICE_FILE="/etc/systemd/system/cynic-kernel.service"

# Source env to get CYNIC_REST_ADDR
source /home/user/.config/cynic/env 2>/dev/null || source /root/.cynic-env 2>/dev/null || true

# Extract Tailscale IP from CYNIC_REST_ADDR (format: IP:PORT)
if [ -n "$CYNIC_REST_ADDR" ]; then
  KERNEL_IP="${CYNIC_REST_ADDR%%:*}"
  KERNEL_ADDR="$KERNEL_IP:3030"
else
  KERNEL_ADDR="<TAILSCALE_CORE>:3030"
fi

cat > "$SERVICE_FILE" << EOF
[Unit]
Description=CYNIC Kernel — Judgment Engine
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=user
WorkingDirectory=/home/user/Bureau/CYNIC
EnvironmentFile=/home/user/.config/cynic/env
ExecStart=/home/user/bin/cynic-kernel --bind $KERNEL_ADDR

Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=cynic-kernel

# Hardening per llama-server.service model
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/home/user/Bureau/CYNIC/.cynic /home/user/.cynic
ProtectHome=read-only
PrivateTmp=true
RestrictAddressFamilies=AF_INET AF_INET6

[Install]
WantedBy=multi-user.target
EOF

echo "✓ Created $SERVICE_FILE with address: $KERNEL_ADDR"

systemctl daemon-reload
echo "✓ Reloaded systemd"

systemctl enable cynic-kernel.service
echo "✓ Enabled cynic-kernel (auto-start on boot)"

systemctl stop cynic-kernel.service 2>/dev/null || true
sleep 1

systemctl start cynic-kernel.service
echo "✓ Started cynic-kernel"

sleep 2

systemctl status cynic-kernel.service
