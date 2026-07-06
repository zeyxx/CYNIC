from enum import Enum
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Dict, Any, Optional, List

class TargetType(str, Enum):
    API_ENDPOINT = "api_endpoint"
    BLOCKCHAIN_PROGRAM = "blockchain_program"
    SOCIAL_FEED = "social_feed"
    GITHUB_REPO = "github_repo"

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
    UPDATE_AVAILABLE = "update_available"
    UNKNOWN = "unknown"

@dataclass
class ReleaseInfo:
    """Captured release or commit info from a source."""
    version: str
    tag: str
    published_at: str  # ISO datetime
    url: str
    prerelease: bool = False
    draft: bool = False
    commit_sha: str = ""
    html_url: str = ""

@dataclass
class Observation:
    target_id: str
    status: ObservationStatus
    timestamp: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    latency_ms: Optional[float] = None
    raw_response: Dict[str, Any] = field(default_factory=dict)
    current_version: Optional[str] = None
    latest_version: Optional[str] = None
    release_info: Optional[ReleaseInfo] = None
    pending_releases: List[ReleaseInfo] = field(default_factory=list)
