#!/bin/bash
# Install cynic-kernel systemd unit with hardening
# Run: sudo bash scripts/install-cynic-kernel-systemd.sh

set -e

SERVICE_FILE="/etc/systemd/system/cynic-kernel.service"

cat > "$SERVICE_FILE" << 'EOF'
[Unit]
Description=CYNIC Kernel — Judgment Engine
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=user
WorkingDirectory=/home/user/Bureau/CYNIC
EnvironmentFile=/home/user/.config/cynic/env
ExecStart=/home/user/bin/cynic-kernel --bind <TAILSCALE_CORE>:3030

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

echo "✓ Created $SERVICE_FILE"

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
