import pytest

from cynic.kernel.organism.brain.dialogue.models import UserMessage
from cynic.kernel.organism.brain.learning.relationship_memory import RelationshipMemory


def test_relationship_memory_creation():
    """Create relationship memory with user profile."""
    memory = RelationshipMemory(
        user_values={"PHI": 0.9, "BURN": 0.6},
        user_preferences={"financial": "GROWL", "governance": "WAG"},
        user_style="analytical",
        communication_style={"verbosity": "concise", "formality": "casual"},
        learning_rate=0.01
    )

    assert memory.user_values["PHI"] == 0.9
    assert memory.user_preferences["financial"] == "GROWL"
    assert memory.user_style == "analytical"
    assert memory.learning_rate == 0.01


def test_relationship_memory_immutability():
    """Relationship memory is frozen (immutable)."""
    memory = RelationshipMemory(
        user_values={},
        user_preferences={},
        user_style="analytical",
        communication_style={},
        learning_rate=0.01
    )

    with pytest.raises((AttributeError, TypeError)):
        memory.user_style = "intuitive"


def test_update_from_feedback():
    """Update memory based on user feedback."""
    initial = RelationshipMemory(
        user_values={"PHI": 0.5, "BURN": 0.5},
        user_preferences={},
        user_style="balanced",
        communication_style={"verbosity": "balanced"},
        learning_rate=0.1
    )

    # Simulate feedback: user prefers more caution on financial
    updated = initial.update_preference("financial", "GROWL")

    assert updated.user_preferences.get("financial") == "GROWL"
    assert updated.learning_rate == initial.learning_rate


def test_infer_communication_style():
    """Infer user communication style from interaction patterns."""
    memory = RelationshipMemory(
        user_values={},
        user_preferences={},
        user_style="analytical",
        communication_style={},
        learning_rate=0.01
    )

    # Short messages suggest conciseness
    short_messages = [UserMessage("q", "Why WAG?", 0.5, None) for _ in range(3)]

    inferred_style = memory.infer_communication_style(short_messages)
    assert "verbosity" in inferred_style or len(inferred_style) > 0
