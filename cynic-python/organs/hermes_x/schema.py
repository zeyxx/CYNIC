"""Layer 3: STRUCTURATION — Data schema for hermes_x organ."""
from dataclasses import dataclass
from typing import Dict, List, Optional
from datetime import datetime

@dataclass
class Tweet:
    id: str
    text: str
    author: str
    created_at: str
    signal_score: Optional[float] = None
    narrative: Optional[str] = None

@dataclass
class Verdict:
    id: str
    content: str
    domain: str
    q_score: float
    verdict_type: str
    timestamp: str
    created_at: str

@dataclass
class SessionTurn:
    session_id: str
    turn_count: int
    timestamp: str
    intent: str
    message_length: int

@dataclass
class RawPerception:
    timestamp: str
    tweets: List[Tweet]
    verdicts: List[Verdict]
    sessions: List[SessionTurn]
    observation_count: int

@dataclass
class CleanedData:
    timestamp: str
    tweets_valid: List[Tweet]
    tweets_dropped: int
    verdicts_valid: List[Verdict]
    verdicts_dropped: int
    sessions_valid: List[SessionTurn]
    sessions_dropped: int

    def quality_score(self) -> float:
        total = sum([len(self.tweets_valid), self.tweets_dropped, len(self.verdicts_valid),
                     self.verdicts_dropped, len(self.sessions_valid), self.sessions_dropped])
        if total == 0:
            return 0.0
        valid = len(self.tweets_valid) + len(self.verdicts_valid) + len(self.sessions_valid)
        return valid / total

@dataclass
class DomainMetric:
    domain: str
    verdict_count: int
    avg_q_score: float
    howl_count: int
    bark_count: int
    growl_count: int
    wag_count: int
    confidence: float

@dataclass
class PatternAnalysis:
    timestamp: str
    cycle: int
    tweets_analyzed: int
    high_signal_tweets: int
    avg_signal_score: float
    verdicts_analyzed: int
    domain_metrics: Dict[str, DomainMetric]
    dominant_verdict_type: str
    sessions_analyzed: int
    dominant_intent: str
    anomalies: List[str]
    opportunities: List[str]
    recommendation: str
    gemini_reasoning: Optional[str] = None

@dataclass
class Reflection:
    timestamp: str
    cycle: int
    patterns: PatternAnalysis
    compounded_observations: List[str]
    skill_updates: List[str]
    organ_health: Dict
    is_healthy: bool
    diagnosis: str
