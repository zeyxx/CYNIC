from abc import ABC, abstractmethod
from typing import List, Optional
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
