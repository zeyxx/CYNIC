from abc import ABC, abstractmethod
from typing import List, Optional, Dict
from .entities import MeshNode, MeshRoute, WireguardConfig

class MeshStatePort(ABC):
    @abstractmethod
    async def fetch_mesh_nodes(self) -> List[MeshNode]:
        pass

    @abstractmethod
    async def fetch_mesh_routes(self) -> List[MeshRoute]:
        pass

    @abstractmethod
    async def publish_node_endpoint(self, node_id: str, endpoint: str) -> bool:
        pass

class NetworkInterfacePort(ABC):
    @abstractmethod
    async def resolve_external_endpoint(self, stun_host: str, stun_port: int) -> Optional[str]:
        pass

    @abstractmethod
    def generate_private_key(self) -> str:
        pass

    @abstractmethod
    def derive_public_key(self, private_key: str) -> str:
        pass

    @abstractmethod
    async def apply_wireguard_config(self, interface: str, config: WireguardConfig) -> bool:
        pass

class RegistryPort(ABC):
    @abstractmethod
    def fetch_static_topology(self) -> List[MeshNode]:
        """Fetches the desired state from the immutable registry (e.g., registry.json)."""
        pass

class DnsConfigPort(ABC):
    @abstractmethod
    def write_synapse_hosts(self, nodes: List[MeshNode]) -> bool:
        """Writes the mapping of names to IPs for CoreDNS to consume from the state/ directory."""
        pass
