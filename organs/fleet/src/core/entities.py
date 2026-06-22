from typing import List, Dict, Any
from dataclasses import dataclass

@dataclass
class FleetNode:
    name: str
    status: str
    raw_spec: Dict[str, Any]
