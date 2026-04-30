"""
Layer 3: STRUCTURATION — Data schema for hermes_x organ.

Defines the contracts between perception, transformation, analysis, and learning.
Each layer transforms data and emits a typed contract for the next layer.
"""

from dataclasses import dataclass, asdict
from typing import Dict, List, Optional, Any
from datetime import datetime


@dataclass
class Tweet:
    """Single X/Twitter post capture"""
    id: str
    text: str
    author: str
    created_at: str
    signal_score: Optional[float] = None  # -5 to +7
    narrative: Optional[str] = None  # brief categorization


@dataclass
class Verdict:
    """Kernel judgment on a token/entity"""
    id: str
    content: str
    domain: str  # token, security, memecoin, etc.
    q_score: float  # 0-1 confidence
    verdict_type: str  # HOWL, BARK, GROWL, WAG, etc.
    timestamp: str
    created_at: str


@dataclass
class SessionTurn:
    """Claude Code session turn"""
    session_id: str
    turn_count: int
    timestamp: str
    intent: str  # refactor, debug, feature, etc.
    message_length: int


@dataclass
class RawPerception:
    """Layer 1 output: raw data from all sources, unprocessed"""
    timestamp: str
    tweets: List[Tweet]
    verdicts: List[Verdict]
    sessions: List[SessionTurn]
    observation_count: int

    def to_dict(self) -> Dict:
        return {
            "timestamp": self.timestamp,
            "tweets_count": len(self.tweets),
            "verdicts_count": len(self.verdicts),
            "sessions_count": len(self.sessions),
            "observation_count": self.observation_count,
        }


@dataclass
class CleanedData:
    """Layer 2 output: cleaned, normalized, validated data"""
    timestamp: str
    tweets_valid: List[Tweet]
    tweets_dropped: int
    verdicts_valid: List[Verdict]
    verdicts_dropped: int
    sessions_valid: List[SessionTurn]
    sessions_dropped: int

    def quality_score(self) -> float:
        """Data quality: valid / (valid + dropped)"""
        total_input = (len(self.tweets_valid) + self.tweets_dropped +
                       len(self.verdicts_valid) + self.verdicts_dropped +
                       len(self.sessions_valid) + self.sessions_dropped)
        if total_input == 0:
            return 0.0
        total_valid = len(self.tweets_valid) + len(self.verdicts_valid) + len(self.sessions_valid)
        return total_valid / total_input


@dataclass
class DomainMetric:
    """Per-domain analysis metric"""
    domain: str
    verdict_count: int
    avg_q_score: float
    howl_count: int
    bark_count: int
    growl_count: int
    wag_count: int
    confidence: float  # φ⁻¹ = 0.618 is threshold


@dataclass
class PatternAnalysis:
    """Layer 4 output: analyzed patterns from cleaned data"""
    timestamp: str
    cycle: int

    # Tweet patterns
    tweets_analyzed: int
    high_signal_tweets: int
    avg_signal_score: float

    # Verdict patterns
    verdicts_analyzed: int
    domain_metrics: Dict[str, DomainMetric]
    dominant_verdict_type: str

    # Session patterns
    sessions_analyzed: int
    dominant_intent: str

    # Anomalies detected
    anomalies: List[str]

    # Opportunities
    opportunities: List[str]

    # Recommendation for human action
    recommendation: str

    # Gemini reasoning (if Gemini was used)
    gemini_reasoning: Optional[str] = None


@dataclass
class Reflection:
    """Layer 5 output: compounded wisdom from multiple cycles"""
    timestamp: str
    cycle: int

    # Current cycle's analysis
    patterns: PatternAnalysis

    # Historical context (from prior cycles)
    compounded_observations: List[str]

    # What we learned
    skill_updates: List[str]  # new entries for SKILL.md

    # Health of the organ
    organ_health: Dict[str, Any]  # perception_ok, transformation_ok, analysis_ok, etc.

    # Final diagnosis
    is_healthy: bool
    diagnosis: str


# Layer contracts (what each layer outputs)
PerceptionOutput = RawPerception
TransformationOutput = CleanedData
AnalysisOutput = PatternAnalysis
LearningOutput = Reflection
