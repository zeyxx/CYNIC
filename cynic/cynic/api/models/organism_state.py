"""
Pydantic response models for organism state API endpoints.

These models are READ-ONLY (frozen=True) to prevent external mutations.
CYNIC's state is observed via HTTP — no direct mutations allowed.

Models:
- StateSnapshotResponse: Full system snapshot (consciousness level, counts, etc.)
- ConsciousnessResponse: Current consciousness level
- DogStatus: Single dog status (nested in DogsResponse)
- DogsResponse: All dogs and their status
- ProposedAction: Single proposed action (nested in ActionsResponse)
- ActionsResponse: All pending proposed actions
"""

from __future__ import annotations

from typing import Dict, List, Optional
from pydantic import BaseModel, Field, ConfigDict


# ════════════════════════════════════════════════════════════════════════════
# STATE SNAPSHOT
# ════════════════════════════════════════════════════════════════════════════


class StateSnapshotResponse(BaseModel):
    """
    Full state snapshot of the CYNIC organism at a point in time.

    Frozen (immutable) to ensure read-only access via API.
    """

    timestamp: float = Field(
        description="Unix timestamp (seconds) when snapshot was taken",
    )
    consciousness_level: str = Field(
        description="Current consciousness level: REFLEX|MICRO|MACRO|META",
    )
    judgment_count: int = Field(
        ge=0,
        description="Number of judgments made (recent window)",
    )
    dog_count: int = Field(
        ge=0,
        description="Number of active dogs in registry",
    )
    qtable_entries: int = Field(
        ge=0,
        description="Number of state-action pairs in Q-table",
    )
    residuals_count: int = Field(
        ge=0,
        description="Number of active residual detections",
    )
    pending_actions_count: int = Field(
        ge=0,
        description="Number of pending proposed actions",
    )

    model_config = ConfigDict(frozen=True)


# ════════════════════════════════════════════════════════════════════════════
# CONSCIOUSNESS
# ════════════════════════════════════════════════════════════════════════════


class ConsciousnessResponse(BaseModel):
    """
    Current consciousness level of the organism.

    Frozen (immutable).
    """

    level: str = Field(
        description="Consciousness level: REFLEX|MICRO|MACRO|META",
    )

    model_config = ConfigDict(frozen=True)


# ════════════════════════════════════════════════════════════════════════════
# DOGS
# ════════════════════════════════════════════════════════════════════════════


class DogStatus(BaseModel):
    """
    Real-time status of a single dog.

    Nested model used in DogsResponse.
    Frozen (immutable).
    """

    q_score: float = Field(
        ge=0.0,
        le=100.0,
        description="Last judgment Q-Score [0, 100]",
    )
    verdict: str = Field(
        description="Last verdict: BARK|GROWL|WAG|HOWL",
    )
    confidence: Optional[float] = Field(
        default=None,
        ge=0.0,
        le=0.618,
        description="Confidence bound [0, 0.618] (φ⁻¹)",
    )
    activity: Optional[str] = Field(
        default=None,
        description="Current activity: idle|judging|learning",
    )

    model_config = ConfigDict(frozen=True)


class DogsResponse(BaseModel):
    """
    All dogs and their current status.

    Frozen (immutable).
    """

    dogs: Dict[str, DogStatus] = Field(
        description="Map of dog_id → DogStatus",
    )
    count: int = Field(
        ge=0,
        description="Number of dogs",
    )

    model_config = ConfigDict(frozen=True)


# ════════════════════════════════════════════════════════════════════════════
# ACTIONS
# ════════════════════════════════════════════════════════════════════════════


class ProposedAction(BaseModel):
    """
    Single proposed action to be executed.

    Nested model used in ActionsResponse.
    Frozen (immutable).
    """

    action_id: str = Field(
        description="Unique action identifier",
    )
    action_type: str = Field(
        description="Type of action: INVESTIGATE|REFACTOR|ALERT|MONITOR",
    )
    priority: int = Field(
        ge=1,
        le=4,
        description="Priority level (1=critical, 4=fyi)",
    )
    description: Optional[str] = Field(
        default=None,
        description="Human-readable description",
    )

    model_config = ConfigDict(frozen=True)


class ActionsResponse(BaseModel):
    """
    All pending proposed actions.

    Frozen (immutable).
    """

    actions: List[ProposedAction] = Field(
        description="List of pending proposed actions",
    )
    count: int = Field(
        ge=0,
        description="Number of pending actions",
    )

    model_config = ConfigDict(frozen=True)
