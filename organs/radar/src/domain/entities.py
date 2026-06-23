from enum import Enum
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Dict, Any, Optional

class TargetType(str, Enum):
    API_ENDPOINT = "api_endpoint"
    BLOCKCHAIN_PROGRAM = "blockchain_program"
    SOCIAL_FEED = "social_feed"

@dataclass
class Target:
    id: str
    target_type: TargetType
    uri: str
    metadata: Dict[str, Any] = field(default_factory=dict)

class ObservationStatus(str, Enum):
    HEALTHY = "healthy"
    DEGRADED = "degraded"
    DOWN = "down"
    MUTATED = "mutated"  # e.g., smart contract upgrade detected
    UNKNOWN = "unknown"

@dataclass
class Observation:
    target_id: str
    status: ObservationStatus
    timestamp: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    latency_ms: Optional[float] = None
    raw_response: Dict[str, Any] = field(default_factory=dict)
