from dataclasses import dataclass, field
from typing import List

@dataclass
class Trend:
    keyword: str
    count: int

@dataclass
class TwitterSurfaceState:
    total_tweets_captured: int
    total_verdicts_judged: int
    latest_trends: List[Trend] = field(default_factory=list)
    proxy_status: str = "unknown"
    timestamp: str = ""
