"""Dialogue message models for bidirectional CYNIC-Human conversation."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from types import MappingProxyType


@dataclass(frozen=True)
class UserMessage:
    """Immutable user message to CYNIC."""

    message_type: str  # "question", "feedback", "exploration"
    content: str
    user_confidence: float  # [0, 0.618] Ï-bounded
    related_judgment_id: str | None = None
    timestamp: float = field(default_factory=lambda: datetime.now().timestamp())

    def __post_init__(self):
        """Validate Ï-bounded confidence."""
        if not (0 <= self.user_confidence <= 0.618):
            raise ValueError(
                f"user_confidence must be Ï-bounded [0, 0.618], got {self.user_confidence}"
            )

    @property
    def is_user_message(self) -> bool:
        """Identify this as a user message."""
        return True


@dataclass(frozen=True)
class CynicMessage:
    """Immutable CYNIC response message."""

    message_type: str  # "reasoning", "curiosity", "proposal", "question"
    content: str
    confidence: float  # [0, 0.618] Ï-bounded
    axiom_scores: dict[str, float] = field(default_factory=dict)
    source_judgment_id: str | None = None
    timestamp: float = field(default_factory=lambda: datetime.now().timestamp())

    def __post_init__(self):
        """Validate Ï-bounded confidence and wrap axiom_scores in MappingProxyType."""
        # Validate Ï-bounded confidence
        if not (0 <= self.confidence <= 0.618):
            raise ValueError(f"confidence must be Ï-bounded [0, 0.618], got {self.confidence}")

        # Wrap axiom_scores in MappingProxyType for true immutability
        if self.axiom_scores and not isinstance(self.axiom_scores, MappingProxyType):
            object.__setattr__(self, "axiom_scores", MappingProxyType(self.axiom_scores))

    @property
    def is_user_message(self) -> bool:
        """Identify this as a CYNIC message (not user)."""
        return False


# Type alias for either message type
DialogueMessage = UserMessage | CynicMessage
