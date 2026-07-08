from __future__ import annotations

from dataclasses import dataclass


@dataclass
class MeshNode:
    id: str
    name: str
    public_key: str
    mesh_ip: str
    external_endpoint: str | None = None
    last_seen: str | None = None
