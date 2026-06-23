from dataclasses import dataclass, field
from typing import List, Optional

@dataclass
class MeshNode:
    id: str
    name: str
    public_key: str
    mesh_ip: str
    external_endpoint: Optional[str] = None
    last_seen: Optional[str] = None

@dataclass
class MeshRoute:
    id: str
    source_node: str
    target_node: str
    allowed_ips: List[str] = field(default_factory=list)

@dataclass
class WireguardPeerConfig:
    public_key: str
    allowed_ips: List[str]
    endpoint: Optional[str] = None
    persistent_keepalive: int = 25

@dataclass
class WireguardConfig:
    private_key: str
    address: str
    listen_port: Optional[int] = None
    peers: List[WireguardPeerConfig] = field(default_factory=list)
