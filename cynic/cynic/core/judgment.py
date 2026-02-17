"""
CYNIC Judgment Models — Pydantic v2

All Pydantic models for the judgment pipeline.
φ-bounds enforced at model level (LAW 5: database constraints mirror these).
"""
from __future__ import annotations

import uuid
from datetime import datetime
from typing import Any, Dict, List, Optional

from pydantic import BaseModel, Field, field_validator, model_validator

from cynic.core.phi import MAX_CONFIDENCE, MAX_Q_SCORE, PHI_INV, PHI_INV_2


# ════════════════════════════════════════════════════════════════════════════
# CORE IDENTIFIERS
# ════════════════════════════════════════════════════════════════════════════

def new_id() -> str:
    """Generate a new UUID4 string identifier."""
    return str(uuid.uuid4())


# ════════════════════════════════════════════════════════════════════════════
# ∞^N SPACE CELL (A specific state in the infinite hypercube)
# ════════════════════════════════════════════════════════════════════════════

class Cell(BaseModel):
    """
    A specific point/state in the ∞^N hypercube.

    7 REALITY × 7 ANALYSIS × 7 TIME + structural dimensions.
    Each Cell is what CYNIC judges, navigates, and learns from.
    """

    # Core coordinates
    reality: str = Field(
        description="Reality dimension (CODE/SOLANA/MARKET/SOCIAL/HUMAN/CYNIC/COSMOS)"
    )
    analysis: str = Field(
        description="Analysis type (PERCEIVE/JUDGE/DECIDE/ACT/LEARN/ACCOUNT/EMERGE)"
    )
    time_dim: str = Field(
        default="PRESENT",
        description="Time dimension (PAST/PRESENT/FUTURE/CYCLE/TREND/EMERGENCE/TRANSCENDENCE)"
    )

    # Content
    content: Any = Field(description="The actual data being judged (code, tx, price, tweet...)")
    context: str = Field(default="", description="Human-readable context string for LLM scoring")

    # Structural dimensions
    dogs_active: List[str] = Field(default_factory=list, description="Dogs involved in this cell")
    lod: int = Field(default=1, ge=0, le=3, description="Level of Detail (0=pattern, 3=LLM)")
    consciousness: int = Field(default=0, ge=0, le=6, description="Consciousness gradient level")

    # Technical dimensions
    llm_model: Optional[str] = Field(default=None, description="LLM used for this cell")
    tech_stack: List[str] = Field(default_factory=list)

    # Economic dimensions
    budget_usd: float = Field(default=1.0, ge=0.0, description="Budget allocated for this cell")

    # Epistemic dimensions
    novelty: float = Field(default=0.5, ge=0.0, le=1.0)
    complexity: float = Field(default=0.5, ge=0.0, le=1.0)
    risk: float = Field(default=0.0, ge=0.0, le=1.0)

    # Metadata
    cell_id: str = Field(default_factory=new_id)
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())
    metadata: Dict[str, Any] = Field(default_factory=dict)

    @field_validator("reality")
    @classmethod
    def validate_reality(cls, v: str) -> str:
        valid = {"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"}
        if v not in valid:
            raise ValueError(f"reality must be one of {valid}, got '{v}'")
        return v

    @field_validator("analysis")
    @classmethod
    def validate_analysis(cls, v: str) -> str:
        valid = {"PERCEIVE", "JUDGE", "DECIDE", "ACT", "LEARN", "ACCOUNT", "EMERGE"}
        if v not in valid:
            raise ValueError(f"analysis must be one of {valid}, got '{v}'")
        return v

    @field_validator("time_dim")
    @classmethod
    def validate_time(cls, v: str) -> str:
        valid = {"PAST", "PRESENT", "FUTURE", "CYCLE", "TREND", "EMERGENCE", "TRANSCENDENCE"}
        if v not in valid:
            raise ValueError(f"time_dim must be one of {valid}, got '{v}'")
        return v

    def state_key(self) -> str:
        """Generate hashable state key for Q-Learning."""
        return f"{self.reality}:{self.analysis}:{self.time_dim}:{self.lod}"


# ════════════════════════════════════════════════════════════════════════════
# JUDGMENT OUTPUT
# ════════════════════════════════════════════════════════════════════════════

class Judgment(BaseModel):
    """
    The result of judging a Cell.

    φ-bounds enforced:
    - q_score ∈ [0, 61.8]  — never exceed φ⁻¹ × 100
    - confidence ∈ [0, φ⁻¹]  — max 61.8%
    """
    judgment_id: str = Field(default_factory=new_id)

    # Input
    cell: Cell

    # Output (φ-bounded)
    q_score: float = Field(ge=0.0, le=MAX_Q_SCORE, description="Q-Score ∈ [0, 61.8]")
    verdict: str = Field(description="HOWL/WAG/GROWL/BARK")
    confidence: float = Field(ge=0.0, le=MAX_CONFIDENCE, description="Confidence ∈ [0, 0.618]")

    # Breakdown
    axiom_scores: Dict[str, float] = Field(default_factory=dict)
    active_axioms: List[str] = Field(default_factory=list)

    # Consensus info
    dog_votes: Dict[str, float] = Field(default_factory=dict)    # {dog_id: q_score}
    consensus_votes: int = Field(default=0)
    consensus_quorum: int = Field(default=7)
    consensus_reached: bool = Field(default=False)

    # Economics
    cost_usd: float = Field(default=0.0, ge=0.0)
    llm_calls: int = Field(default=0, ge=0)
    llm_tokens: int = Field(default=0, ge=0)

    # THE_UNNAMEABLE
    residual_variance: float = Field(default=0.0, ge=0.0, description="Unexplained variance")
    unnameable_detected: bool = Field(default=False)

    # Metadata
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())
    duration_ms: float = Field(default=0.0, ge=0.0)

    @field_validator("verdict")
    @classmethod
    def validate_verdict(cls, v: str) -> str:
        valid = {"HOWL", "WAG", "GROWL", "BARK"}
        if v not in valid:
            raise ValueError(f"verdict must be one of {valid}, got '{v}'")
        return v

    @field_validator("q_score")
    @classmethod
    def validate_q_score(cls, v: float) -> float:
        # φ enforcement (LAW 5)
        if v > MAX_Q_SCORE:
            raise ValueError(f"q_score {v} exceeds φ⁻¹ limit of {MAX_Q_SCORE}")
        return v

    @model_validator(mode="after")
    def validate_consistency(self) -> "Judgment":
        """Verify verdict matches q_score."""
        q = self.q_score
        v = self.verdict
        if q >= 82.0 and v != "HOWL":
            raise ValueError(f"q_score={q} should be HOWL, got {v}")
        if 61.8 <= q < 82.0 and v != "WAG":
            raise ValueError(f"q_score={q} should be WAG, got {v}")
        if 38.2 <= q < 61.8 and v != "GROWL":
            raise ValueError(f"q_score={q} should be GROWL, got {v}")
        if q < 38.2 and v != "BARK":
            raise ValueError(f"q_score={q} should be BARK, got {v}")
        return self

    def to_dict(self) -> Dict:
        return {
            "judgment_id": self.judgment_id,
            "cell_id": self.cell.cell_id,
            "reality": self.cell.reality,
            "analysis": self.cell.analysis,
            "q_score": round(self.q_score, 3),
            "verdict": self.verdict,
            "confidence": round(self.confidence, 3),
            "axiom_scores": {k: round(v, 2) for k, v in self.axiom_scores.items()},
            "active_axioms": self.active_axioms,
            "consensus_reached": self.consensus_reached,
            "consensus_votes": self.consensus_votes,
            "residual_variance": round(self.residual_variance, 4),
            "unnameable_detected": self.unnameable_detected,
            "cost_usd": round(self.cost_usd, 6),
            "llm_calls": self.llm_calls,
            "timestamp": self.timestamp,
            "duration_ms": round(self.duration_ms, 1),
        }


# ════════════════════════════════════════════════════════════════════════════
# CONSENSUS RESULT (from PBFT Dogs)
# ════════════════════════════════════════════════════════════════════════════

class ConsensusResult(BaseModel):
    """Result of PBFT consensus among Dogs."""
    consensus: bool
    votes: int = Field(ge=0)
    quorum: int = Field(ge=0)

    # If consensus reached
    final_q_score: Optional[float] = Field(default=None, ge=0.0, le=MAX_Q_SCORE)
    final_verdict: Optional[str] = None
    final_confidence: Optional[float] = Field(default=None, ge=0.0, le=MAX_CONFIDENCE)

    # Failure reason
    reason: Optional[str] = None

    # Dog contributions
    dog_judgments: List[Dict] = Field(default_factory=list)

    @property
    def quorum_reached(self) -> bool:
        return self.votes >= self.quorum


# ════════════════════════════════════════════════════════════════════════════
# E-SCORE (Reputation)
# ════════════════════════════════════════════════════════════════════════════

class EScoreDimension(BaseModel):
    """One dimension of E-Score 7D."""
    name: str       # BURN/BUILD/JUDGE/RUN/SOCIAL/GRAPH/HOLD
    raw_score: float = Field(ge=0.0)
    weight: float = Field(gt=0.0)
    on_chain: bool = False


class EScore(BaseModel):
    """Agent reputation score across 7 φ-weighted dimensions."""
    agent_id: str
    total: float = Field(ge=0.0, le=100.0, description="Total E-Score [0, 100]")
    dimensions: Dict[str, EScoreDimension] = Field(default_factory=dict)
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())

    @property
    def trust_weight(self) -> float:
        """φ-amplified trust weight for consensus voting."""
        return (self.total / 100.0) ** PHI_INV


# ════════════════════════════════════════════════════════════════════════════
# LEARNING EVENTS
# ════════════════════════════════════════════════════════════════════════════

class LearningEvent(BaseModel):
    """Record of what the system learned from a judgment outcome."""
    event_id: str = Field(default_factory=new_id)
    loop_name: str      # Which loop (Q_LEARNING, THOMPSON, EWC, etc.)
    judgment_id: str
    state_key: str
    action: str
    reward: float       # Actual outcome reward
    q_delta: float = 0.0    # Change in Q-value
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())


class PerceptionEvent(BaseModel):
    """Raw perception data from a Watcher."""
    event_id: str = Field(default_factory=new_id)
    source: str         # CODE/SOLANA/MARKET/SOCIAL
    event_type: str     # file_changed, tx_confirmed, price_tick, etc.
    data: Dict[str, Any] = Field(default_factory=dict)
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())
