#!/bin/bash
# Harden surreal.service with systemd security features
# Run: sudo bash scripts/harden-surreal-systemd.sh

set -e

SERVICE_FILE="/etc/systemd/system/surreal.service"

cat > "$SERVICE_FILE" << 'EOF'
[Unit]
Description=SurrealDB — CYNIC Knowledge Store
After=network.target

[Service]
Type=simple
User=user
WorkingDirectory=%h/.surrealdb
ExecStart=/usr/local/bin/surreal start \
  --bind 127.0.0.1:8000 \
  --transaction-timeout 10s \
  --query-timeout 30s \
  surrealkv://%h/.surrealdb/data

Restart=on-failure
RestartSec=5

StandardOutput=journal
StandardError=journal
SyslogIdentifier=surreal

# Hardening — modeled on llama-server.service
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=%h/.surrealdb
ProtectHome=read-only
PrivateTmp=true
RestrictAddressFamilies=AF_INET AF_INET6

[Install]
WantedBy=multi-user.target
EOF

echo "✓ Updated $SERVICE_FILE with hardening"

systemctl daemon-reload
echo "✓ Reloaded systemd"

systemctl restart surreal.service
echo "✓ Restarted surreal with hardening applied"

sleep 2
systemctl status surreal.service
