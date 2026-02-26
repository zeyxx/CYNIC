"""Dialogue message models for bidirectional CYNIC-Human conversation."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional
from datetime import datetime


@dataclass(frozen=True)
class UserMessage:
    """Immutable user message to CYNIC."""

    message_type: str  # "question", "feedback", "exploration"
    content: str
    user_confidence: float  # [0, 1]
    related_judgment_id: Optional[str] = None
    timestamp: float = field(default_factory=lambda: datetime.now().timestamp())

    @property
    def is_user_message(self) -> bool:
        """Identify this as a user message."""
        return True


@dataclass(frozen=True)
class CynicMessage:
    """Immutable CYNIC response message."""

    message_type: str  # "reasoning", "curiosity", "proposal", "question"
    content: str
    confidence: float  # [0, 0.618] φ-bounded
    axiom_scores: dict[str, float] = field(default_factory=dict)
    source_judgment_id: Optional[str] = None
    timestamp: float = field(default_factory=lambda: datetime.now().timestamp())

    @property
    def is_user_message(self) -> bool:
        """Identify this as a CYNIC message (not user)."""
        return False


# Type alias for either message type
DialogueMessage = UserMessage | CynicMessage
