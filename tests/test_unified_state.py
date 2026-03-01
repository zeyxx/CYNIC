"""
Tests for unified state models (UnifiedJudgment, UnifiedLearningOutcome, UnifiedConsciousState)

TDD approach:
1. Test immutability of frozen dataclasses
2. Test buffer management and auto-pruning
3. Test consensus score calculation
4. Test learning outcome recording
"""
from __future__ import annotations

import pytest
from dataclasses import FrozenInstanceError
from typing import Dict, Any

from cynic.kernel.core.unified_state import (
    UnifiedJudgment,
    UnifiedLearningOutcome,
    JudgmentBuffer,
    OutcomeBuffer,
    UnifiedConsciousState,
)


class TestUnifiedJudgmentImmutability:
    """Test that UnifiedJudgment is properly frozen."""

    def test_unified_judgment_immutable(self):
        """Verify UnifiedJudgment is truly immutable (frozen=True)."""
        judgment = UnifiedJudgment(
            judgment_id="j123",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={"FIDELITY": 0.9, "PHI": 0.85},
            dog_votes={1: {"vote": "HOWL", "q_score": 85.0}},
            reasoning="Good code quality",
            latency_ms=150.0,
        )

        # Attempt to modify should raise FrozenInstanceError
        with pytest.raises(FrozenInstanceError):
            judgment.verdict = "BARK"

        with pytest.raises(FrozenInstanceError):
            judgment.q_score = 50.0

        with pytest.raises(FrozenInstanceError):
            judgment.confidence = 0.5


class TestJudgmentBufferManagement:
    """Test JudgmentBuffer add, retrieve, and pruning."""

    def test_conscious_state_buffer_management(self):
        """Test adding 10 judgments to buffer and retrieving them."""
        buffer = JudgmentBuffer()

        # Add 10 judgments
        for i in range(10):
            judgment = UnifiedJudgment(
                judgment_id=f"j{i}",
                verdict="HOWL" if i % 2 == 0 else "WAG",
                q_score=80.0 + i,
                confidence=0.618,
                axiom_scores={"FIDELITY": 0.9},
                dog_votes={},
                reasoning=f"Judgment {i}",
                latency_ms=100.0 + i,
            )
            buffer.add(judgment)

        # Should have 10 items
        assert len(buffer.buffer) == 10

        # Retrieve all
        recent = buffer.get_recent(10)
        assert len(recent) == 10
        assert recent[0].judgment_id == "j0"
        assert recent[9].judgment_id == "j9"

        # Retrieve subset
        recent_5 = buffer.get_recent(5)
        assert len(recent_5) == 5
        assert recent_5[0].judgment_id == "j5"
        assert recent_5[4].judgment_id == "j9"


class TestLearningOutcomeImmutability:
    """Test that UnifiedLearningOutcome is properly frozen."""

    def test_learning_outcome_immutable(self):
        """Verify UnifiedLearningOutcome is truly immutable."""
        outcome = UnifiedLearningOutcome(
            judgment_id="j123",
            predicted_verdict="HOWL",
            actual_verdict="HOWL",
            satisfaction_rating=0.9,
        )

        # Attempt to modify should raise FrozenInstanceError
        with pytest.raises(FrozenInstanceError):
            outcome.predicted_verdict = "BARK"

        with pytest.raises(FrozenInstanceError):
            outcome.satisfaction_rating = 0.5


class TestBufferAutoPruning:
    """Test that buffers auto-prune oldest items when full."""

    def test_buffer_auto_pruning(self):
        """Add 100 items to 89-capacity buffer and verify oldest are pruned."""
        buffer = JudgmentBuffer()  # maxlen=89

        # Add 100 judgments
        for i in range(100):
            judgment = UnifiedJudgment(
                judgment_id=f"j{i}",
                verdict="HOWL",
                q_score=85.0,
                confidence=0.618,
                axiom_scores={"FIDELITY": 0.9},
                dog_votes={},
                reasoning=f"Judgment {i}",
                latency_ms=100.0,
            )
            buffer.add(judgment)

        # Should only have 89 items (oldest 11 should be pruned)
        assert len(buffer.buffer) == 89

        # First item should be j11 (j0 through j10 pruned)
        recent = buffer.get_recent(89)
        assert recent[0].judgment_id == "j11"
        assert recent[88].judgment_id == "j99"

        # Verify maxlen is Fibonacci(11)=89
        assert buffer.buffer.maxlen == 89


class TestOutcomeBufferAutoPruning:
    """Test OutcomeBuffer auto-pruning at maxlen=55 (Fibonacci(10))."""

    def test_outcome_buffer_auto_pruning(self):
        """Add 100 outcomes to 55-capacity buffer and verify oldest are pruned."""
        buffer = OutcomeBuffer()  # maxlen=55

        # Add 100 outcomes
        for i in range(100):
            outcome = UnifiedLearningOutcome(
                judgment_id=f"j{i}",
                predicted_verdict="HOWL",
                actual_verdict="WAG",
                satisfaction_rating=0.7,
            )
            buffer.add(outcome)

        # Should only have 55 items (oldest 45 should be pruned)
        assert len(buffer.buffer) == 55

        # First item should be j45 (j0 through j44 pruned)
        recent = buffer.get_recent(55)
        assert recent[0].judgment_id == "j45"
        assert recent[54].judgment_id == "j99"

        # Verify maxlen is Fibonacci(10)=55
        assert buffer.buffer.maxlen == 55


class TestConsensusScoreCalculation:
    """Test consensus score computation from dog agreement."""

    def test_consensus_score_calculation(self):
        """Test get_consensus_score() calculation."""
        state = UnifiedConsciousState()

        # Initial consensus should be 0.0 (no dogs)
        assert state.get_consensus_score() == 0.0

        # Add dog agreement scores
        state.dog_agreement_scores = {
            1: 0.9,
            2: 0.95,
            3: 0.88,
            4: 0.92,
            5: 0.91,
            6: 0.89,
            7: 0.94,
            8: 0.87,
            9: 0.90,
            10: 0.93,
            11: 0.85,
        }

        # Consensus should be average
        expected = sum(state.dog_agreement_scores.values()) / len(state.dog_agreement_scores)
        actual = state.get_consensus_score()
        assert abs(actual - expected) < 0.0001


class TestUnifiedConsciousStateIntegration:
    """Integration tests for UnifiedConsciousState."""

    def test_add_judgment_updates_state(self):
        """Test that add_judgment() updates total_judgments counter."""
        state = UnifiedConsciousState()
        initial_count = state.total_judgments

        judgment = UnifiedJudgment(
            judgment_id="j1",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={"FIDELITY": 0.9},
            dog_votes={},
            reasoning="Test",
            latency_ms=100.0,
        )

        state.add_judgment(judgment)
        assert state.total_judgments == initial_count + 1
        assert len(state.recent_judgments.buffer) == 1

    def test_add_outcome_updates_state(self):
        """Test that add_outcome() records learning outcome."""
        state = UnifiedConsciousState()
        initial_count = len(state.learning_outcomes.buffer)

        outcome = UnifiedLearningOutcome(
            judgment_id="j1",
            predicted_verdict="HOWL",
            actual_verdict="HOWL",
            satisfaction_rating=0.9,
        )

        state.add_outcome(outcome)
        assert len(state.learning_outcomes.buffer) == initial_count + 1

    def test_unified_conscious_state_multiple_operations(self):
        """Test multiple judgment and outcome additions."""
        state = UnifiedConsciousState()

        # Add 5 judgments
        for i in range(5):
            judgment = UnifiedJudgment(
                judgment_id=f"j{i}",
                verdict="HOWL" if i % 2 == 0 else "WAG",
                q_score=80.0 + i,
                confidence=0.618,
                axiom_scores={"FIDELITY": 0.9},
                dog_votes={i: {"vote": "HOWL", "q_score": 85.0}},
                reasoning=f"Judgment {i}",
                latency_ms=100.0,
            )
            state.add_judgment(judgment)

        # Add 3 outcomes
        for i in range(3):
            outcome = UnifiedLearningOutcome(
                judgment_id=f"j{i}",
                predicted_verdict="HOWL",
                actual_verdict="WAG" if i % 2 == 0 else "HOWL",
                satisfaction_rating=0.8 + i * 0.05,
            )
            state.add_outcome(outcome)

        # Verify state
        assert state.total_judgments == 5
        assert len(state.recent_judgments.buffer) == 5
        assert len(state.learning_outcomes.buffer) == 3


class TestDataclassTypes:
    """Test that all classes are properly typed."""

    def test_unified_judgment_fields(self):
        """Verify UnifiedJudgment has all required fields with correct types."""
        judgment = UnifiedJudgment(
            judgment_id="j123",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={"FIDELITY": 0.9, "PHI": 0.85, "VERIFY": 0.88},
            dog_votes={
                1: {"vote": "HOWL", "q_score": 85.0},
                2: {"vote": "HOWL", "q_score": 84.0},
            },
            reasoning="Excellent quality",
            latency_ms=150.5,
        )

        # Verify types
        assert isinstance(judgment.judgment_id, str)
        assert isinstance(judgment.verdict, str)
        assert isinstance(judgment.q_score, float)
        assert isinstance(judgment.confidence, float)
        # axiom_scores and dog_votes are wrapped in MappingProxyType (read-only views)
        # They behave like dicts but are immutable at the top level
        assert hasattr(judgment.axiom_scores, '__getitem__')  # Dict-like interface
        assert hasattr(judgment.dog_votes, '__getitem__')     # Dict-like interface
        assert isinstance(judgment.reasoning, str)
        assert isinstance(judgment.latency_ms, float)

    def test_unified_learning_outcome_fields(self):
        """Verify UnifiedLearningOutcome has all required fields."""
        outcome = UnifiedLearningOutcome(
            judgment_id="j123",
            predicted_verdict="HOWL",
            actual_verdict="HOWL",
            satisfaction_rating=0.92,
        )

        assert isinstance(outcome.judgment_id, str)
        assert isinstance(outcome.predicted_verdict, str)
        assert isinstance(outcome.actual_verdict, str)
        assert isinstance(outcome.satisfaction_rating, float)


class TestPhiBoundedConfidence:
    """Test that confidence respects φ-bound (max 0.618)."""

    def test_confidence_bounded(self):
        """Create judgments with valid confidence values."""
        # Valid: 0.618 (PHI_INV)
        judgment = UnifiedJudgment(
            judgment_id="j1",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={},
            dog_votes={},
            reasoning="Test",
            latency_ms=100.0,
        )
        assert judgment.confidence == 0.618

        # Valid: 0.0
        judgment2 = UnifiedJudgment(
            judgment_id="j2",
            verdict="WAG",
            q_score=65.0,
            confidence=0.0,
            axiom_scores={},
            dog_votes={},
            reasoning="Test",
            latency_ms=100.0,
        )
        assert judgment2.confidence == 0.0


class TestUnifiedJudgmentDictImmutability:
    """Test that axiom_scores and dog_votes are truly immutable."""

    def test_unified_judgment_axiom_scores_immutable(self):
        """axiom_scores should be immutable via MappingProxyType."""
        judgment = UnifiedJudgment(
            judgment_id="j123",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={"FIDELITY": 0.9, "PHI": 0.85},
            dog_votes={},
            reasoning="Test immutability",
            latency_ms=100.0,
        )

        # Attempt to add a new key should raise TypeError
        with pytest.raises(TypeError):
            judgment.axiom_scores["NEW_KEY"] = 0.5

        # Attempt to modify existing key should raise TypeError
        with pytest.raises(TypeError):
            judgment.axiom_scores["FIDELITY"] = 0.5

        # Original values should still be accessible
        assert judgment.axiom_scores["FIDELITY"] == 0.9
        assert judgment.axiom_scores["PHI"] == 0.85

    def test_unified_judgment_dog_votes_immutable(self):
        """dog_votes top-level structure should be immutable via MappingProxyType."""
        judgment = UnifiedJudgment(
            judgment_id="j123",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={"FIDELITY": 0.9},
            dog_votes={
                1: {"vote": "HOWL", "q_score": 85.0},
                2: {"vote": "HOWL", "q_score": 84.0},
            },
            reasoning="Test dog votes immutability",
            latency_ms=100.0,
        )

        # Attempt to add a new dog vote should raise TypeError
        # (cannot mutate the MappingProxyType at the top level)
        with pytest.raises(TypeError):
            judgment.dog_votes[3] = {"vote": "WAG", "q_score": 75.0}

        # Attempt to delete a dog vote should raise TypeError
        with pytest.raises(TypeError):
            del judgment.dog_votes[1]

        # Original values should still be accessible
        assert judgment.dog_votes[1]["vote"] == "HOWL"
        assert judgment.dog_votes[1]["q_score"] == 85.0
        assert judgment.dog_votes[2]["vote"] == "HOWL"


class TestDogAgreementScoresValidation:
    """Test that dog_agreement_scores are validated for bounds [0, 1]."""

    def test_dog_agreement_scores_valid_bounds(self):
        """Valid bounds should create state successfully."""
        # All valid values
        state = UnifiedConsciousState(
            dog_agreement_scores={
                1: 0.0,
                2: 0.5,
                3: 1.0,
                4: 0.618,
            }
        )

        assert state.dog_agreement_scores[1] == 0.0
        assert state.dog_agreement_scores[2] == 0.5
        assert state.dog_agreement_scores[3] == 1.0
        assert state.dog_agreement_scores[4] == 0.618

    def test_dog_agreement_scores_too_high(self):
        """Score > 1.0 should raise ValueError."""
        with pytest.raises(ValueError, match="must be in"):
            UnifiedConsciousState(dog_agreement_scores={1: 1.5})

    def test_dog_agreement_scores_too_low(self):
        """Score < 0.0 should raise ValueError."""
        with pytest.raises(ValueError, match="must be in"):
            UnifiedConsciousState(dog_agreement_scores={1: -0.1})

    def test_dog_agreement_scores_multiple_invalid(self):
        """Multiple valid + one invalid should raise on first invalid dog."""
        with pytest.raises(ValueError, match="Dog 2 agreement score"):
            UnifiedConsciousState(
                dog_agreement_scores={
                    1: 0.5,
                    2: 1.5,  # Invalid
                    3: 0.7,
                }
            )

    def test_dog_agreement_scores_boundary_values(self):
        """Test exact boundary values (0.0 and 1.0)."""
        # Lower bound: 0.0
        state1 = UnifiedConsciousState(dog_agreement_scores={1: 0.0})
        assert state1.dog_agreement_scores[1] == 0.0

        # Upper bound: 1.0
        state2 = UnifiedConsciousState(dog_agreement_scores={11: 1.0})
        assert state2.dog_agreement_scores[11] == 1.0

        # Just above upper bound
        with pytest.raises(ValueError):
            UnifiedConsciousState(dog_agreement_scores={1: 1.0001})

        # Just below lower bound
        with pytest.raises(ValueError):
            UnifiedConsciousState(dog_agreement_scores={1: -0.0001})
