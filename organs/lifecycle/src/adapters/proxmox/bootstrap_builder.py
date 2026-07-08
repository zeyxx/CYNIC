"""
The factory for generating universal cloud-init / hookscript snippets.
Pure, side-effect free logic.
"""
from typing import Optional

class BootstrapBuilder:
    @staticmethod
    def generate_synapse_cloud_init(mesh_ip: str, public_key_post_url: str) -> str:
        """
        Generates the Cloud-Init user-data payload for a Synapse node.
        Installs the minimal WireGuard substrate, configures the synapse IP,
        and publishes the generated pubkey to the Kernel.
        """
        return f"""#cloud-config
package_update: true
packages:
  - wireguard-tools
  - curl
  - git
  - python3
  - python3-pip
  - python3-venv
runcmd:
  - wg genkey | tee /etc/wireguard/privatekey | wg pubkey > /etc/wireguard/publickey
  - chmod 600 /etc/wireguard/privatekey
  - |
    cat <<EOF > /etc/wireguard/wg0.conf
    [Interface]
    PrivateKey = $(cat /etc/wireguard/privatekey)
    Address = {mesh_ip}/24
    # Peers are reconciled later by the canonical network control plane.
    EOF
  - systemctl enable wg-quick@wg0
  - systemctl start wg-quick@wg0
  - PUBKEY=$(cat /etc/wireguard/publickey)
  - curl -X POST {public_key_post_url} -H "Content-Type: application/json" -d "{{\\"mesh_ip\\":\\"{mesh_ip}\\",\\"public_key\\":\\"$PUBKEY\\"}}"
  - git clone https://github.com/zeyxx/CYNIC.git /opt/cynic
"""
