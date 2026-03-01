"""
Unified State Models for CYNIC Consciousness

Consolidates immutable state contracts for judgments, learning, and value creation.
Enforces Ï†-bounds and allows extra metadata for topological flexibility.
"""

from __future__ import annotations

import dataclasses
import time
from dataclasses import dataclass, field
from types import MappingProxyType
from typing import Any

from pydantic import BaseModel, ConfigDict, Field, field_validator

from cynic.kernel.core.phi import MAX_CONFIDENCE, fibonacci


# â”€â”€ Base Model for Pydantic-based models with flexibility â”€â”€
class UnifiedModel(BaseModel):
    model_config = ConfigDict(extra="allow", frozen=False)


@dataclass(frozen=True)
class UnifiedJudgment:
    """Immutable record of a single judgment verdict.

    Frozen=True: Cannot be modified after creation.
    Dicts wrapped in MappingProxyType for deep immutability.
    """

    judgment_id: str
    verdict: str  # HOWL | WAG | GROWL | BARK
    q_score: float
    confidence: float
    axiom_scores: dict[str, float] = field(default_factory=dict)
    dog_votes: dict[Any, Any] = field(default_factory=dict)
    reality: str = "CODE"
    state_key: str = ""
    analysis: str = "JUDGE"
    time_dim: str = "PRESENT"
    lod: int = 1
    level_used: str = "REFLEX"
    reasoning: str = ""
    consensus_reason: str = ""
    timestamp: float = field(default_factory=time.time)
    consensus_reached: bool = True
    consensus_votes: int = 0
    consensus_quorum: int = 7
    latency_ms: float = 0.0

    def __post_init__(self):
        """Validate and wrap dicts in MappingProxyType for true immutability."""
        # Validate q_score is in [0, 100]
        if not (0 <= self.q_score <= 100):
            raise ValueError(f"q_score must be in [0, 100], got {self.q_score}")

        # Validate confidence is in [0, MAX_CONFIDENCE (0.618)]
        if not (0 <= self.confidence <= MAX_CONFIDENCE):
            raise ValueError(f"confidence must be in [0, {MAX_CONFIDENCE}], got {self.confidence}")

        # Validate verdict is in {HOWL, WAG, GROWL, BARK}
        valid_verdicts = {"HOWL", "WAG", "GROWL", "BARK"}
        if self.verdict not in valid_verdicts:
            raise ValueError(f"verdict must be in {valid_verdicts}, got {self.verdict}")

        # Wrap axiom_scores
        if self.axiom_scores and not isinstance(self.axiom_scores, MappingProxyType):
            object.__setattr__(self, "axiom_scores", MappingProxyType(self.axiom_scores))
        elif not self.axiom_scores:
            object.__setattr__(self, "axiom_scores", MappingProxyType({}))

        # Wrap dog_votes
        if self.dog_votes and not isinstance(self.dog_votes, MappingProxyType):
            object.__setattr__(self, "dog_votes", MappingProxyType(self.dog_votes))
        elif not self.dog_votes:
            object.__setattr__(self, "dog_votes", MappingProxyType({}))


@dataclass(frozen=True)
class UnifiedLearningOutcome:
    """Immutable record of predicted vs actual outcome."""

    judgment_id: str
    predicted_verdict: str
    actual_verdict: str
    satisfaction_rating: float  # [0, 1]
    timestamp: float = field(default_factory=time.time)


@dataclass(frozen=True)
class ValueCreation:
    """Phase 3: Immutable record of a value-creating event.

    Frozen=True: Cannot be modified after creation.
    Use evolve() method to create updated instances.
    """

    creation_id: str
    creator_id: str
    creation_type: str
    description: str
    timestamp: float = field(default_factory=time.time)
    direct_impact: float = 0.0
    indirect_impact: float = 0.0
    collective_impact: float = 0.0
    temporal_impact: float = 0.0

    def evolve(self, **kwargs) -> ValueCreation:
        """Create new instance with updated fields."""
        return dataclasses.replace(self, **kwargs)


@dataclass(frozen=True)
class ImpactMeasurement:
    """Phase 3: Computed impact metrics.

    Frozen=True: Cannot be modified after creation.
    Use evolve() method to create updated instances.
    """

    human_id: str
    total_impact: float
    dimension_scores: dict[str, float] = field(default_factory=dict)
    governance_weight: float = 0.01
    timestamp: float = field(default_factory=time.time)

    def __post_init__(self):
        """Wrap dimension_scores in MappingProxyType for deep immutability."""
        if self.dimension_scores and not isinstance(self.dimension_scores, MappingProxyType):
            object.__setattr__(self, "dimension_scores", MappingProxyType(self.dimension_scores))
        elif not self.dimension_scores:
            object.__setattr__(self, "dimension_scores", MappingProxyType({}))

    def evolve(self, **kwargs) -> ImpactMeasurement:
        """Create new instance with updated fields."""
        return dataclasses.replace(self, **kwargs)


@dataclass(frozen=True)
class GovernanceCommunity:
    """Immutable record of a community.

    Frozen=True: Cannot be modified after creation.
    Use evolve() method to create updated instances.
    """

    community_id: str
    name: str
    platform: str
    token_symbol: str = "CYNIC"
    voting_period_h: int = 72
    quorum_pct: float = 25.0
    threshold_pct: float = 50.0
    gasdf_enabled: bool = True
    near_address: str | None = None
    created_at: float = field(default_factory=time.time)

    def evolve(self, **kwargs) -> GovernanceCommunity:
        """Create new instance with updated fields."""
        return dataclasses.replace(self, **kwargs)


@dataclass(frozen=True)
class GovernanceProposal:
    """Immutable record of a governance proposal.

    Frozen=True: Cannot be modified after creation.
    Use evolve() method to create updated instances.
    """

    proposal_id: str
    community_id: str
    proposer_id: str
    title: str
    description: str
    category: str
    status: str = "PENDING"
    created_at: float = field(default_factory=time.time)
    voting_end: float = 0.0
    yes_votes: float = 0.0
    no_votes: float = 0.0
    judgment_id: str | None = None
    verdict: str | None = None

    def evolve(self, **kwargs) -> GovernanceProposal:
        """Create new instance with updated fields."""
        return dataclasses.replace(self, **kwargs)


@dataclass(frozen=True)
class GovernanceVote:
    """Immutable record of a user vote.

    Frozen=True: Cannot be modified after creation.
    Use evolve() method to create updated instances.
    """

    vote_id: str
    proposal_id: str
    voter_id: str
    choice: str
    weight: float = 1.0
    timestamp: float = field(default_factory=time.time)

    def evolve(self, **kwargs) -> GovernanceVote:
        """Create new instance with updated fields."""
        return dataclasses.replace(self, **kwargs)


# â”€â”€ Buffers â”€â”€


class JudgmentBuffer(BaseModel):
    """Immutable buffer of UnifiedJudgment entries.

    Uses tuple internally for immutability. add() returns new buffer instance.
    Automatically trims to fibonacci(11) = 89 entries.
    """

    buffer: tuple[UnifiedJudgment, ...] = Field(default_factory=tuple)
    max_len: int = Field(default=fibonacci(11), init=False)

    def add(self, item: UnifiedJudgment) -> JudgmentBuffer:
        """Add item and return new buffer (immutable operation)."""
        new_buffer = self.buffer + (item,)
        # Trim to maxlen if exceeded
        if len(new_buffer) > self.max_len:
            new_buffer = new_buffer[-self.max_len:]
        return JudgmentBuffer(buffer=new_buffer)

    def get_recent(self, n: int) -> list[UnifiedJudgment]:
        """Get last n items."""
        return list(self.buffer)[-n:] if n > 0 else []


class OutcomeBuffer(BaseModel):
    """Immutable buffer of UnifiedLearningOutcome entries.

    Uses tuple internally for immutability. add() returns new buffer instance.
    Automatically trims to fibonacci(10) = 55 entries.
    """

    buffer: tuple[UnifiedLearningOutcome, ...] = Field(default_factory=tuple)
    max_len: int = Field(default=fibonacci(10), init=False)

    def add(self, item: UnifiedLearningOutcome) -> OutcomeBuffer:
        """Add item and return new buffer (immutable operation)."""
        new_buffer = self.buffer + (item,)
        # Trim to maxlen if exceeded
        if len(new_buffer) > self.max_len:
            new_buffer = new_buffer[-self.max_len:]
        return OutcomeBuffer(buffer=new_buffer)

    def get_recent(self, n: int) -> list[UnifiedLearningOutcome]:
        """Get last n items."""
        return list(self.buffer)[-n:] if n > 0 else []


class ValueBuffer(BaseModel):
    """Immutable buffer of ValueCreation entries.

    Uses tuple internally for immutability. add() returns new buffer instance.
    Automatically trims to fibonacci(12) = 144 entries.
    """

    buffer: tuple[ValueCreation, ...] = Field(default_factory=tuple)
    max_len: int = Field(default=fibonacci(12), init=False)

    def add(self, item: ValueCreation) -> ValueBuffer:
        """Add item and return new buffer (immutable operation)."""
        new_buffer = self.buffer + (item,)
        # Trim to maxlen if exceeded
        if len(new_buffer) > self.max_len:
            new_buffer = new_buffer[-self.max_len:]
        return ValueBuffer(buffer=new_buffer)


class ImpactBuffer(BaseModel):
    """Immutable buffer of ImpactMeasurement entries.

    Uses tuple internally for immutability. add() returns new buffer instance.
    Automatically trims to fibonacci(10) = 55 entries.
    """

    buffer: tuple[ImpactMeasurement, ...] = Field(default_factory=tuple)
    max_len: int = Field(default=fibonacci(10), init=False)

    def add(self, item: ImpactMeasurement) -> ImpactBuffer:
        """Add item and return new buffer (immutable operation)."""
        new_buffer = self.buffer + (item,)
        # Trim to maxlen if exceeded
        if len(new_buffer) > self.max_len:
            new_buffer = new_buffer[-self.max_len:]
        return ImpactBuffer(buffer=new_buffer)


class CommunityBuffer(BaseModel):
    """Immutable buffer of GovernanceCommunity entries indexed by community_id.

    Uses dict internally but add() returns new buffer instance for immutability.
    Communities are accessed by ID for quick lookup.
    """

    model_config = ConfigDict(arbitrary_types_allowed=True)

    buffer: dict[str, GovernanceCommunity] = Field(default_factory=dict)

    def add(self, item: GovernanceCommunity) -> CommunityBuffer:
        """Add or update community and return new buffer (immutable operation)."""
        new_buffer = dict(self.buffer)
        new_buffer[item.community_id] = item
        return CommunityBuffer(buffer=new_buffer)

    def get(self, community_id: str) -> GovernanceCommunity | None:
        """Get community by ID."""
        return self.buffer.get(community_id)

    def all_communities(self) -> list[GovernanceCommunity]:
        """Get all communities."""
        return list(self.buffer.values())


class ProposalBuffer(BaseModel):
    """Immutable buffer of GovernanceProposal entries.

    Uses tuple internally for immutability. add() returns new buffer instance.
    Automatically trims to fibonacci(11) = 89 entries.
    """

    buffer: tuple[GovernanceProposal, ...] = Field(default_factory=tuple)
    max_len: int = Field(default=fibonacci(11), init=False)

    def add(self, item: GovernanceProposal) -> ProposalBuffer:
        """Add item and return new buffer (immutable operation)."""
        new_buffer = self.buffer + (item,)
        # Trim to maxlen if exceeded
        if len(new_buffer) > self.max_len:
            new_buffer = new_buffer[-self.max_len:]
        return ProposalBuffer(buffer=new_buffer)

    def get_recent(self, n: int) -> list[GovernanceProposal]:
        """Get last n proposals."""
        return list(self.buffer)[-n:] if n > 0 else []


class UnifiedConsciousState(BaseModel):
    """Consolidated state of CYNIC's consciousness."""

    model_config = ConfigDict(arbitrary_types_allowed=True)

    recent_judgments: JudgmentBuffer = Field(default_factory=JudgmentBuffer)
    learning_outcomes: OutcomeBuffer = Field(default_factory=OutcomeBuffer)
    value_creations: ValueBuffer = Field(default_factory=ValueBuffer)
    impact_measurements: ImpactBuffer = Field(default_factory=ImpactBuffer)
    communities: CommunityBuffer = Field(default_factory=CommunityBuffer)
    proposals: ProposalBuffer = Field(default_factory=ProposalBuffer)
    total_judgments: int = 0
    dog_agreement_scores: dict[int, float] = Field(default_factory=dict)
    consciousness_level: str = "REFLEX"
    active_axioms: tuple[str, ...] = Field(default_factory=tuple)
    emergent_states: dict[str, bool] = Field(default_factory=dict)
    activation_log: tuple[MappingProxyType, ...] = Field(default_factory=tuple)

    @field_validator("dog_agreement_scores", mode="before")
    @classmethod
    def validate_dog_agreement_scores(cls, v: dict[int, float]) -> dict[int, float]:
        """Validate that all dog agreement scores are in [0.0, 1.0]."""
        if v is None:
            return {}

        for dog_id, score in v.items():
            if not isinstance(score, int | float):
                raise ValueError(f"Dog {dog_id} score must be numeric, got {type(score)}")
            if not (0.0 <= score <= 1.0):
                raise ValueError(
                    f"Dog {dog_id} agreement score must be in [0.0, 1.0], " f"got {score}"
                )
        return v

    def get_consensus_score(self) -> float:
        """Calculate consensus score as average of dog agreement scores.

        Returns 0.0 if no dogs have agreement scores.
        """
        if not self.dog_agreement_scores:
            return 0.0
        scores = list(self.dog_agreement_scores.values())
        return sum(scores) / len(scores)

    def update_dog_agreement_score(self, dog_id: int, score: float) -> None:
        """Update a dog agreement score (copy-on-write, no in-place mutation)."""
        if not (0.0 <= score <= 1.0):
            raise ValueError(f"Dog agreement score must be in [0.0, 1.0], got {score}")

        self.dog_agreement_scores = {**self.dog_agreement_scores, dog_id: score}

    def add_axiom(self, axiom: str) -> None:
        """Add active axiom immutably (creates new tuple)."""
        if axiom not in self.active_axioms:
            self.active_axioms = self.active_axioms + (axiom,)

    def set_emergent_state(self, key: str, value: bool) -> None:
        """Set emergent state (copy-on-write, no in-place mutation)."""
        self.emergent_states = {**self.emergent_states, key: value}

    def log_activation(self, log_entry: dict) -> None:
        """Add to activation log immutably (creates new tuple with frozen entry)."""
        frozen_entry = MappingProxyType(log_entry.copy())
        self.activation_log = self.activation_log + (frozen_entry,)

    def add_judgment(self, j: UnifiedJudgment):
        """Add judgment to buffer (returns new buffer instance)."""
        self.recent_judgments = self.recent_judgments.add(j)
        self.total_judgments += 1

    def add_outcome(self, o: UnifiedLearningOutcome):
        """Add learning outcome to buffer (returns new buffer instance)."""
        self.learning_outcomes = self.learning_outcomes.add(o)

    def add_value_creation(self, v: ValueCreation):
        """Add value creation to buffer (returns new buffer instance)."""
        self.value_creations = self.value_creations.add(v)

    def add_impact_measurement(self, i: ImpactMeasurement):
        """Add impact measurement to buffer (returns new buffer instance)."""
        self.impact_measurements = self.impact_measurements.add(i)

    def add_community(self, c: GovernanceCommunity):
        """Add community to buffer (returns new buffer instance)."""
        self.communities = self.communities.add(c)

    def add_proposal(self, p: GovernanceProposal):
        """Add proposal to buffer (returns new buffer instance)."""
        self.proposals = self.proposals.add(p)

    async def reach_consensus_judgment(self, judgments: list[UnifiedJudgment]) -> UnifiedJudgment:
        """Reach consensus using PBFT engine (convenience wrapper)."""
        from cynic.kernel.organism.brain.consensus.pbft_engine import PBFTEngine

        engine = PBFTEngine(num_dogs=len(judgments) or 11)
        return await engine.reach_consensus(judgments)
