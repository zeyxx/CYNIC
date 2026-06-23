from enum import Enum
from pydantic import BaseModel, Field
from datetime import datetime
from typing import Dict, Any, Optional

class TargetType(str, Enum):
    API_ENDPOINT = "api_endpoint"
    BLOCKCHAIN_PROGRAM = "blockchain_program"
    SOCIAL_FEED = "social_feed"

class Target(BaseModel):
    id: str
    target_type: TargetType
    uri: str
    metadata: Dict[str, Any] = Field(default_factory=dict)

class ObservationStatus(str, Enum):
    HEALTHY = "healthy"
    DEGRADED = "degraded"
    DOWN = "down"
    MUTATED = "mutated"  # e.g., smart contract upgrade detected
    UNKNOWN = "unknown"

class Observation(BaseModel):
    target_id: str
    timestamp: datetime = Field(default_factory=datetime.utcnow)
    status: ObservationStatus
    latency_ms: Optional[float] = None
    raw_response: Dict[str, Any] = Field(default_factory=dict)
