"""
CYNIC Judgment Models — Pydantic v2 (Fortress Mode)

All Pydantic models for the judgment pipeline.
Enforces immutability, strict typing, and forbids extra fields to prevent LLM hallucination.
"""

from __future__ import annotations

import uuid
from datetime import datetime
from typing import Any, Optional

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator

from cynic.kernel.core.phi import MAX_CONFIDENCE, MAX_Q_SCORE, PHI_INV

# Global configuration for all models: Frozen (Immutable) and No extra fields allowed.
_STRICT = ConfigDict(frozen=True, extra="forbid")

def new_id() -> str:
    """Generate a new UUID4 string identifier."""
    return str(uuid.uuid4())


class Cell(BaseModel):
    """
    A specific point/state in the ∞^N hypercube.
    """
    model_config = _STRICT

    # Core coordinates
    reality: str = Field(description="Reality dimension (CODE/SOLANA/MARKET/SOCIAL/HUMAN/CYNIC/COSMOS)")
    analysis: str = Field(description="Analysis type (PERCEIVE/JUDGE/DECIDE/ACT/LEARN/ACCOUNT/EMERGE)")
    time_dim: str = Field(default="PRESENT", description="Time dimension (PAST/PRESENT/FUTURE/CYCLE/TREND/EMERGENCE/TRANSCENDENCE)")

    # Content
    content: Any = Field(description="The actual data being judged")
    context: str = Field(default="", description="Context string for LLM scoring")

    # Structural dimensions
    dogs_active: tuple[str, ...] = Field(default_factory=tuple)
    lod: int = Field(default=1, ge=0, le=3)
    consciousness: int = Field(default=0, ge=0, le=6)

    # Economic dimensions
    budget_usd: float = Field(default=1.0, ge=0.0)

    # Epistemic dimensions
    novelty: float = Field(default=0.5, ge=0.0, le=1.0)
    complexity: float = Field(default=0.5, ge=0.0, le=1.0)
    risk: float = Field(default=0.0, ge=0.0, le=1.0)

    # Metadata
    cell_id: str = Field(default_factory=new_id)
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())
    metadata: dict[str, Any] = Field(default_factory=dict)

    def state_key(self) -> str:
        return f"{self.reality}:{self.analysis}:{self.time_dim}:{self.lod}"


class Judgment(BaseModel):
    """
    The immutable result of judging a Cell.
    """
    model_config = _STRICT

    judgment_id: str = Field(default_factory=new_id)
    cell: Cell

    # Output (φ-bounded)
    q_score: float = Field(ge=0.0, le=MAX_Q_SCORE)
    verdict: str = Field(description="HOWL/WAG/GROWL/BARK")
    confidence: float = Field(ge=0.0, le=MAX_CONFIDENCE)
    reasoning: str = Field(default="")

    # Breakdown
    axiom_scores: dict[str, float] = Field(default_factory=dict)
    active_axioms: tuple[str, ...] = Field(default_factory=tuple)

    # Consensus info
    dog_votes: dict[str, float] = Field(default_factory=dict)
    consensus_votes: int = Field(default=0)
    consensus_quorum: int = Field(default=7)
    consensus_reached: bool = Field(default=False)

    # Economics
    cost_usd: float = Field(default=0.0, ge=0.0)
    llm_calls: int = Field(default=0, ge=0)

    # THE_UNNAMEABLE
    residual_variance: float = Field(default=0.0, ge=0.0)
    unnameable_detected: bool = Field(default=False)

    # Metadata
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())
    duration_ms: float = Field(default=0.0, ge=0.0)

    @field_validator("verdict")
    @classmethod
    def validate_verdict(cls, v: str) -> str:
        valid = {"HOWL", "WAG", "GROWL", "BARK"}
        if v not in valid:
            raise ValueError(f"verdict must be one of {valid}")
        return v

    def to_dict(self) -> dict:
        return self.model_dump()


class ConsensusResult(BaseModel):
    """Result of PBFT consensus among Dogs."""
    model_config = _STRICT

    consensus: bool
    votes: int = Field(ge=0)
    quorum: int = Field(ge=0)
    final_q_score: Optional[float] = Field(default=None, ge=0.0, le=MAX_Q_SCORE)
    final_verdict: Optional[str] = None
    final_confidence: Optional[float] = Field(default=None, ge=0.0, le=MAX_CONFIDENCE)
    reason: Optional[str] = None
    dog_judgments: tuple[dict[str, Any], ...] = Field(default_factory=tuple)


class EScoreDimension(BaseModel):
    """One dimension of E-Score 7D."""
    model_config = _STRICT
    name: str
    raw_score: float = Field(ge=0.0)
    weight: float = Field(gt=0.0)
    on_chain: bool = False


class EScore(BaseModel):
    """Agent reputation score across 7 φ-weighted dimensions."""
    model_config = _STRICT
    agent_id: str
    total: float = Field(ge=0.0, le=100.0)
    dimensions: dict[str, EScoreDimension] = Field(default_factory=dict)
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())


class LearningEvent(BaseModel):
    """Record of what the system learned."""
    model_config = _STRICT
    event_id: str = Field(default_factory=new_id)
    loop_name: str
    judgment_id: str
    state_key: str
    action: str
    reward: float
    q_delta: float = 0.0
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())


class PerceptionEvent(BaseModel):
    """Raw perception data from a Watcher."""
    model_config = _STRICT
    event_id: str = Field(default_factory=new_id)
    source: str
    event_type: str
    data: dict[str, Any] = Field(default_factory=dict)
    timestamp: float = Field(default_factory=lambda: datetime.now().timestamp())
