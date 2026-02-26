import pytest
from dataclasses import dataclass
from cynic.dialogue.models import UserMessage, CynicMessage, DialogueMessage


def test_user_message_creation():
    """User message captures all required fields."""
    msg = UserMessage(
        message_type="question",
        content="Why did you choose WAG?",
        user_confidence=0.9,
        related_judgment_id=None
    )
    assert msg.message_type == "question"
    assert msg.content == "Why did you choose WAG?"
    assert msg.user_confidence == 0.9
    assert msg.is_user_message is True


def test_cynic_message_creation():
    """CYNIC message includes reasoning and metadata."""
    msg = CynicMessage(
        message_type="reasoning",
        content="Dog 5 voted HOWL, Dog 7 voted WAG...",
        confidence=0.73,
        axiom_scores={"PHI": 0.8, "BURN": 0.6},
        source_judgment_id="j123"
    )
    assert msg.message_type == "reasoning"
    assert msg.confidence == 0.73
    assert msg.axiom_scores == {"PHI": 0.8, "BURN": 0.6}
    assert msg.is_user_message is False


def test_message_immutability():
    """Messages are immutable frozen dataclasses."""
    msg = UserMessage("feedback", "Try GROWL next time", 0.8, None)
    with pytest.raises((AttributeError, TypeError)):
        msg.content = "Modified"
