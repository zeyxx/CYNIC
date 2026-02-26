"""
Tests for CYNIC unified Q-Learning system.

Tests validate:
- UnifiedQTable initialization with neutral values (0.5)
- Q-value updates from correct and incorrect predictions
- Learning rate effects on update magnitude
- Q-value bounds [0, 1]
- Prediction confidence calculations
- Φ-bounding to 0.618
- LearningSession outcome tracking
- Accuracy and satisfaction calculations
- Multiple outcomes integration
"""
from __future__ import annotations

import pytest

from cynic.core.unified_state import UnifiedLearningOutcome
from cynic.learning.unified_learning import UnifiedQTable, LearningSession


class TestUnifiedQTable:
    """Test UnifiedQTable Q-Learning initialization and updates."""

    def test_q_table_initialization(self):
        """All verdict transitions should initialize to neutral 0.5."""
        q_table = UnifiedQTable()

        # All 16 possible (predicted, actual) pairs should exist
        verdicts = ["HOWL", "WAG", "GROWL", "BARK"]
        assert len(q_table.values) == 16

        # All should be initialized to 0.5 (neutral)
        for pred in verdicts:
            for actual in verdicts:
                key = (pred, actual)
                assert key in q_table.values
                assert q_table.values[key] == 0.5

    def test_q_table_get_q_value(self):
        """get_q_value should return correct Q-value for transition."""
        q_table = UnifiedQTable()

        # Should return 0.5 for uninitialized transitions
        assert q_table.get_q_value("HOWL", "WAG") == 0.5

        # Manually set a value
        q_table.values[("HOWL", "WAG")] = 0.75
        assert q_table.get_q_value("HOWL", "WAG") == 0.75

    def test_q_table_update_correct_prediction(self):
        """Q-value should increase when prediction is correct."""
        q_table = UnifiedQTable()
        q_table.learning_rate = 0.1

        # Create outcome: predicted HOWL, actually HOWL, satisfied (rating=1.0)
        outcome = UnifiedLearningOutcome(
            judgment_id="j1",
            predicted_verdict="HOWL",
            actual_verdict="HOWL",
            satisfaction_rating=1.0,
        )

        initial_q = q_table.get_q_value("HOWL", "HOWL")
        assert initial_q == 0.5

        q_table.update(outcome)

        # Q_new = 0.5 + 0.1 * (1.0 - 0.5) = 0.5 + 0.05 = 0.55
        updated_q = q_table.get_q_value("HOWL", "HOWL")
        assert abs(updated_q - 0.55) < 1e-9

    def test_q_table_update_incorrect_prediction(self):
        """Q-value should decrease when prediction is incorrect."""
        q_table = UnifiedQTable()
        q_table.learning_rate = 0.1

        # Create outcome: predicted HOWL, actually WAG, unsatisfied (rating=0.0)
        outcome = UnifiedLearningOutcome(
            judgment_id="j1",
            predicted_verdict="HOWL",
            actual_verdict="WAG",
            satisfaction_rating=0.0,
        )

        initial_q = q_table.get_q_value("HOWL", "WAG")
        assert initial_q == 0.5

        q_table.update(outcome)

        # Q_new = 0.5 + 0.1 * (0.0 - 0.5) = 0.5 - 0.05 = 0.45
        updated_q = q_table.get_q_value("HOWL", "WAG")
        assert abs(updated_q - 0.45) < 1e-9

    def test_q_table_update_partial_satisfaction(self):
        """Q-value should update based on satisfaction rating."""
        q_table = UnifiedQTable()
        q_table.learning_rate = 0.1

        # Partially satisfied (rating=0.5)
        outcome = UnifiedLearningOutcome(
            judgment_id="j1",
            predicted_verdict="WAG",
            actual_verdict="WAG",
            satisfaction_rating=0.5,
        )

        q_table.update(outcome)

        # Q_new = 0.5 + 0.1 * (0.5 - 0.5) = 0.5 (no change)
        updated_q = q_table.get_q_value("WAG", "WAG")
        assert abs(updated_q - 0.5) < 1e-9

    def test_q_table_learning_rate_effect(self):
        """Higher learning rate should cause bigger Q-value changes."""
        q_table_slow = UnifiedQTable()
        q_table_slow.learning_rate = 0.01

        q_table_fast = UnifiedQTable()
        q_table_fast.learning_rate = 0.5

        outcome = UnifiedLearningOutcome(
            judgment_id="j1",
            predicted_verdict="HOWL",
            actual_verdict="HOWL",
            satisfaction_rating=1.0,
        )

        q_table_slow.update(outcome)
        q_table_fast.update(outcome)

        slow_q = q_table_slow.get_q_value("HOWL", "HOWL")
        fast_q = q_table_fast.get_q_value("HOWL", "HOWL")

        # Fast learning rate should produce larger change
        assert fast_q > slow_q
        assert abs(slow_q - 0.505) < 1e-9  # 0.5 + 0.01 * 0.5
        assert abs(fast_q - 0.75) < 1e-9  # 0.5 + 0.5 * 0.5

    def test_q_table_bounds_upper(self):
        """Q-values should be clamped to 1.0 maximum."""
        q_table = UnifiedQTable()
        q_table.learning_rate = 1.0  # Extreme learning rate

        # Multiple high-satisfaction outcomes
        for i in range(5):
            outcome = UnifiedLearningOutcome(
                judgment_id=f"j{i}",
                predicted_verdict="HOWL",
                actual_verdict="HOWL",
                satisfaction_rating=1.0,
            )
            q_table.update(outcome)

        # Should never exceed 1.0
        updated_q = q_table.get_q_value("HOWL", "HOWL")
        assert updated_q <= 1.0
        assert abs(updated_q - 1.0) < 1e-9

    def test_q_table_bounds_lower(self):
        """Q-values should be clamped to 0.0 minimum."""
        q_table = UnifiedQTable()
        q_table.learning_rate = 1.0

        # Multiple low-satisfaction outcomes
        for i in range(5):
            outcome = UnifiedLearningOutcome(
                judgment_id=f"j{i}",
                predicted_verdict="HOWL",
                actual_verdict="WAG",
                satisfaction_rating=0.0,
            )
            q_table.update(outcome)

        # Should never go below 0.0
        updated_q = q_table.get_q_value("HOWL", "WAG")
        assert updated_q >= 0.0
        assert abs(updated_q - 0.0) < 1e-9

    def test_q_table_reset(self):
        """reset() should return all Q-values to 0.5."""
        q_table = UnifiedQTable()

        # Update several values
        for i in range(3):
            outcome = UnifiedLearningOutcome(
                judgment_id=f"j{i}",
                predicted_verdict="HOWL",
                actual_verdict="HOWL",
                satisfaction_rating=1.0,
            )
            q_table.update(outcome)

        # Verify they changed
        assert q_table.get_q_value("HOWL", "HOWL") > 0.5

        # Reset
        q_table.reset()

        # All should return to 0.5
        verdicts = ["HOWL", "WAG", "GROWL", "BARK"]
        for pred in verdicts:
            for actual in verdicts:
                assert abs(q_table.get_q_value(pred, actual) - 0.5) < 1e-9

    def test_q_table_get_prediction_confidence(self):
        """Confidence should be average of Q-values for a verdict."""
        q_table = UnifiedQTable()

        # Manually set Q-values for HOWL predictions
        q_table.values[("HOWL", "HOWL")] = 0.9
        q_table.values[("HOWL", "WAG")] = 0.6
        q_table.values[("HOWL", "GROWL")] = 0.4
        q_table.values[("HOWL", "BARK")] = 0.2

        # Average = (0.9 + 0.6 + 0.4 + 0.2) / 4 = 0.525
        confidence = q_table.get_prediction_confidence("HOWL")
        assert abs(confidence - 0.525) < 1e-9

    def test_q_table_confidence_phi_bounded(self):
        """Prediction confidence should be φ-bounded to 0.618."""
        q_table = UnifiedQTable()

        # Set all HOWL→* transitions to 1.0
        for verdict in ["HOWL", "WAG", "GROWL", "BARK"]:
            q_table.values[("HOWL", verdict)] = 1.0

        # Average would be 1.0, but should be clamped to PHI_INV ≈ 0.618
        confidence = q_table.get_prediction_confidence("HOWL")
        assert confidence <= 0.618033988749895
        assert abs(confidence - 0.618033988749895) < 1e-9

    def test_q_table_confidence_no_matching_verdicts(self):
        """Confidence should return 0.5 if no predictions found."""
        q_table = UnifiedQTable()

        # Create table with no entries (impossible, but test default)
        # Reset and clear all HOWL predictions somehow
        confidence = q_table.get_prediction_confidence("NONEXISTENT")
        assert confidence == 0.5


class TestLearningSession:
    """Test LearningSession tracking and statistics."""

    def test_learning_session_initialization(self):
        """LearningSession should start with empty outcomes and new Q-Table."""
        session = LearningSession()

        assert len(session.outcomes) == 0
        assert isinstance(session.q_table, UnifiedQTable)
        assert session.q_table.learning_rate == 0.1

    def test_learning_session_add_outcome(self):
        """add_outcome should record outcome and update Q-Table."""
        session = LearningSession()

        outcome = UnifiedLearningOutcome(
            judgment_id="j1",
            predicted_verdict="HOWL",
            actual_verdict="HOWL",
            satisfaction_rating=1.0,
        )

        session.add_outcome(outcome)

        assert len(session.outcomes) == 1
        assert session.outcomes[0] == outcome
        # Q-Table should be updated
        assert session.q_table.get_q_value("HOWL", "HOWL") > 0.5

    def test_learning_session_tracks_outcomes(self):
        """LearningSession should accumulate multiple outcomes."""
        session = LearningSession()

        outcomes = [
            UnifiedLearningOutcome("j1", "HOWL", "HOWL", 1.0),
            UnifiedLearningOutcome("j2", "WAG", "WAG", 1.0),
            UnifiedLearningOutcome("j3", "GROWL", "BARK", 0.2),
        ]

        for outcome in outcomes:
            session.add_outcome(outcome)

        assert len(session.outcomes) == 3
        assert session.outcomes == outcomes

    def test_learning_session_accuracy_all_correct(self):
        """Accuracy should be 1.0 when all predictions match actuals."""
        session = LearningSession()

        outcomes = [
            UnifiedLearningOutcome("j1", "HOWL", "HOWL", 1.0),
            UnifiedLearningOutcome("j2", "WAG", "WAG", 1.0),
            UnifiedLearningOutcome("j3", "GROWL", "GROWL", 1.0),
        ]

        for outcome in outcomes:
            session.add_outcome(outcome)

        assert abs(session.accuracy_rate() - 1.0) < 1e-9

    def test_learning_session_accuracy_all_incorrect(self):
        """Accuracy should be 0.0 when no predictions match actuals."""
        session = LearningSession()

        outcomes = [
            UnifiedLearningOutcome("j1", "HOWL", "WAG", 1.0),
            UnifiedLearningOutcome("j2", "WAG", "GROWL", 1.0),
            UnifiedLearningOutcome("j3", "GROWL", "BARK", 1.0),
        ]

        for outcome in outcomes:
            session.add_outcome(outcome)

        assert abs(session.accuracy_rate() - 0.0) < 1e-9

    def test_learning_session_accuracy_mixed(self):
        """Accuracy should reflect percentage of correct predictions."""
        session = LearningSession()

        outcomes = [
            UnifiedLearningOutcome("j1", "HOWL", "HOWL", 1.0),  # correct
            UnifiedLearningOutcome("j2", "WAG", "WAG", 1.0),    # correct
            UnifiedLearningOutcome("j3", "GROWL", "BARK", 0.2),  # incorrect
            UnifiedLearningOutcome("j4", "BARK", "BARK", 1.0),  # correct
        ]

        for outcome in outcomes:
            session.add_outcome(outcome)

        # 3 correct out of 4 = 0.75
        assert abs(session.accuracy_rate() - 0.75) < 1e-9

    def test_learning_session_accuracy_empty(self):
        """Accuracy should be 0.0 when no outcomes recorded."""
        session = LearningSession()
        assert session.accuracy_rate() == 0.0

    def test_learning_session_satisfaction_average(self):
        """Satisfaction average should compute mean satisfaction rating."""
        session = LearningSession()

        outcomes = [
            UnifiedLearningOutcome("j1", "HOWL", "HOWL", 1.0),
            UnifiedLearningOutcome("j2", "WAG", "WAG", 0.8),
            UnifiedLearningOutcome("j3", "GROWL", "BARK", 0.6),
        ]

        for outcome in outcomes:
            session.add_outcome(outcome)

        # Average = (1.0 + 0.8 + 0.6) / 3 = 0.8
        assert abs(session.satisfaction_average() - 0.8) < 1e-9

    def test_learning_session_satisfaction_empty(self):
        """Satisfaction average should be 0.0 when no outcomes."""
        session = LearningSession()
        assert session.satisfaction_average() == 0.0

    def test_learning_session_get_learning_rate(self):
        """get_learning_rate should return Q-Table learning rate."""
        session = LearningSession()
        assert session.get_learning_rate() == 0.1

    def test_learning_session_set_learning_rate(self):
        """set_learning_rate should update Q-Table learning rate."""
        session = LearningSession()

        session.set_learning_rate(0.25)
        assert session.get_learning_rate() == 0.25
        assert session.q_table.learning_rate == 0.25

    def test_learning_session_set_learning_rate_validation(self):
        """set_learning_rate should reject invalid rates."""
        session = LearningSession()

        with pytest.raises(ValueError):
            session.set_learning_rate(-0.1)

        with pytest.raises(ValueError):
            session.set_learning_rate(1.5)

        # Valid bounds should work
        session.set_learning_rate(0.0)
        assert session.get_learning_rate() == 0.0

        session.set_learning_rate(1.0)
        assert session.get_learning_rate() == 1.0

    def test_learning_session_multiple_outcomes_accumulate(self):
        """Session should track growing Q-Table confidence over many outcomes."""
        session = LearningSession()
        session.set_learning_rate(0.2)

        # All correct HOWL predictions
        for i in range(5):
            outcome = UnifiedLearningOutcome(
                f"j{i}", "HOWL", "HOWL", 1.0
            )
            session.add_outcome(outcome)

        # Q-value for (HOWL, HOWL) should improve
        q_value = session.q_table.get_q_value("HOWL", "HOWL")
        assert q_value > 0.5

        # Accuracy should be 100%
        assert abs(session.accuracy_rate() - 1.0) < 1e-9

        # Satisfaction should be 100%
        assert abs(session.satisfaction_average() - 1.0) < 1e-9

    def test_learning_session_mixed_judgments(self):
        """Session should handle complex mix of correct and incorrect predictions."""
        session = LearningSession()

        outcomes = [
            UnifiedLearningOutcome("j1", "HOWL", "HOWL", 1.0),
            UnifiedLearningOutcome("j2", "HOWL", "WAG", 0.3),
            UnifiedLearningOutcome("j3", "WAG", "WAG", 0.9),
            UnifiedLearningOutcome("j4", "WAG", "GROWL", 0.1),
            UnifiedLearningOutcome("j5", "GROWL", "GROWL", 0.8),
        ]

        for outcome in outcomes:
            session.add_outcome(outcome)

        # Check statistics
        assert len(session.outcomes) == 5
        assert abs(session.accuracy_rate() - 0.6) < 1e-9  # 3 correct out of 5
        avg_satisfaction = (1.0 + 0.3 + 0.9 + 0.1 + 0.8) / 5
        assert abs(session.satisfaction_average() - avg_satisfaction) < 1e-9

        # Q-Table should have multiple updates
        assert session.q_table.get_q_value("HOWL", "HOWL") > 0.5
        assert session.q_table.get_q_value("HOWL", "WAG") < 0.5
        assert session.q_table.get_q_value("WAG", "WAG") > 0.5


class TestUnifiedQTablePHIProperties:
    """Test φ-mathematical properties of Q-Table."""

    def test_phi_constant(self):
        """Verify PHI_INV is correctly set to 0.618033988749895."""
        q_table = UnifiedQTable()
        assert abs(q_table.PHI_INV - 0.618033988749895) < 1e-15

    def test_phi_identity(self):
        """Verify φ × φ⁻¹ = 1."""
        q_table = UnifiedQTable()
        product = q_table.PHI * q_table.PHI_INV
        assert abs(product - 1.0) < 1e-15

    def test_discount_factor_default(self):
        """Discount factor should default to 0.99."""
        q_table = UnifiedQTable()
        assert q_table.discount_factor == 0.99


class TestUnifiedConsciousStateIntegration:
    """Test integration of learning with UnifiedConsciousState."""

    def test_update_from_outcome_records_outcome(self):
        """update_from_outcome should record learning outcome."""
        from cynic.core.unified_state import UnifiedConsciousState

        state = UnifiedConsciousState()
        outcome = UnifiedLearningOutcome("j1", "HOWL", "HOWL", 1.0)

        state.update_from_outcome(outcome)

        assert len(state.get_recent_outcomes(1)) == 1
        assert state.get_recent_outcomes(1)[0] == outcome

    def test_get_learned_confidence_returns_placeholder(self):
        """get_learned_confidence should return confidence value."""
        from cynic.core.unified_state import UnifiedConsciousState

        state = UnifiedConsciousState()

        confidence = state.get_learned_confidence("HOWL")
        assert isinstance(confidence, float)
        assert 0.0 <= confidence <= 1.0

    def test_consciousness_with_learning_session(self):
        """UnifiedConsciousState should work with LearningSession."""
        from cynic.core.unified_state import UnifiedConsciousState

        state = UnifiedConsciousState()
        session = LearningSession()

        # Record outcomes in session and state
        outcomes = [
            UnifiedLearningOutcome("j1", "HOWL", "HOWL", 1.0),
            UnifiedLearningOutcome("j2", "WAG", "WAG", 0.9),
        ]

        for outcome in outcomes:
            session.add_outcome(outcome)
            state.update_from_outcome(outcome)

        # Both should have recorded the outcomes
        assert len(session.outcomes) == 2
        assert len(state.get_recent_outcomes(2)) == 2

        # Session should compute statistics
        assert abs(session.accuracy_rate() - 1.0) < 1e-9
        assert abs(session.satisfaction_average() - 0.95) < 1e-9
