from dataclasses import dataclass, field
from typing import List, Dict, Any

@dataclass
class OrganEvent:
    organ_name: str
    timestamp: int
    data: Dict[str, Any]

@dataclass
class ProofOfHistory:
    domain: str  # "organism" or "human"
    total_events: int
    recent_events: List[OrganEvent] = field(default_factory=list)
    summary: str = "Unprocessed PoH"
