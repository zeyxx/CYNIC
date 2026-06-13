from dataclasses import dataclass, field
from typing import List, Dict, Any

@dataclass
class Container:
    raw_data: Dict[str, Any]
    name: str
    status: str
    is_up: bool

@dataclass
class DockerState:
    timestamp: str
    total_containers: int
    running: int
    containers: List[Dict[str, Any]]
    last_actions: List[str] = field(default_factory=list)
