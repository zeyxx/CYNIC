"""Classify decisions into autonomous, consultation, or exploration."""

from __future__ import annotations

from collections import defaultdict
from enum import Enum
from typing import Any


class DecisionClass(Enum):
    """Classification for decisions."""

    AUTONOMOUS = "A"
    CONSULTATION = "B"
    EXPLORATION = "C"


class DecisionClassifier:
    """Learn which decisions should be autonomous, consultation, or exploration."""

    # Explicit classifications that shouldn't change
    ALWAYS_AUTONOMOUS = {
        "compress_traces",
        "compress_old_traces",
        "cleanup_disk",
        "backup_experiment_log",
        "create_snapshot",
    }

    ALWAYS_CONSULTATION = {
        "change_axiom_weight",
        "modify_relationship_memory",
        "delete_historical_data",
        "reset_learning_state",
    }

    ALWAYS_EXPLORATION = {
        "try_dog_combination",
        "test_alternative_algorithm",
        "hypothesize_axiom_relationship",
        "explore_edge_case",
    }

    def __init__(self):
        # Track approval/rejection patterns: action â†’ (approvals, rejections)
        self.decision_history: dict[str, tuple[int, int]] = defaultdict(lambda: (0, 0))
        # Learned classifications
        self.learned_classes: dict[str, DecisionClass] = {}

    def classify(self, decision: dict[str, Any]) -> DecisionClass:
        """Classify a decision."""
        action = decision.get("action", "unknown")

        # Check explicit classifications first
        if action in self.ALWAYS_AUTONOMOUS:
            return DecisionClass.AUTONOMOUS
        if action in self.ALWAYS_CONSULTATION:
            return DecisionClass.CONSULTATION
        if action in self.ALWAYS_EXPLORATION:
            return DecisionClass.EXPLORATION

        # Check learned classifications
        if action in self.learned_classes:
            return self.learned_classes[action]

        # Default: ask (consultation) for unknown decisions
        return DecisionClass.CONSULTATION

    def learn_approval(self, decision: dict[str, Any], actual_class: DecisionClass) -> None:
        """Record approval of a decision type."""
        action = decision.get("action", "unknown")
        approvals, rejections = self.decision_history.get(action, (0, 0))

        self.decision_history[action] = (approvals + 1, rejections)

        # If pattern emerges (3+ approvals), learn it
        if approvals + 1 >= 3:
            self.learned_classes[action] = actual_class

    def learn_rejection(self, decision: dict[str, Any]) -> None:
        """Record rejection of a decision type."""
        action = decision.get("action", "unknown")
        approvals, rejections = self.decision_history.get(action, (0, 0))

        self.decision_history[action] = (approvals, rejections + 1)

        # Keep as CONSULTATION if rejected
        # (don't auto-move to autonomous)
        if action not in self.learned_classes:
            self.learned_classes[action] = DecisionClass.CONSULTATION

    def get_statistics(self) -> dict[str, Any]:
        """Get classification statistics."""
        total_autonomous = len(
            [d for d in self.learned_classes.values() if d == DecisionClass.AUTONOMOUS]
        )
        total_consultation = len(
            [d for d in self.learned_classes.values() if d == DecisionClass.CONSULTATION]
        )
        total_exploration = len(
            [d for d in self.learned_classes.values() if d == DecisionClass.EXPLORATION]
        )

        return {
            "total_learned": len(self.learned_classes),
            "autonomous_count": total_autonomous,
            "consultation_count": total_consultation,
            "exploration_count": total_exploration,
        }
