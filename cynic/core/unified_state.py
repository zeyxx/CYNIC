"""
Unified State Models for CYNIC Consciousness

This module consolidates CYNIC's scattered state management into 3 core dataclasses:

1. UnifiedJudgment (frozen) — Represents a single judgment verdict
2. UnifiedLearningOutcome (frozen) — Records predicted vs. actual outcome
3. UnifiedConsciousState (mutable) — Main organism state container

Key principles:
- UnifiedJudgment and UnifiedLearningOutcome are IMMUTABLE (frozen=True)
- JudgmentBuffer and OutcomeBuffer auto-prune using Fibonacci-sized deques
  - JudgmentBuffer: F(11)=89 max items (BURN principle)
  - OutcomeBuffer: F(10)=55 max items (BURN principle)
- All confidence values respect φ-bound: max 0.618 (PHI_INV)
- All Q-scores are in [0, 100] range
- UnifiedConsciousState tracks consensus via dog_agreement_scores

Architecture:
┌─────────────────────────────────────────────────────────┐
│          UNIFIED CONSCIOUSNESS STATE                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  UnifiedConsciousState                                  │
│  ├─ recent_judgments: JudgmentBuffer (89 max)          │
│  │  └─ Stores: UnifiedJudgment (frozen)                │
│  ├─ learning_outcomes: OutcomeBuffer (55 max)          │
│  │  └─ Stores: UnifiedLearningOutcome (frozen)         │
│  ├─ total_judgments: int (counter)                     │
│  ├─ dog_agreement_scores: Dict[int, float]             │
│  └─ Methods: add_judgment(), add_outcome(), consensus()│
│                                                         │
└─────────────────────────────────────────────────────────┘
"""

from __future__ import annotations

from dataclasses import dataclass, field
from collections import deque
from typing import Dict, Any, Optional
from datetime import datetime
from types import MappingProxyType

from cynic.core.phi import fibonacci, PHI_INV, MAX_CONFIDENCE

# Import TYPE_CHECKING to avoid circular imports
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.consensus.pbft_engine import PBFTEngine


# ════════════════════════════════════════════════════════════════════════════
# IMMUTABLE DATACLASSES (frozen=True)
# ════════════════════════════════════════════════════════════════════════════


@dataclass(frozen=True)
class UnifiedJudgment:
    """
    Immutable representation of a single judgment verdict.

    φ-bounded constraints:
    - confidence ∈ [0, PHI_INV] (max 0.618 = 61.8%)
    - q_score ∈ [0, 100]
    - verdict ∈ {HOWL, WAG, GROWL, BARK}

    Attributes:
        judgment_id: Unique identifier (UUID4)
        verdict: The verdict string (HOWL, WAG, GROWL, BARK)
        q_score: Quality score [0, 100]
        confidence: Confidence level [0, 0.618]
        axiom_scores: Dict of axiom→score (FIDELITY, PHI, VERIFY, CULTURE, BURN)
        dog_votes: Dict of dog_id→{vote_data} (votes from 11 Dogs)
        reasoning: Explanation of the verdict
        latency_ms: Judgment computation time in milliseconds
        actual_verdict: Actual outcome (set after community feedback)
        satisfaction_rating: User satisfaction [0, 1] (set after feedback)
    """

    judgment_id: str
    verdict: str  # HOWL, WAG, GROWL, BARK
    q_score: float  # [0, 100]
    confidence: float  # [0, 0.618] — φ-bounded
    axiom_scores: Dict[str, float]  # {FIDELITY, PHI, VERIFY, CULTURE, BURN}
    dog_votes: Dict[int, Dict[str, Any]]  # {dog_id: {vote_data}}
    reasoning: str
    latency_ms: float
    actual_verdict: Optional[str] = None
    satisfaction_rating: Optional[float] = None

    def __post_init__(self):
        """Validate φ-bounds and enforce immutability of dict fields (frozen class)."""
        # Validate bounds
        if not (0.0 <= self.confidence <= MAX_CONFIDENCE):
            raise ValueError(
                f"confidence must be ∈ [0, {MAX_CONFIDENCE}], got {self.confidence}"
            )
        if not (0.0 <= self.q_score <= 100.0):
            raise ValueError(f"q_score must be ∈ [0, 100], got {self.q_score}")
        if self.verdict not in {"HOWL", "WAG", "GROWL", "BARK"}:
            raise ValueError(
                f"verdict must be HOWL/WAG/GROWL/BARK, got {self.verdict}"
            )
        if self.actual_verdict and self.actual_verdict not in {"HOWL", "WAG", "GROWL", "BARK"}:
            raise ValueError(
                f"actual_verdict must be HOWL/WAG/GROWL/BARK, got {self.actual_verdict}"
            )
        if self.satisfaction_rating is not None:
            if not (0.0 <= self.satisfaction_rating <= 1.0):
                raise ValueError(
                    f"satisfaction_rating must be ∈ [0, 1], got {self.satisfaction_rating}"
                )

        # Wrap dict fields with MappingProxyType to enforce true immutability
        # This prevents external code from mutating axiom_scores and dog_votes
        # even though the dataclass is frozen=True
        object.__setattr__(self, "axiom_scores", MappingProxyType(self.axiom_scores))
        object.__setattr__(self, "dog_votes", MappingProxyType(self.dog_votes))


@dataclass(frozen=True)
class UnifiedLearningOutcome:
    """
    Immutable record of predicted vs. actual verdict + satisfaction.

    Used by Q-Learning to update policy: (predicted → actual) + satisfaction.

    Attributes:
        judgment_id: References the judgment that made the prediction
        predicted_verdict: What CYNIC predicted (HOWL, WAG, GROWL, BARK)
        actual_verdict: What actually happened (HOWL, WAG, GROWL, BARK)
        satisfaction_rating: Community satisfaction [0, 1] with the verdict
    """

    judgment_id: str
    predicted_verdict: str  # HOWL, WAG, GROWL, BARK
    actual_verdict: str  # HOWL, WAG, GROWL, BARK
    satisfaction_rating: float  # [0, 1]

    def __post_init__(self):
        """Validate verdicts and satisfaction rating."""
        for verdict in [self.predicted_verdict, self.actual_verdict]:
            if verdict not in {"HOWL", "WAG", "GROWL", "BARK"}:
                raise ValueError(f"verdict must be HOWL/WAG/GROWL/BARK, got {verdict}")
        if not (0.0 <= self.satisfaction_rating <= 1.0):
            raise ValueError(
                f"satisfaction_rating must be ∈ [0, 1], got {self.satisfaction_rating}"
            )


# ════════════════════════════════════════════════════════════════════════════
# MUTABLE BUFFER CLASSES (auto-pruning via deque maxlen)
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class JudgmentBuffer:
    """
    Mutable buffer of recent judgments with automatic BURN-based pruning.

    BURN principle: When buffer reaches F(11)=89 capacity, oldest judgment
    is automatically discarded (BURN axiom).

    Attributes:
        buffer: deque with maxlen=89 (F(11)=89 from Fibonacci)
    """

    buffer: deque = field(default_factory=lambda: deque(maxlen=fibonacci(11)))

    def add(self, judgment: UnifiedJudgment) -> None:
        """
        Add a judgment to the buffer.

        If buffer is full (89 items), oldest is automatically discarded.

        Args:
            judgment: UnifiedJudgment to add
        """
        self.buffer.append(judgment)

    def get_recent(self, count: int) -> list[UnifiedJudgment]:
        """
        Get the most recent N judgments (newest last).

        Args:
            count: Number of judgments to retrieve

        Returns:
            List of UnifiedJudgment objects, newest at end
        """
        return list(self.buffer)[-count:] if count > 0 else []


@dataclass
class OutcomeBuffer:
    """
    Mutable buffer of learning outcomes with automatic BURN-based pruning.

    BURN principle: When buffer reaches F(10)=55 capacity, oldest outcome
    is automatically discarded.

    Attributes:
        buffer: deque with maxlen=55 (F(10)=55 from Fibonacci)
    """

    buffer: deque = field(default_factory=lambda: deque(maxlen=fibonacci(10)))

    def add(self, outcome: UnifiedLearningOutcome) -> None:
        """
        Add a learning outcome to the buffer.

        If buffer is full (55 items), oldest is automatically discarded.

        Args:
            outcome: UnifiedLearningOutcome to add
        """
        self.buffer.append(outcome)

    def get_recent(self, count: int) -> list[UnifiedLearningOutcome]:
        """
        Get the most recent N outcomes (newest last).

        Args:
            count: Number of outcomes to retrieve

        Returns:
            List of UnifiedLearningOutcome objects, newest at end
        """
        return list(self.buffer)[-count:] if count > 0 else []


# ════════════════════════════════════════════════════════════════════════════
# MAIN STATE CLASS
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class UnifiedConsciousState:
    """
    Mutable container for CYNIC's complete consciousness state.

    This is the unified view of:
    - Recent judgment history (immutable records in JudgmentBuffer)
    - Learning outcomes (immutable records in OutcomeBuffer)
    - Dog consensus scores (11 Dogs, each contributing agreement metric)
    - Total judgment counter (for statistics)

    Principles:
    - Buffers auto-prune via BURN (Fibonacci-bounded)
    - Consensus computed as average of dog_agreement_scores
    - No direct mutation of buffer contents (use add_judgment, add_outcome)
    - All buffer lookups return immutable UnifiedJudgment/UnifiedLearningOutcome objects

    Attributes:
        recent_judgments: JudgmentBuffer tracking last 89 judgments
        learning_outcomes: OutcomeBuffer tracking last 55 outcomes
        total_judgments: Counter of all judgments ever made
        dog_agreement_scores: Dict[dog_id (1-11)] → agreement score (0-1)
    """

    recent_judgments: JudgmentBuffer = field(default_factory=JudgmentBuffer)
    learning_outcomes: OutcomeBuffer = field(default_factory=OutcomeBuffer)
    total_judgments: int = 0
    dog_agreement_scores: Dict[int, float] = field(default_factory=dict)

    def __post_init__(self):
        """Validate dog_agreement_scores bounds [0, 1]."""
        for dog_id, score in self.dog_agreement_scores.items():
            if not (0.0 <= score <= 1.0):
                raise ValueError(
                    f"Dog {dog_id} agreement score must be in [0, 1], got {score}"
                )

    def add_judgment(self, judgment: UnifiedJudgment) -> None:
        """
        Record a new judgment in the state.

        Increments total_judgments counter and appends to recent_judgments buffer.

        Args:
            judgment: UnifiedJudgment to record
        """
        self.recent_judgments.add(judgment)
        self.total_judgments += 1

    def add_outcome(self, outcome: UnifiedLearningOutcome) -> None:
        """
        Record a learning outcome (feedback from community).

        Args:
            outcome: UnifiedLearningOutcome to record
        """
        self.learning_outcomes.add(outcome)

    def get_consensus_score(self) -> float:
        """
        Compute average dog agreement score.

        Returns average of all dog_agreement_scores, or 0.0 if no dogs.

        Returns:
            float: Average agreement [0, 1], or 0.0 if no dogs registered
        """
        if not self.dog_agreement_scores:
            return 0.0
        scores = list(self.dog_agreement_scores.values())
        return sum(scores) / len(scores) if scores else 0.0

    def get_recent_judgments(self, count: int) -> list[UnifiedJudgment]:
        """
        Get the most recent N judgments.

        Args:
            count: Number of judgments to retrieve

        Returns:
            List of UnifiedJudgment objects (immutable), newest last
        """
        return self.recent_judgments.get_recent(count)

    def get_recent_outcomes(self, count: int) -> list[UnifiedLearningOutcome]:
        """
        Get the most recent N learning outcomes.

        Args:
            count: Number of outcomes to retrieve

        Returns:
            List of UnifiedLearningOutcome objects (immutable), newest last
        """
        return self.learning_outcomes.get_recent(count)

    def update_from_outcome(self, outcome: UnifiedLearningOutcome) -> None:
        """
        Record learning outcome in consciousness and update state.

        This connects judgment → outcome → learning into unified consciousness.

        Args:
            outcome: UnifiedLearningOutcome to record
        """
        self.add_outcome(outcome)

    def get_learned_confidence(self, verdict: str) -> float:
        """
        Get confidence for verdict based on learned Q-values.

        This method is designed to be used with a Q-Table to adjust future
        judgment confidence based on past accuracy.

        Args:
            verdict: Verdict to get confidence for (HOWL, WAG, GROWL, BARK)

        Returns:
            Confidence value [0, 1]

        Note:
            Actual Q-Table integration happens in LearningSession.
            This method is a placeholder for future unified integration.
        """
        # Placeholder for Q-Table integration
        # In practice, would call: q_table.get_prediction_confidence(verdict)
        return 0.5

    async def reach_consensus_judgment(self, judgments: list[UnifiedJudgment]) -> UnifiedJudgment:
        """
        Reach Byzantine consensus among Dogs using PBFT algorithm.

        Uses PBFTEngine internally to aggregate judgments from multiple Dogs
        into a unified consensus verdict.

        Args:
            judgments: List of UnifiedJudgment objects from Dogs

        Returns:
            UnifiedJudgment representing consensus verdict with aggregated:
            - verdict: Consensus verdict (HOWL, WAG, GROWL, BARK)
            - confidence: Average confidence of agreeing Dogs
            - q_score: Average Q-score of agreeing Dogs
            - dog_votes: Aggregated votes from consensual Dogs

        Raises:
            ValueError: If judgments list is empty
        """
        from cynic.consensus.pbft_engine import PBFTEngine

        # Create PBFT engine with 11 Dogs (default)
        engine = PBFTEngine(num_dogs=11)

        # Reach consensus
        consensus = await engine.reach_consensus(judgments)

        return consensus
