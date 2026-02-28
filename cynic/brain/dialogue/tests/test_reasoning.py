import pytest
from cynic.brain.dialogue.reasoning import ReasoningEngine
from unittest.mock import MagicMock


@pytest.fixture
def mock_conscious_state():
    """Mock CYNIC's conscious state."""
    state = MagicMock()
    state.last_judgment = {
        "verdict": "WAG",
        "q_score": 78,
        "confidence": 0.5,
        "axiom_scores": {
            "PHI": 0.85,
            "BURN": 0.65,
            "FIDELITY": 0.90,
            "VERIFY": 0.70,
            "CULTURE": 0.75
        },
        "dog_votes": {
            "dog_1": "WAG",
            "dog_5": "HOWL",
            "dog_11": "WAG"
        },
        "consensus_algorithm": "PBFT"
    }
    return state


def test_format_judgment_reasoning(mock_conscious_state):
    """Format judgment into explanation."""
    engine = ReasoningEngine()
    reasoning = engine.format_judgment_reasoning(mock_conscious_state.last_judgment)

    assert "WAG" in reasoning
    assert "78" in reasoning
    assert "PHI" in reasoning


def test_extract_axiom_explanations():
    """Extract which axioms influenced decision."""
    engine = ReasoningEngine()
    axiom_scores = {
        "PHI": 0.85,
        "BURN": 0.65,
        "FIDELITY": 0.90
    }

    explanations = engine.extract_axiom_explanations(axiom_scores)
    assert len(explanations) == 3
    assert "FIDELITY" in explanations
    assert any("harmony" in e.lower() or "elegance" in e.lower()
              for e in explanations.values())


def test_create_context_for_claude(mock_conscious_state):
    """Create structured context for Claude API."""
    engine = ReasoningEngine()
    context = engine.create_context_for_claude(
        question="Why did you choose WAG?",
        judgment=mock_conscious_state.last_judgment,
        user_communication_style="concise"
    )

    assert "question" in context
    assert "verdict" in context
    assert "axiom_scores" in context
    assert context["communication_style"] == "concise"
