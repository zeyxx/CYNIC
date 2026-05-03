#!/usr/bin/env python3
"""
Unified Behavioral Schema — Multi-agent learning infrastructure.

Single log format for all agents (Hermes, Kairos, Gemini-organic, orthogonal domains).
Not a framework design — emergent from observed data patterns.

Data flow:
  1. Human interaction (clicks, scrolls) → behavior_log.jsonl
  2. Organ captures content → capture event
  3. Organ processes → observation with signal_score
  4. Organism learns from outcome → behavioral_interaction record

All agents read/write to this schema. Single source of truth for adaptation.
"""

from dataclasses import dataclass, asdict
from datetime import datetime
from typing import Optional, Dict, List, Any
from enum import Enum
import json
from pathlib import Path


class InteractionType(Enum):
    """User interaction primitives from behavior_log.jsonl."""
    CLICK = "click"
    SCROLL = "scroll"
    KEY = "key"
    MOUSE_MOVE = "mouse_move"
    WINDOW_FOCUS = "window_focus"


class ObservationType(Enum):
    """Organ observations (what it sees/concludes)."""
    NARRATIVE = "narrative"
    SIGNAL = "signal"
    DOMAIN_PREDICTION = "domain_prediction"
    DOMAIN_PREFERENCE = "domain_preference"
    BEHAVIOR_PATTERN = "behavior_pattern"


class Agent(Enum):
    """Agents in the organism."""
    HERMES = "hermes"
    KAIROS = "kairos"
    GEMINI_ORGANIC = "gemini_organic"
    DETERMINISTIC_DOG = "deterministic_dog"


@dataclass
class HumanInteraction:
    """Raw user interaction from behavior_log.jsonl."""
    timestamp: str  # ISO 8601
    type: InteractionType
    x: int
    y: int
    window_id: str
    window_name: str
    button: Optional[str] = None  # "left" for clicks
    key: Optional[str] = None  # for key events

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d['type'] = self.type.value
        return d


@dataclass
class OrganCapture:
    """What an organ observed (content + context)."""
    timestamp: str  # ISO 8601, when captured
    agent: Agent
    domain: str  # "twitter", "general", "token-analysis", etc.
    content_hash: str  # Hash of observed content (not content itself)
    content_summary: str  # Brief summary or title
    metadata: Dict[str, Any]  # Domain-specific: tweet_id, author, etc.
    source_url: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d['agent'] = self.agent.value
        return d


@dataclass
class OrganObservation:
    """Organ's conclusion from processing a capture."""
    timestamp: str  # ISO 8601, when observation made
    agent: Agent
    domain: str  # Predicted or known domain
    observation_type: ObservationType
    signal_score: float  # 0.0-1.0, quality/confidence
    finding: str  # Human-readable conclusion
    evidence: List[str]  # References to captured content
    metadata: Dict[str, Any]  # finding details: narratives, tweet_ids, etc.

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d['agent'] = self.agent.value
        d['observation_type'] = self.observation_type.value
        return d


@dataclass
class BehavioralInteraction:
    """
    Unified learning record: human action → organ response → organism outcome.

    This is what all agents log. A single schema for feedback loop closure.
    """
    timestamp: str  # ISO 8601, when interaction occurred
    agent: Agent

    # Human side
    human_interaction: Optional[HumanInteraction] = None

    # Organ side
    capture: Optional[OrganCapture] = None
    observation: Optional[OrganObservation] = None

    # Feedback
    domain_predicted: Optional[str] = None  # What domain did organ predict for the capture?
    signal_measured: Optional[float] = None  # Actual signal score (post-processing)
    human_engagement: Optional[float] = None  # 0.0-1.0, did human act on this observation? (0=ignore, 1=click)
    outcome: Optional[str] = None  # Brief description of what happened next

    def to_dict(self) -> Dict[str, Any]:
        d = {}
        d['timestamp'] = self.timestamp
        d['agent'] = self.agent.value

        if self.human_interaction:
            d['human_interaction'] = self.human_interaction.to_dict()
        if self.capture:
            d['capture'] = self.capture.to_dict()
        if self.observation:
            d['observation'] = self.observation.to_dict()

        d['domain_predicted'] = self.domain_predicted
        d['signal_measured'] = self.signal_measured
        d['human_engagement'] = self.human_engagement
        d['outcome'] = self.outcome

        return d


class BehavioralLog:
    """Unified log writer for all agents."""

    def __init__(self, log_path: Optional[Path] = None):
        self.log_path = log_path or Path.home() / ".cynic" / "organisms" / "behavioral_interactions.jsonl"
        self.log_path.parent.mkdir(parents=True, exist_ok=True)

    def append(self, interaction: BehavioralInteraction) -> None:
        """Append interaction to log (append-only, thread-safe file I/O)."""
        with open(self.log_path, 'a') as f:
            f.write(json.dumps(interaction.to_dict()) + '\n')

    def read_all(self) -> List[BehavioralInteraction]:
        """Read all interactions (for analysis/training)."""
        interactions = []
        if not self.log_path.exists():
            return interactions

        with open(self.log_path) as f:
            for line in f:
                try:
                    data = json.loads(line)
                    # Reconstruct (simplified, for analysis)
                    interactions.append(data)
                except:
                    pass

        return interactions

    def read_by_agent(self, agent: Agent) -> List[BehavioralInteraction]:
        """Query by agent."""
        return [i for i in self.read_all() if i.get('agent') == agent.value]

    def read_by_domain(self, domain: str) -> List[BehavioralInteraction]:
        """Query by predicted domain."""
        return [i for i in self.read_all() if i.get('domain_predicted') == domain or
                (i.get('capture') and i['capture'].get('domain') == domain)]


if __name__ == "__main__":
    # Test: Create and log a sample interaction
    log = BehavioralLog()

    # Simulate: Human clicks on Twitter → Hermes captures → Hermes observes high-signal narrative
    interaction = BehavioralInteraction(
        timestamp=datetime.utcnow().isoformat(),
        agent=Agent.HERMES,
        human_interaction=HumanInteraction(
            timestamp=datetime.utcnow().isoformat(),
            type=InteractionType.CLICK,
            x=1920, y=1080,
            window_id="0x123abc", window_name="Twitter"
        ),
        capture=OrganCapture(
            timestamp=datetime.utcnow().isoformat(),
            agent=Agent.HERMES,
            domain="twitter",
            content_hash="abc123def456",
            content_summary="Recovery scammer exposed by KOL",
            metadata={"tweet_id": "1234567890", "author_tier": "HIGH"}
        ),
        observation=OrganObservation(
            timestamp=datetime.utcnow().isoformat(),
            agent=Agent.HERMES,
            domain="token-analysis",
            observation_type=ObservationType.NARRATIVE,
            signal_score=0.87,
            finding="Coordinated rug pull warning",
            evidence=["tweet_id:1234567890"],
            metadata={"narratives": ["scammer", "rug"], "confidence": 0.87}
        ),
        domain_predicted="token-analysis",
        signal_measured=0.87,
        human_engagement=1.0,  # User clicked
        outcome="User engaged with observation, routed to token-analysis domain"
    )

    log.append(interaction)
    print(f"✓ Logged to {log.log_path}")

    # Verify
    all_interactions = log.read_all()
    print(f"✓ {len(all_interactions)} interactions logged")
