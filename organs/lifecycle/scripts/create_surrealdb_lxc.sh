#!/usr/bin/env bash
set -e

echo "=== Provisioning cynic-lxc-surrealdb (103) ==="

# Check if container 103 already exists
if pct status 103 >/dev/null 2>&1; then
    echo "Container 103 already exists. Aborting to prevent overwrite."
    exit 1
fi

# We assume a debian-12-standard template exists in local storage
TEMPLATE=$(pvesm list local -vmid 0 | grep debian-12-standard | awk '{print $1}' | head -n 1)

if [ -z "$TEMPLATE" ]; then
    echo "Error: Debian 12 template not found. Please download it: pveam download local debian-12-standard_12.2-1_amd64.tar.zst"
    exit 1
fi

# Create the container
pct create 103 $TEMPLATE \
    --hostname cynic-lxc-surrealdb \
    --storage local-lvm \
    --rootfs local-lvm:8 \
    --memory 2048 \
    --swap 512 \
    --cores 2 \
    --net0 name=eth0,bridge=vmbr0,ip=192.168.0.43/24,gw=192.168.0.1 \
    --unprivileged 1 \
    --features nesting=1 \
    --ostype debian

# Start the container
pct start 103

echo "Waiting for container to boot..."
sleep 5

# Install dependencies and SurrealDB
pct exec 103 -- bash -c "apt-get update && apt-get install -y curl wget gnupg nano"
pct exec 103 -- bash -c "curl -sSf https://install.surrealdb.com | sh"

# Install Headscale client (Tailscale)
pct exec 103 -- bash -c "curl -fsSL https://tailscale.com/install.sh | sh"

# Setup Systemd service for SurrealDB
pct exec 103 -- bash -c 'cat <<EOF > /etc/systemd/system/surrealdb.service
[Unit]
Description=SurrealDB Server
After=network.target

[Service]
ExecStart=/root/.surrealdb/surreal start --log trace --user root --pass root file://data/surreal.db
Restart=on-failure
WorkingDirectory=/root

[Install]
WantedBy=multi-user.target
EOF'

pct exec 103 -- bash -c "systemctl enable --now surrealdb.service"

echo "=== Provisioning Complete ==="
echo "Note: You must still join the Tailscale mesh manually with: pct exec 103 -- tailscale up --login-server=http://<HEADSCALE_IP>:8080"
