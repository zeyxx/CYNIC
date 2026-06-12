from dataclasses import dataclass
from typing import List, Dict, Any

@dataclass
class Deployment:
    id: str
    name: str
    url: str
    state: str
    created: int

@dataclass
class VercelState:
    timestamp: str
    total_fetched: int
    failing: int
    ready: int
    deployments: List[Dict[str, Any]]
