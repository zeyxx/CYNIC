import pytest
from types import MappingProxyType
from cynic.brain.dialogue.models import UserMessage, CynicMessage, DialogueMessage


def test_user_message_creation():
    """User message captures all required fields."""
    msg = UserMessage(
        message_type="question",
        content="Why did you choose WAG?",
        user_confidence=0.5,
        related_judgment_id=None
    )
    assert msg.message_type == "question"
    assert msg.content == "Why did you choose WAG?"
    assert msg.user_confidence == 0.5
    assert msg.is_user_message is True


def test_cynic_message_creation():
    """CYNIC message includes reasoning and metadata."""
    msg = CynicMessage(
        message_type="reasoning",
        content="Dog 5 voted HOWL, Dog 7 voted WAG...",
        confidence=0.618,
        axiom_scores={"PHI": 0.8, "BURN": 0.6},
        source_judgment_id="j123"
    )
    assert msg.message_type == "reasoning"
    assert msg.confidence == 0.618
    assert msg.axiom_scores == {"PHI": 0.8, "BURN": 0.6}
    assert msg.is_user_message is False


def test_message_immutability():
    """Messages are immutable frozen dataclasses."""
    msg = UserMessage(
        message_type="feedback",
        content="Try GROWL next time",
        user_confidence=0.5,
        related_judgment_id=None
    )
    with pytest.raises((AttributeError, TypeError)):
        msg.content = "Modified"


def test_user_message_φ_bounded_confidence_valid():
    """user_confidence must be φ-bounded [0, 0.618] - valid values."""
    # At lower boundary
    msg1 = UserMessage(message_type="question", content="Why?", user_confidence=0.0)
    assert msg1.user_confidence == 0.0

    # At upper boundary
    msg2 = UserMessage(message_type="question", content="Why?", user_confidence=0.618)
    assert msg2.user_confidence == 0.618

    # In the middle
    msg3 = UserMessage(message_type="question", content="Why?", user_confidence=0.4)
    assert msg3.user_confidence == 0.4


def test_user_message_φ_bounded_confidence_invalid():
    """user_confidence must be φ-bounded [0, 0.618] - invalid values."""
    # Exceeds φ-bound (0.7 > 0.618)
    with pytest.raises(ValueError, match="user_confidence must be φ-bounded"):
        UserMessage(message_type="question", content="Why?", user_confidence=0.7)

    # Negative
    with pytest.raises(ValueError, match="user_confidence must be φ-bounded"):
        UserMessage(message_type="question", content="Why?", user_confidence=-0.1)


def test_cynic_message_φ_bounded_confidence_valid():
    """confidence must be φ-bounded [0, 0.618] - valid values."""
    # At lower boundary
    msg1 = CynicMessage(message_type="reasoning", content="Because...", confidence=0.0)
    assert msg1.confidence == 0.0

    # At upper boundary
    msg2 = CynicMessage(message_type="reasoning", content="Because...", confidence=0.618)
    assert msg2.confidence == 0.618

    # In the middle
    msg3 = CynicMessage(message_type="reasoning", content="Because...", confidence=0.4)
    assert msg3.confidence == 0.4


def test_cynic_message_φ_bounded_confidence_invalid():
    """confidence must be φ-bounded [0, 0.618] - invalid values."""
    # Exceeds φ-bound (0.73 > 0.618)
    with pytest.raises(ValueError, match="confidence must be φ-bounded"):
        CynicMessage(message_type="reasoning", content="Because...", confidence=0.73)

    # Negative
    with pytest.raises(ValueError, match="confidence must be φ-bounded"):
        CynicMessage(message_type="reasoning", content="Because...", confidence=-0.1)


def test_cynic_message_axiom_scores_immutable():
    """axiom_scores should be MappingProxyType (deep immutable)."""
    msg = CynicMessage(
        message_type="reasoning",
        content="Because...",
        confidence=0.5,
        axiom_scores={"PHI": 0.8, "BURN": 0.6}
    )

    # axiom_scores should be MappingProxyType
    assert isinstance(msg.axiom_scores, MappingProxyType)

    # Should not be able to modify
    with pytest.raises(TypeError):
        msg.axiom_scores["PHI"] = 0.9

    # Should be able to read
    assert msg.axiom_scores["PHI"] == 0.8
    assert msg.axiom_scores["BURN"] == 0.6


def test_cynic_message_empty_axiom_scores():
    """Empty axiom_scores dict should work correctly."""
    msg = CynicMessage(
        message_type="reasoning",
        content="Because...",
        confidence=0.5,
        axiom_scores={}
    )

    # Empty dict is acceptable
    assert msg.axiom_scores == {}
    assert len(msg.axiom_scores) == 0
