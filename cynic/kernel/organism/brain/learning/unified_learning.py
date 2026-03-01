"""
Unified Q-Learning System for CYNIC

This module consolidates learning logic into a single Q-Table for all judgment domains.

Q-Learning maps: (predicted_verdict, actual_verdict) â†’ quality_score [0, 1]
Updated as community provides feedback (satisfaction_rating).
Used to adjust future prediction confidence based on past accuracy.

Key classes:
- UnifiedQTable: Q-Learning table with Ï†-bounded confidence
- LearningSession: Tracks outcomes and computes learning statistics
"""

from __future__ import annotations

from dataclasses import dataclass, field

from cynic.kernel.core.phi import PHI, PHI_INV
from cynic.kernel.core.unified_state import UnifiedLearningOutcome


@dataclass
class UnifiedQTable:
    """
    Q-Learning table for judgment prediction quality.

    Maps (predicted_verdict, actual_verdict) â†’ Q-value [0, 1].
    Updated as community provides feedback (satisfaction_rating).
    Used to adjust future prediction confidence based on past accuracy.

    Q-Learning Formula:
    Q_new = Q_old + learning_rate * (reward - Q_old)

    Where:
    - reward = satisfaction_rating (0-1, 1 = perfect prediction)
    - learning_rate = how much feedback affects Q-value

    Ï†-bounded constraints:
    - Confidence âˆˆ [0, PHI_INV] (max 0.618 = 61.8%)
    - Q-values âˆˆ [0, 1]
    - All verdicts âˆˆ {HOWL, WAG, GROWL, BARK}
    """

    # Q-values: Dict[(predicted_verdict, actual_verdict)] â†’ float [0, 1]
    values: dict[tuple[str, str], float] = field(default_factory=dict)

    # Learning rate (how much feedback affects Q-values)
    learning_rate: float = 0.1

    # Discount factor (weight of future predictions)
    discount_factor: float = 0.99

    # Phi constant for bounded values
    PHI: float = field(default=PHI, init=False)
    PHI_INV: float = field(default=PHI_INV, init=False)

    def __post_init__(self):
        """Initialize all verdict transitions with neutral values."""
        verdicts = ["HOWL", "WAG", "GROWL", "BARK"]
        for pred in verdicts:
            for actual in verdicts:
                key = (pred, actual)
                if key not in self.values:
                    # Start neutral (0.5 = 50% confidence in this transition)
                    self.values[key] = 0.5

    def get_q_value(self, predicted: str, actual: str) -> float:
        """
        Get Q-value for verdict transition.

        Args:
            predicted: Predicted verdict (HOWL, WAG, GROWL, BARK)
            actual: Actual verdict (HOWL, WAG, GROWL, BARK)

        Returns:
            Q-value [0, 1]
        """
        return self.values.get((predicted, actual), 0.5)

    def update(self, outcome: UnifiedLearningOutcome) -> None:
        """
        Update Q-value based on learning outcome.

        Q_new = Q_old + learning_rate * (reward - Q_old)

        Args:
            outcome: Learning outcome with predicted/actual verdicts and satisfaction
        """
        key = (outcome.predicted_verdict, outcome.actual_verdict)
        old_q = self.get_q_value(outcome.predicted_verdict, outcome.actual_verdict)

        # Reward = satisfaction_rating (0-1, 1 = perfect)
        reward = outcome.satisfaction_rating

        # Q-learning update rule
        new_q = old_q + self.learning_rate * (reward - old_q)

        # Clamp to [0, 1]
        self.values[key] = max(0.0, min(1.0, new_q))

    def get_prediction_confidence(self, verdict: str) -> float:
        """
        Estimate confidence in verdict based on Q-values.

        Average Q-values for this verdict across all actual outcomes.
        Higher average = more confident we predict this verdict correctly.

        Returns:
            confidence [0, PHI_INV], Ï†-bounded to max 0.618
        """
        matching_qs = [q for (pred, _), q in self.values.items() if pred == verdict]
        if not matching_qs:
            return 0.5

        avg_q = sum(matching_qs) / len(matching_qs)

        # Bound to Ï†â»Â¹ = 0.618 (max confidence)
        return min(avg_q, self.PHI_INV)

    def reset(self) -> None:
        """Reset Q-Table to initial neutral state."""
        for key in self.values:
            self.values[key] = 0.5


@dataclass
class LearningSession:
    """
    Tracks learning outcomes during a session.

    Accumulates outcomes, computes statistics, updates Q-Table.
    Used to measure and improve judgment quality over time.

    Attributes:
        outcomes: List of recorded learning outcomes
        q_table: Q-Learning table for this session
    """

    outcomes: list[UnifiedLearningOutcome] = field(default_factory=list)
    q_table: UnifiedQTable = field(default_factory=UnifiedQTable)

    def add_outcome(self, outcome: UnifiedLearningOutcome) -> None:
        """
        Record a learning outcome and update Q-Table.

        Args:
            outcome: UnifiedLearningOutcome to record
        """
        self.outcomes.append(outcome)
        self.q_table.update(outcome)

    def accuracy_rate(self) -> float:
        """
        Compute accuracy of predictions in this session.

        Returns:
            % of predictions that matched actual verdict (0-1).
        """
        if not self.outcomes:
            return 0.0

        correct = sum(1 for o in self.outcomes if o.predicted_verdict == o.actual_verdict)
        return correct / len(self.outcomes)

    def satisfaction_average(self) -> float:
        """
        Compute average satisfaction rating (0-1).

        Returns:
            Average satisfaction across all outcomes
        """
        if not self.outcomes:
            return 0.0

        return sum(o.satisfaction_rating for o in self.outcomes) / len(self.outcomes)

    def get_learning_rate(self) -> float:
        """
        Get current learning rate (how much feedback affects Q-values).

        Returns:
            Learning rate [0, 1]
        """
        return self.q_table.learning_rate

    def set_learning_rate(self, rate: float) -> None:
        """
        Set learning rate [0, 1].

        Args:
            rate: Learning rate in [0, 1]

        Raises:
            ValueError: If rate not in [0, 1]
        """
        if not (0.0 <= rate <= 1.0):
            raise ValueError(f"Learning rate must be [0, 1], got {rate}")
        self.q_table.learning_rate = rate
