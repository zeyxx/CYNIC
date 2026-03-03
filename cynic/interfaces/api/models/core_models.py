"""
CYNIC API Request/Response models - Pydantic v2

All API models are separate from internal models.
This lets the API contract evolve independently from the kernel.
"""
from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field, field_validator


# ==============================================================================
# ROOT
# ==============================================================================

class RootResponse(BaseModel):
    """Response from GET /."""
    status: str
    name: str
    phi: float = Field(alias="PHI")
    routes: list[str]


# ==============================================================================
# JUDGE
# ==============================================================================

class JudgeRequest(BaseModel):
    """POST /judge - judge any content through the CYNIC pipeline."""
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
    fractal_depth: int = Field(default=1, ge=1, le=55, description="Recursion depth for axiom facets")

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
        valid = {"REFLEX", "MICRO", "MACRO", "META"}
        v = v.upper()
        if v not in valid:
            raise ValueError(f"level must be one of {valid}")
        return v


class JudgeResponse(BaseModel):
    """Response from POST /judge."""
    judgment_id: str
    q_score: float
    verdict: str
    confidence: float
    axiom_scores: dict[str, float]
    dog_votes: dict[str, float]
    consensus_reached: bool
    consensus_votes: int
    cost_usd: float
    duration_ms: float
    reasoning: str | None = None


# ==============================================================================
# PERCEIVE
# ==============================================================================

class PerceiveRequest(BaseModel):
    """POST /perceive - feed raw data into the organism's sensory layer."""
    content: Any = Field(description="Raw data to perceive")
    reality: str = Field(default="SOLANA", description="Reality dimension")
    analysis: str = Field(default="PERCEIVE", description="Analysis mode")
    source: str = Field(default="api", description="Signal source")


class PerceiveResponse(BaseModel):
    """Response from POST /perceive."""
    status: str = Field(default="RECEIVED")
    judgment_id: str | None = None
    message: str


# ==============================================================================
# ACT
# ==============================================================================

class ActRequest(BaseModel):
    """POST /act/execute - execute a task via Claude Code."""
    prompt: str = Field(description="Task description for Claude Code")
    cwd: str | None = Field(default=None, description="Working directory for task")
    model: str | None = Field(default=None, description="Claude model override")
    timeout: float = Field(default=300.0, ge=10.0, le=3600.0, description="Max execution time in seconds")


class ActResponse(BaseModel):
    """Response from POST /act/execute."""
    success: bool
    session_id: str | None = None
    cost_usd: float = 0.0
    error: str | None = None


class TelemetryResponse(BaseModel):
    """Response from GET /act/telemetry."""
    stats: dict[str, Any]
    sessions: list[dict[str, Any]]
    message: str


# ==============================================================================
# MCP
# ==============================================================================

class MCPToolCallRequest(BaseModel):
    """POST /mcp/call - call an MCP tool."""
    name: str = Field(description="Tool name")
    arguments: dict[str, Any] = Field(default_factory=dict, description="Tool arguments")


class MCPToolCallResponse(BaseModel):
    """Response from MCP tool call."""
    content: list[dict[str, Any]]
    is_error: bool = False


class MCPResourceResponse(BaseModel):
    """Response from GET /mcp/resources."""
    resources: list[dict[str, Any]]


# ==============================================================================
# FEEDBACK & LEARNING
# ==============================================================================

class LearnRequest(BaseModel):
    """POST /learn - inject a learning signal directly into the Q-Table."""
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


class FeedbackRequest(BaseModel):
    """POST /feedback - user feedback on a judgment."""
    judgment_id: str
    correct: bool
    comment: str | None = None


class FeedbackResponse(BaseModel):
    """Response from POST /feedback."""
    success: bool
    message: str


# ==============================================================================
# POLICY & STATE
# ==============================================================================

class PolicyResponse(BaseModel):
    """GET /policy - current learned policy snapshot."""
    table_size: int
    top_states: list[dict[str, Any]]
    stats: dict[str, Any]


class AccountRequest(BaseModel):
    """POST /account/budget - update budget limits."""
    limit_usd: float = Field(ge=0.0)


class AccountResponse(BaseModel):
    """Response from account endpoints."""
    limit_usd: float
    spent_usd: float
    remaining_usd: float
    usage_pct: float


# ==============================================================================
# INFRASTRUCTURE & HEALTH
# ==============================================================================

class HealthResponse(BaseModel):
    """GET /health - kernel vital signs."""
    status: str
    uptime_s: float
    consciousness: dict[str, Any]
    dogs: list[str]
    learning: dict[str, Any]
    scheduler: dict[str, Any]
    llm_adapters: list[str]
    judgments_total: int
    phi: float
    storage: dict[str, Any]


class HealthEventsResponse(BaseModel):
    """Response from GET /health/events."""
    status: str
    event_handlers: dict[str, Any]
    judgment_pipeline: dict[str, Any]
    timestamp: float


class DogHealthMetric(BaseModel):
    """Health metrics for a single Dog."""
    name: str
    status: str
    judgments: int
    errors: int
    error_rate: float
    avg_latency_ms: float
    priority: float


class HealthFullResponse(BaseModel):
    """Response from GET /health/full."""
    timestamp: float
    status: str
    uptime_seconds: float
    components: dict[str, dict[str, Any]]
    dogs: dict[str, Any]
    learning: dict[str, Any]
    resources: dict[str, Any]


class HealthReadyResponse(BaseModel):
    """Response from GET /health/ready."""
    status: str
    waited_seconds: float
    ready_components: dict[str, bool]
    timestamp: float


class StatsResponse(BaseModel):
    """GET /stats - cumulative kernel performance."""
    judgments: dict[str, Any]
    learning: dict[str, Any]
    metabolism: dict[str, Any]
    timestamp: float


class EventSnapshot(BaseModel):
    """Single event snapshot for timeline."""
    event_id: str
    type: str
    timestamp: float
    source: str
    payload: dict[str, Any]


class EcosystemStateResponse(BaseModel):
    """Full ecosystem snapshot."""
    instance_id: str
    health: HealthResponse
    active_dogs: list[str]
    recent_events: list[EventSnapshot]


class DecisionPathStage(BaseModel):
    """Single stage in a decision trace."""
    stage: str
    duration_ms: float
    outcome: dict[str, Any]


class DecisionTraceResponse(BaseModel):
    """Full decision audit trail."""
    trace_id: str
    judgment_id: str
    stages: list[DecisionPathStage]
    final_verdict: str
    confidence: float


class DecisionTraceSingleResponse(BaseModel):
    """Response for a single decision trace."""
    trace: dict[str, Any]


class DecisionTracesResponse(BaseModel):
    """Response for multiple decision traces."""
    traces: list[dict[str, Any]]
    count: int
    verdict: str | None = None
    component: str | None = None


class TopologyConsciousnessResponse(BaseModel):
    """Kernel topology and handler graph."""
    instance_id: str
    handlers: dict[str, Any]
    topology: dict[str, Any]


class GuardrailDecision(BaseModel):
    """Result of a safety guardrail check."""
    guardrail: str
    passed: bool
    reason: str | None = None


class NervousSystemAuditResponse(BaseModel):
    """Complete event + decision history."""
    all_events: list[dict[str, Any]]
    decision_reasons: list[dict[str, Any]]
    loop_integrity_checks: list[dict[str, Any]]
    event_count: int
    decision_count: int
    timestamp: float


class SelfAwarenessResponse(BaseModel):
    """Organism's self-model snapshot."""
    self_identity: dict[str, Any]
    self_assessment: dict[str, Any]
    timestamp: float


class JournalEventsResponse(BaseModel):
    """Generic response for journal event queries."""
    events: list[dict[str, Any]]
    count: int
    event_type: str | None = None
    source: str | None = None
    category: str | None = None


class JournalStatsResponse(BaseModel):
    """Response for journal statistics."""
    stats: dict[str, Any]


class LoopClosureResponse(BaseModel):
    """Response for loop closure queries."""
    open_cycles: list[dict[str, Any]] | None = None
    stalled_cycles: list[dict[str, Any]] | None = None
    orphan_judgments: list[dict[str, Any]] | None = None
    recent_closures: list[dict[str, Any]] | None = None
    count: int
    threshold_ms: float | None = None
    complete_only: bool | None = None
