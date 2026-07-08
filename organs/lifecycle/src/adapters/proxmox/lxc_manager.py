from typing import Dict, Any
from .client import ProxmoxClient

class LxcManager:
    """
    Manages LXC specific logic (cloning, configuring network, snippets).
    """
    def __init__(self, client: ProxmoxClient):
        self.client = client

    def clone_container(self, node: str, vmid: int, newid: int, newname: str) -> None:
        endpoint = f"/nodes/{node}/lxc/{vmid}/clone"
        self.client.post(endpoint, {"newid": newid, "hostname": newname})

    def attach_hookscript(self, node: str, vmid: int, snippet_path: str) -> None:
        """Attaches a hookscript/snippet to an LXC container for bootstrap"""
        endpoint = f"/nodes/{node}/lxc/{vmid}/config"
        self.client.put(endpoint, {"hookscript": snippet_path})
        
    def start_container(self, node: str, vmid: int) -> None:
        endpoint = f"/nodes/{node}/lxc/{vmid}/status/start"
        self.client.post(endpoint, {})
