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
- AccountStatusResponse: Account metrics (balance, spend, learn_rate, reputation)
- AgentScore: Per-agent E-Score breakdown (nested in EScoreResponse)
- EScoreResponse: All agent E-Scores and reputation dimensions
- PolicyAction: Single learned policy action (nested in PolicyActionsResponse)
- PolicyActionsResponse: Learned best actions per state from Q-table
- PolicyStatsResponse: Policy coverage and learning statistics
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


# ════════════════════════════════════════════════════════════════════════════
# ACCOUNT
# ════════════════════════════════════════════════════════════════════════════


class AccountStatusResponse(BaseModel):
    """
    Account and budget tracking for CYNIC organism.

    Frozen (immutable) to ensure read-only access via API.
    """

    timestamp: float = Field(
        description="Unix timestamp (seconds) when snapshot was taken",
    )
    balance_usd: float = Field(
        ge=0.0,
        description="Session budget (USD) available",
    )
    spent_usd: float = Field(
        ge=0.0,
        description="Total cumulative spend (USD) in session",
    )
    budget_remaining_usd: float = Field(
        ge=0.0,
        description="Budget remaining (USD) = balance - spent",
    )
    learn_rate: float = Field(
        ge=0.0,
        le=0.618,
        description="Learning rate [0, φ⁻¹=0.618]",
    )
    reputation: float = Field(
        ge=0.0,
        le=100.0,
        description="Overall reputation score [0, 100]",
    )

    model_config = ConfigDict(frozen=True)


# ════════════════════════════════════════════════════════════════════════════
# E-SCORE (REPUTATION)
# ════════════════════════════════════════════════════════════════════════════


class AgentScore(BaseModel):
    """
    E-Score reputation breakdown for a single agent (dog).

    Nested model used in EScoreResponse.
    Frozen (immutable).

    7 dimensions (φ-weighted):
      BURN:   Irreversible token burn (commitment signal)
      BUILD:  Code/artifact quality contributions
      JUDGE:  Judgment accuracy (prediction vs reality)
      RUN:    Execution reliability
      SOCIAL: Community engagement quality
      GRAPH:  Network connectivity (trust graph)
      HOLD:   Long-term commitment
    """

    agent_id: str = Field(
        description="Agent identifier (e.g., 'SAGE', 'GUARDIAN', 'ANALYST')",
    )
    burn: float = Field(
        ge=0.0,
        le=100.0,
        description="BURN E-Score dimension [0, 100]",
    )
    build: float = Field(
        ge=0.0,
        le=100.0,
        description="BUILD E-Score dimension [0, 100]",
    )
    judge: float = Field(
        ge=0.0,
        le=100.0,
        description="JUDGE E-Score dimension [0, 100]",
    )
    run: float = Field(
        ge=0.0,
        le=100.0,
        description="RUN E-Score dimension [0, 100]",
    )
    social: float = Field(
        ge=0.0,
        le=100.0,
        description="SOCIAL E-Score dimension [0, 100]",
    )
    graph: float = Field(
        ge=0.0,
        le=100.0,
        description="GRAPH E-Score dimension [0, 100]",
    )
    hold: float = Field(
        ge=0.0,
        le=100.0,
        description="HOLD E-Score dimension [0, 100]",
    )
    total: float = Field(
        ge=0.0,
        le=100.0,
        description="Aggregate E-Score (φ-weighted geometric mean) [0, 100]",
    )

    model_config = ConfigDict(frozen=True)


class EScoreResponse(BaseModel):
    """
    All agents and their current E-Score reputation.

    Frozen (immutable).
    """

    timestamp: float = Field(
        description="Unix timestamp (seconds) when snapshot was taken",
    )
    agents: List[AgentScore] = Field(
        description="List of agent E-Scores",
    )
    count: int = Field(
        ge=0,
        description="Number of agents with E-Score data",
    )

    model_config = ConfigDict(frozen=True)


# ════════════════════════════════════════════════════════════════════════════
# POLICY
# ════════════════════════════════════════════════════════════════════════════


class PolicyAction(BaseModel):
    """
    One learned policy action — best action for a state in the Q-table.

    Nested model used in PolicyActionsResponse.
    Frozen (immutable).
    """

    state_key: str = Field(
        description="State identifier (e.g., 'CODE:JUDGE:PRESENT:1')",
    )
    best_action: str = Field(
        description="Highest Q-value action: BARK|GROWL|WAG|HOWL",
    )
    q_value: float = Field(
        ge=0.0,
        le=1.0,
        description="Q-value estimate for best action [0, 1]",
    )
    confidence: float = Field(
        ge=0.0,
        le=0.618,
        description="Confidence in prediction [0, φ⁻¹=0.618]",
    )

    model_config = ConfigDict(frozen=True)


class PolicyActionsResponse(BaseModel):
    """
    Learned best actions per state from Q-table.

    Shows CYNIC's learned policy: what action should be taken in each observed state.
    Frozen (immutable).
    """

    timestamp: float = Field(
        description="Unix timestamp (seconds) when snapshot was taken",
    )
    actions: List[PolicyAction] = Field(
        default_factory=list,
        description="List of learned policy actions (state → best action)",
    )
    count: int = Field(
        ge=0,
        description="Number of learned policy actions",
    )

    model_config = ConfigDict(frozen=True)


class PolicyStatsResponse(BaseModel):
    """
    Policy coverage and learning statistics from Q-table.

    Measures how much of the state space has been explored and learned.
    Frozen (immutable).
    """

    timestamp: float = Field(
        description="Unix timestamp (seconds) when snapshot was taken",
    )
    total_states: int = Field(
        ge=0,
        description="Number of distinct states seen in Q-table",
    )
    total_actions_per_state: float = Field(
        ge=0.0,
        description="Average number of actions learned per state",
    )
    policy_coverage: float = Field(
        ge=0.0,
        le=1.0,
        description="Fraction of states with learned policy [0, 1]",
    )
    average_confidence: float = Field(
        ge=0.0,
        le=0.618,
        description="Mean confidence across all learned actions [0, φ⁻¹=0.618]",
    )
    max_q_value: float = Field(
        ge=0.0,
        le=1.0,
        description="Highest Q-value in table [0, 1]",
    )

    model_config = ConfigDict(frozen=True)
