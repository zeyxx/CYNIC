"""
CYNIC API Request/Response models — Pydantic v2

All API models are separate from internal models.
This lets the API contract evolve independently from the kernel.
"""
from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field, field_validator

from cynic.core.phi import MAX_Q_SCORE, MAX_CONFIDENCE


# ════════════════════════════════════════════════════════════════════════════
# JUDGE
# ════════════════════════════════════════════════════════════════════════════

class JudgeRequest(BaseModel):
    """POST /judge — judge any content through the CYNIC pipeline."""
    content: Any = Field(description="Content to judge (code, text, data...)")
    reality: str = Field(
        default="CODE",
        description="Reality dimension: CODE/SOLANA/MARKET/SOCIAL/HUMAN/CYNIC/COSMOS",
    )
    analysis: str = Field(
        default="JUDGE",
        description="Analysis type: PERCEIVE/JUDGE/DECIDE/ACT/LEARN/ACCOUNT/EMERGE",
    )
    context: str = Field(default="", description="Human-readable context for LLM scoring")
    time_dim: str | None = Field(
        default=None,
        description="Time dimension: PAST/PRESENT/FUTURE/CYCLE/TREND/EMERGENCE/TRANSCENDENCE (inferred if None)",
    )
    level: str | None = Field(
        default=None,
        description="Consciousness level: REFLEX/MICRO/MACRO (auto-selected if None)",
    )
    budget_usd: float = Field(default=0.01, ge=0.0, description="Max USD budget for this judgment")
    lod: int = Field(default=1, ge=0, le=3, description="Level of Detail (0=pattern, 3=LLM)")

    @field_validator("reality")
    @classmethod
    def validate_reality(cls, v: str) -> str:
        valid = {"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"}
        v = v.upper()
        if v not in valid:
            raise ValueError(f"reality must be one of {valid}")
        return v

    @field_validator("level")
    @classmethod
    def validate_level(cls, v: str | None) -> str | None:
        if v is None:
            return None
        valid = {"REFLEX", "MICRO", "MACRO"}
        v = v.upper()
        if v not in valid:
            raise ValueError(f"level must be one of {valid}")
        return v


class JudgeResponse(BaseModel):
    """Response from POST /judge."""
    judgment_id: str
    q_score: float = Field(description="Quality score [0, 100]")
    verdict: str = Field(description="HOWL/WAG/GROWL/BARK")
    confidence: float = Field(description="Confidence [0, 0.618]")
    axiom_scores: dict[str, float]
    dog_votes: dict[str, float]
    consensus_reached: bool
    consensus_votes: int
    residual_variance: float = 0.0
    unnameable_detected: bool = False
    cost_usd: float
    llm_calls: int
    duration_ms: float
    level_used: str


# ════════════════════════════════════════════════════════════════════════════
# PERCEIVE (JS hooks → Python kernel)
# ════════════════════════════════════════════════════════════════════════════

class PerceiveRequest(BaseModel):
    """
    POST /perceive — receive raw perception from JS hooks (or any source).

    This is the bridge: JS thin hooks POST here instead of to JS daemon.
    Python kernel runs the full judgment cycle on the incoming perception.
    """
    source: str = Field(description="Source identifier (hook name, service name, etc.)")
    reality: str = Field(
        default="CYNIC",
        description="Reality dimension of this perception",
    )
    data: Any = Field(description="Raw perception data (any JSON-serializable)")
    context: str = Field(default="", description="Human-readable context")
    time_dim: str | None = Field(
        default=None,
        description="Time dimension: PAST/PRESENT/FUTURE/CYCLE/TREND/EMERGENCE/TRANSCENDENCE (inferred if None)",
    )
    run_judgment: bool = Field(
        default=True,
        description="If True, run full judgment pipeline on this perception",
    )
    level: str | None = Field(default="REFLEX", description="Judgment level")


class PerceiveResponse(BaseModel):
    """Response from POST /perceive."""
    cell_id: str
    source: str
    reality: str
    judgment: JudgeResponse | None = None
    enqueued: bool = False
    message: str = ""


# ════════════════════════════════════════════════════════════════════════════
# LEARN
# ════════════════════════════════════════════════════════════════════════════

class LearnRequest(BaseModel):
    """POST /learn — inject a learning signal directly into the Q-Table."""
    state_key: str = Field(description="State key (e.g. 'CODE:JUDGE:PRESENT:1')")
    action: str = Field(description="Verdict action: BARK/GROWL/WAG/HOWL")
    reward: float = Field(ge=0.0, le=1.0, description="Normalized reward [0, 1]")
    judgment_id: str = Field(default="", description="Source judgment ID")
    loop_name: str = Field(default="API_DIRECT", description="Learning loop identifier")

    @field_validator("action")
    @classmethod
    def validate_action(cls, v: str) -> str:
        valid = {"BARK", "GROWL", "WAG", "HOWL"}
        v = v.upper()
        if v not in valid:
            raise ValueError(f"action must be one of {valid}")
        return v


class LearnResponse(BaseModel):
    """Response from POST /learn."""
    state_key: str
    action: str
    q_value: float
    visits: int
    confidence: float
    wins: int
    losses: int


# ════════════════════════════════════════════════════════════════════════════
# POLICY
# ════════════════════════════════════════════════════════════════════════════

class PolicyResponse(BaseModel):
    """Response from GET /policy/{state_key}."""
    state_key: str
    mode: str  # exploit / explore
    recommended_action: str
    q_value: float
    confidence: float
    top_actions: list[dict[str, Any]]


# ════════════════════════════════════════════════════════════════════════════
# FEEDBACK (explicit user reward signal → Q-Table update)
# ════════════════════════════════════════════════════════════════════════════

class FeedbackRequest(BaseModel):
    """POST /feedback — user rates the last kernel judgment (1=bad, 5=good)."""
    rating: int = Field(ge=1, le=5, description="1=very bad, 5=very good")


class FeedbackResponse(BaseModel):
    """Response from POST /feedback."""
    state_key: str
    action: str
    reward: float = Field(description="Normalized reward applied to Q-Table [0.1, 0.9]")
    q_value: float = Field(description="Updated Q(s,a) after feedback")
    visits: int
    message: str


# ════════════════════════════════════════════════════════════════════════════
# HEALTH
# ════════════════════════════════════════════════════════════════════════════

class HealthResponse(BaseModel):
    """Response from GET /health."""
    status: str  # alive / degraded / dead
    version: str = "2.0.0"
    uptime_s: float
    consciousness: dict[str, Any]
    dogs: list[str]
    learning: dict[str, Any]
    scheduler: dict[str, Any] = Field(default_factory=dict)
    llm_adapters: list[str]
    judgments_total: int
    phi: float  # 1.618... (always displayed as reminder)
    storage: dict[str, Any] = Field(default_factory=dict)  # T02: surreal/asyncpg status


# ════════════════════════════════════════════════════════════════════════════
# STATS
# ════════════════════════════════════════════════════════════════════════════

class StatsResponse(BaseModel):
    """Response from GET /stats — detailed kernel metrics."""
    judgments: dict[str, Any]
    learning: dict[str, Any]
    top_states: list[dict[str, Any]]
    consciousness: dict[str, Any]
    compressor: dict[str, Any] = Field(default_factory=dict)  # γ2 ContextCompressor stats
