"""
MCP Models — Type-safe request/response definitions.

Three core operations:
- OBSERVE: Get current CYNIC state (component health, judgments, events)
- ACT: Execute a Claude Code action proposal
- LEARN: Submit human feedback for Q-Table learning

All Pydantic v2 with JSON serialization.
"""
from __future__ import annotations

from typing import Any, Optional
from datetime import datetime
from pydantic import BaseModel, Field


# ════════════════════════════════════════════════════════════════════════════
# OBSERVE — Get CYNIC state
# ════════════════════════════════════════════════════════════════════════════

class ComponentHealthSnapshot(BaseModel):
    """Health status of one component."""
    name: str
    status: str  # HEALTHY, DEGRADED, STALLED, FAILED
    timestamp: float
    judgment_count: int
    last_judgment_id: Optional[str] = None


class RegistrySnapshot(BaseModel):
    """Current state of all CYNIC components."""
    timestamp: float
    components: list[ComponentHealthSnapshot]
    health_summary: dict[str, int]  # e.g., {"HEALTHY": 9, "DEGRADED": 1}
    total_components: int


class ObserveRequest(BaseModel):
    """Request current CYNIC state."""
    include_judgments: bool = True
    include_events: bool = True
    max_events: int = Field(default=50, ge=1, le=1000)


class ObserveResponse(BaseModel):
    """Response with CYNIC state snapshot."""
    timestamp: float
    registry_snapshot: RegistrySnapshot
    recent_judgments: list[dict] = Field(default_factory=list)
    recent_events: list[dict] = Field(default_factory=list)
    status: str  # "ok" or error message


# ════════════════════════════════════════════════════════════════════════════
# ACT — Execute Claude Code action
# ════════════════════════════════════════════════════════════════════════════

class ActionProposal(BaseModel):
    """Claude Code action proposal for CYNIC to execute."""
    action_id: str
    action_type: str  # INVESTIGATE, REFACTOR, ALERT, MONITOR
    priority: int  # 1=highest
    prompt: str
    context: dict = Field(default_factory=dict)
    metadata: dict = Field(default_factory=dict)


class ActRequest(BaseModel):
    """Request CYNIC to execute an action."""
    action: ActionProposal
    timeout_s: float = Field(default=30.0, gt=0)
    allow_learning: bool = True  # Feed results back to Q-Table


class ActResult(BaseModel):
    """Result of action execution."""
    action_id: str
    success: bool
    output: Optional[str] = None
    error: Optional[str] = None
    execution_time_s: float
    learning_signal: Optional[dict] = None  # For Q-Table update


class ActResponse(BaseModel):
    """Response from action execution."""
    timestamp: float
    result: ActResult
    status: str  # "ok", "timeout", "error"


# ════════════════════════════════════════════════════════════════════════════
# LEARN — Human feedback
# ════════════════════════════════════════════════════════════════════════════

class FeedbackSignal(BaseModel):
    """Human feedback on a CYNIC judgment."""
    judgment_id: str
    rating: float = Field(ge=-1.0, le=1.0)  # -1 (bad) to +1 (good)
    comment: Optional[str] = None
    correction: Optional[str] = None  # Human's better answer


class LearnRequest(BaseModel):
    """Request CYNIC to learn from human feedback."""
    signal: FeedbackSignal
    update_qtable: bool = True


class LearnResult(BaseModel):
    """Result of learning update."""
    judgment_id: str
    qtable_updated: bool
    new_q_score: Optional[float] = None
    learning_rate_applied: float


class LearnResponse(BaseModel):
    """Response from learning operation."""
    timestamp: float
    result: LearnResult
    status: str  # "ok" or error message


# ════════════════════════════════════════════════════════════════════════════
# Error Response
# ════════════════════════════════════════════════════════════════════════════

class ErrorResponse(BaseModel):
    """Standard error response."""
    timestamp: float
    status: str
    error: str
    details: dict = Field(default_factory=dict)
