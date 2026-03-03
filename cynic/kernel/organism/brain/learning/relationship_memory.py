"""Relationship memory for user preferences and learning."""

from __future__ import annotations

from dataclasses import dataclass, field, replace

from cynic.kernel.organism.brain.dialogue.models import UserMessage


@dataclass(frozen=True)
class RelationshipMemory:
    """Immutable profile of user values, preferences, and communication style."""

    user_values: dict[str, float]
    """User's axiom value weights [0, 1]. E.g. PHI: 0.9 (cares about harmony)"""

    user_preferences: dict[str, str]
    """Topic-specific verdict preferences. E.g. financial  GROWL (caution)"""

    user_style: str
    """User decision-making style: analytical, intuitive, careful, exploratory"""

    communication_style: dict[str, str]
    """How user prefers to communicate: verbosity, formality, etc"""

    learning_rate: float
    """How quickly to update from feedback [0.001, 0.1]. Default: 0.01"""

    knowledge_areas: list[str] = field(default_factory=list)
    """User's expertise areas: blockchain, game theory, etc"""

    @property
    def is_frozen(self) -> bool:
        """RelationshipMemory is always frozen (immutable)."""
        return True

    def update_preference(self, topic: str, verdict: str) -> RelationshipMemory:
        """Create new RelationshipMemory with updated preference."""
        new_preferences = {**self.user_preferences, topic: verdict}
        return replace(self, user_preferences=new_preferences)

    def update_value(self, axiom: str, weight: float) -> RelationshipMemory:
        """Create new RelationshipMemory with updated axiom weight."""
        new_values = {**self.user_values, axiom: weight}
        return replace(self, user_values=new_values)

    def update_communication_style(self, key: str, value: str) -> RelationshipMemory:
        """Create new RelationshipMemory with updated communication preference."""
        new_style = {**self.communication_style, key: value}
        return replace(self, communication_style=new_style)

    def infer_communication_style(
        self, recent_messages: list[UserMessage]
    ) -> dict[str, str]:
        """Infer communication preferences from recent messages."""
        if not recent_messages:
            return self.communication_style

        # Analyze message length
        avg_length = sum(len(m.content) for m in recent_messages) / len(recent_messages)

        inferred = dict(self.communication_style)

        # Short messages  concise preference
        if avg_length < 30:
            inferred["verbosity"] = "concise"
        elif avg_length > 100:
            inferred["verbosity"] = "detailed"
        else:
            inferred["verbosity"] = "balanced"

        return inferred

    def with_updated_learning_rate(self, new_rate: float) -> RelationshipMemory:
        """Create new memory with updated learning rate."""
        return replace(self, learning_rate=max(0.001, min(0.1, new_rate)))
