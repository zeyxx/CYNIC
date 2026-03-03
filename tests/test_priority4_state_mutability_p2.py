"""Test Priority 4 Phase 2: Buffer Immutability

Verifies that all 6 buffer classes use immutable add() methods that return
new buffer instances instead of mutating in-place.
"""

from __future__ import annotations

import pytest

from cynic.kernel.core.unified_state import (
    CommunityBuffer,
    GovernanceCommunity,
    GovernanceProposal,
    ImpactBuffer,
    ImpactMeasurement,
    JudgmentBuffer,
    OutcomeBuffer,
    ProposalBuffer,
    UnifiedJudgment,
    UnifiedLearningOutcome,
    ValueBuffer,
    ValueCreation,
)
from cynic.kernel.core.phi import fibonacci


class TestJudgmentBufferImmutability:
    """Test JudgmentBuffer uses immutable add()."""

    def test_judgment_buffer_add_returns_new_instance(self):
        """Verify add() returns new buffer, original unchanged."""
        original = JudgmentBuffer()
        judgment = UnifiedJudgment(
            judgment_id="j1",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={},
            dog_votes={},
        )

        updated = original.add(judgment)

        # Original unchanged
        assert len(original.buffer) == 0
        # New instance has item
        assert len(updated.buffer) == 1
        # Different instances
        assert original is not updated

    def test_judgment_buffer_add_returns_judgment_buffer_type(self):
        """Verify add() returns correct type."""
        buffer = JudgmentBuffer()
        judgment = UnifiedJudgment(
            judgment_id="j1",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={},
            dog_votes={},
        )

        result = buffer.add(judgment)
        assert isinstance(result, JudgmentBuffer)

    def test_judgment_buffer_auto_trims_to_max_len(self):
        """Verify add() auto-trims when exceeding max_len."""
        buffer = JudgmentBuffer()
        max_len = buffer.max_len

        # Add max_len + 1 items
        for i in range(max_len + 1):
            judgment = UnifiedJudgment(
                judgment_id=f"j{i}",
                verdict="HOWL",
                q_score=85.0,
                confidence=0.618,
                axiom_scores={},
                dog_votes={},
            )
            buffer = buffer.add(judgment)

        # Should trim to max_len
        assert len(buffer.buffer) == max_len
        # First item should be j1 (j0 pruned)
        assert buffer.buffer[0].judgment_id == "j1"

    def test_judgment_buffer_max_len_is_fibonacci_11(self):
        """Verify max_len is fibonacci(11) = 89."""
        buffer = JudgmentBuffer()
        assert buffer.max_len == fibonacci(11)
        assert buffer.max_len == 89


class TestOutcomeBufferImmutability:
    """Test OutcomeBuffer uses immutable add()."""

    def test_outcome_buffer_add_returns_new_instance(self):
        """Verify add() returns new buffer, original unchanged."""
        original = OutcomeBuffer()
        outcome = UnifiedLearningOutcome(
            judgment_id="j1",
            predicted_verdict="HOWL",
            actual_verdict="WAG",
            satisfaction_rating=0.8,
        )

        updated = original.add(outcome)

        assert len(original.buffer) == 0
        assert len(updated.buffer) == 1
        assert original is not updated

    def test_outcome_buffer_max_len_is_fibonacci_10(self):
        """Verify max_len is fibonacci(10) = 55."""
        buffer = OutcomeBuffer()
        assert buffer.max_len == fibonacci(10)
        assert buffer.max_len == 55


class TestValueBufferImmutability:
    """Test ValueBuffer uses immutable add()."""

    def test_value_buffer_add_returns_new_instance(self):
        """Verify add() returns new buffer, original unchanged."""
        original = ValueBuffer()
        value = ValueCreation(
            creation_id="vc1",
            creator_id="creator1",
            creation_type="code",
            description="Test value",
            direct_impact=0.5,
        )

        updated = original.add(value)

        assert len(original.buffer) == 0
        assert len(updated.buffer) == 1
        assert original is not updated

    def test_value_buffer_max_len_is_fibonacci_12(self):
        """Verify max_len is fibonacci(12) = 144."""
        buffer = ValueBuffer()
        assert buffer.max_len == fibonacci(12)
        assert buffer.max_len == 144


class TestImpactBufferImmutability:
    """Test ImpactBuffer uses immutable add()."""

    def test_impact_buffer_add_returns_new_instance(self):
        """Verify add() returns new buffer, original unchanged."""
        original = ImpactBuffer()
        impact = ImpactMeasurement(
            human_id="h1",
            total_impact=0.7,
            dimension_scores={"code": 0.8},
        )

        updated = original.add(impact)

        assert len(original.buffer) == 0
        assert len(updated.buffer) == 1
        assert original is not updated

    def test_impact_buffer_max_len_is_fibonacci_10(self):
        """Verify max_len is fibonacci(10) = 55."""
        buffer = ImpactBuffer()
        assert buffer.max_len == fibonacci(10)
        assert buffer.max_len == 55


class TestCommunityBufferImmutability:
    """Test CommunityBuffer uses immutable add()."""

    def test_community_buffer_add_returns_new_instance(self):
        """Verify add() returns new buffer, original unchanged."""
        original = CommunityBuffer()
        community = GovernanceCommunity(
            community_id="comm1",
            name="Test Community",
            platform="discord",
        )

        updated = original.add(community)

        assert len(original.buffer) == 0
        assert len(updated.buffer) == 1
        assert original is not updated

    def test_community_buffer_get_method(self):
        """Verify get() retrieves community by ID."""
        buffer = CommunityBuffer()
        community = GovernanceCommunity(
            community_id="comm1",
            name="Test Community",
            platform="discord",
        )

        buffer = buffer.add(community)
        retrieved = buffer.get("comm1")

        assert retrieved is not None
        assert retrieved.community_id == "comm1"
        assert retrieved.name == "Test Community"

    def test_community_buffer_get_missing_returns_none(self):
        """Verify get() returns None for missing community."""
        buffer = CommunityBuffer()
        assert buffer.get("missing") is None

    def test_community_buffer_all_communities(self):
        """Verify all_communities() returns all communities."""
        buffer = CommunityBuffer()

        communities = [
            GovernanceCommunity(
                community_id=f"comm{i}",
                name=f"Community {i}",
                platform="discord",
            )
            for i in range(3)
        ]

        for comm in communities:
            buffer = buffer.add(comm)

        all_comms = buffer.all_communities()
        assert len(all_comms) == 3
        assert all(isinstance(c, GovernanceCommunity) for c in all_comms)


class TestProposalBufferImmutability:
    """Test ProposalBuffer uses immutable add()."""

    def test_proposal_buffer_add_returns_new_instance(self):
        """Verify add() returns new buffer, original unchanged."""
        original = ProposalBuffer()
        proposal = GovernanceProposal(
            proposal_id="prop1",
            community_id="comm1",
            proposer_id="proposer1",
            title="Test Proposal",
            description="Test",
            category="feature",
        )

        updated = original.add(proposal)

        assert len(original.buffer) == 0
        assert len(updated.buffer) == 1
        assert original is not updated

    def test_proposal_buffer_max_len_is_fibonacci_11(self):
        """Verify max_len is fibonacci(11) = 89."""
        buffer = ProposalBuffer()
        assert buffer.max_len == fibonacci(11)
        assert buffer.max_len == 89


class TestBufferChaining:
    """Test chaining immutable buffer operations."""

    def test_chain_multiple_adds_judgment_buffer(self):
        """Test chaining add() calls on JudgmentBuffer."""
        buffer = JudgmentBuffer()

        j1 = UnifiedJudgment(
            judgment_id="j1",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={},
            dog_votes={},
        )
        j2 = UnifiedJudgment(
            judgment_id="j2",
            verdict="WAG",
            q_score=75.0,
            confidence=0.5,
            axiom_scores={},
            dog_votes={},
        )
        j3 = UnifiedJudgment(
            judgment_id="j3",
            verdict="GROWL",
            q_score=65.0,
            confidence=0.4,
            axiom_scores={},
            dog_votes={},
        )

        # Chain adds
        buffer = buffer.add(j1).add(j2).add(j3)

        assert len(buffer.buffer) == 3
        assert buffer.buffer[0].judgment_id == "j1"
        assert buffer.buffer[2].judgment_id == "j3"

    def test_chain_multiple_adds_community_buffer(self):
        """Test chaining add() calls on CommunityBuffer."""
        buffer = CommunityBuffer()

        communities = [
            GovernanceCommunity(
                community_id=f"comm{i}",
                name=f"Community {i}",
                platform="discord",
            )
            for i in range(3)
        ]

        # Chain adds
        for comm in communities:
            buffer = buffer.add(comm)

        assert len(buffer.buffer) == 3
        for i, comm in enumerate(communities):
            assert buffer.get(f"comm{i}").name == f"Community {i}"


class TestBufferTupleImmutability:
    """Test that tuple buffers are truly immutable."""

    def test_judgment_buffer_tuple_is_immutable(self):
        """Verify tuple buffer cannot be mutated directly."""
        buffer = JudgmentBuffer()
        judgment = UnifiedJudgment(
            judgment_id="j1",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={},
            dog_votes={},
        )
        buffer = buffer.add(judgment)

        # Attempt to mutate tuple should raise error
        with pytest.raises(TypeError):
            buffer.buffer[0] = None  # type: ignore

    def test_outcome_buffer_tuple_is_immutable(self):
        """Verify tuple buffer cannot be mutated directly."""
        buffer = OutcomeBuffer()
        outcome = UnifiedLearningOutcome(
            judgment_id="j1",
            predicted_verdict="HOWL",
            actual_verdict="WAG",
            satisfaction_rating=0.8,
        )
        buffer = buffer.add(outcome)

        # Tuples don't have append method, so this raises AttributeError
        with pytest.raises(AttributeError):
            buffer.buffer.append(outcome)  # type: ignore


class TestBufferIntegrationWithConsciousState:
    """Test buffers work correctly with UnifiedConsciousState."""

    def test_conscious_state_add_judgment_returns_new_buffer(self):
        """Verify UnifiedConsciousState.add_judgment() updates buffer reference."""
        from cynic.kernel.core.unified_state import UnifiedConsciousState

        state = UnifiedConsciousState()
        original_buffer = state.recent_judgments

        judgment = UnifiedJudgment(
            judgment_id="j1",
            verdict="HOWL",
            q_score=85.0,
            confidence=0.618,
            axiom_scores={},
            dog_votes={},
        )

        state.add_judgment(judgment)

        # Buffer reference should change
        assert state.recent_judgments is not original_buffer
        assert len(state.recent_judgments.buffer) == 1

    def test_conscious_state_multiple_adds(self):
        """Verify multiple adds to UnifiedConsciousState work correctly."""
        from cynic.kernel.core.unified_state import UnifiedConsciousState

        state = UnifiedConsciousState()

        for i in range(5):
            judgment = UnifiedJudgment(
                judgment_id=f"j{i}",
                verdict="HOWL",
                q_score=85.0,
                confidence=0.618,
                axiom_scores={},
                dog_votes={},
            )
            state.add_judgment(judgment)

        assert len(state.recent_judgments.buffer) == 5
        assert state.total_judgments == 5
