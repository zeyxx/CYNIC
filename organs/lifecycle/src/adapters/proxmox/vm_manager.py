from typing import Dict, Any
from .client import ProxmoxClient

class VmManager:
    """
    Manages QEMU VM specific logic (cloning, configuring Cloud-Init cdrom).
    """
    def __init__(self, client: ProxmoxClient):
        self.client = client

    def clone_vm(self, node: str, vmid: int, newid: int, newname: str) -> None:
        endpoint = f"/nodes/{node}/qemu/{vmid}/clone"
        self.client.post(endpoint, {"newid": newid, "name": newname})

    def configure_cloud_init(self, node: str, vmid: int, snippet_path: str) -> None:
        """Configures the VM to use a specific Cloud-Init custom user-data snippet"""
        endpoint = f"/nodes/{node}/qemu/{vmid}/config"
        # Proxmox allows injecting custom Cloud-Init files from snippets
        self.client.post(endpoint, {"cicustom": f"user={snippet_path}"})
        
    def start_vm(self, node: str, vmid: int) -> None:
        endpoint = f"/nodes/{node}/qemu/{vmid}/status/start"
        self.client.post(endpoint, {})
