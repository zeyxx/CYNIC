"""Test Priority 4 Phase 1: Core State Model Immutability

Verifies that ValueCreation, ImpactMeasurement, GovernanceCommunity,
GovernanceProposal, and GovernanceVote are frozen dataclasses with evolve() methods.
"""

from __future__ import annotations

from dataclasses import FrozenInstanceError

import pytest

from cynic.kernel.core.unified_state import (
    GovernanceCommunity,
    GovernanceProposal,
    GovernanceVote,
    ImpactMeasurement,
    ValueCreation,
)


class TestValueCreationImmutability:
    """Test ValueCreation is properly frozen."""

    def test_value_creation_frozen(self):
        """Verify ValueCreation is truly immutable (frozen=True)."""
        vc = ValueCreation(
            creation_id="vc123",
            creator_id="creator1",
            creation_type="code",
            description="Added feature X",
            direct_impact=0.8,
            indirect_impact=0.3,
        )

        # Attempt to modify should raise FrozenInstanceError
        with pytest.raises(FrozenInstanceError):
            vc.direct_impact = 0.9

        with pytest.raises(FrozenInstanceError):
            vc.description = "Modified description"

    def test_value_creation_evolve(self):
        """Test evolve() creates new instance with updated fields."""
        original = ValueCreation(
            creation_id="vc123",
            creator_id="creator1",
            creation_type="code",
            description="Original description",
            direct_impact=0.5,
            indirect_impact=0.2,
            collective_impact=0.1,
            temporal_impact=0.05,
        )

        # Evolve with single field update
        updated = original.evolve(direct_impact=0.9)

        # Original unchanged
        assert original.direct_impact == 0.5
        assert original.description == "Original description"

        # New instance has updated field
        assert updated.direct_impact == 0.9
        assert updated.description == "Original description"
        assert updated.creation_id == original.creation_id

        # Different instances
        assert original is not updated

    def test_value_creation_evolve_multiple_fields(self):
        """Test evolve() with multiple field updates."""
        original = ValueCreation(
            creation_id="vc123",
            creator_id="creator1",
            creation_type="code",
            description="Original",
            direct_impact=0.5,
        )

        updated = original.evolve(
            description="Updated",
            direct_impact=0.8,
            indirect_impact=0.4,
        )

        assert original.description == "Original"
        assert original.direct_impact == 0.5
        assert original.indirect_impact == 0.0

        assert updated.description == "Updated"
        assert updated.direct_impact == 0.8
        assert updated.indirect_impact == 0.4


class TestImpactMeasurementImmutability:
    """Test ImpactMeasurement is properly frozen."""

    def test_impact_measurement_frozen(self):
        """Verify ImpactMeasurement is truly immutable."""
        im = ImpactMeasurement(
            human_id="h123",
            total_impact=0.7,
            dimension_scores={"code": 0.8, "community": 0.6},
            governance_weight=0.15,
        )

        with pytest.raises(FrozenInstanceError):
            im.total_impact = 0.9

        with pytest.raises(FrozenInstanceError):
            im.governance_weight = 0.2

    def test_impact_measurement_evolve(self):
        """Test evolve() creates new instance with updated fields."""
        original = ImpactMeasurement(
            human_id="h123",
            total_impact=0.5,
            dimension_scores={"code": 0.6, "community": 0.4},
            governance_weight=0.1,
        )

        updated = original.evolve(
            total_impact=0.8,
            governance_weight=0.25,
        )

        assert original.total_impact == 0.5
        assert original.governance_weight == 0.1

        assert updated.total_impact == 0.8
        assert updated.governance_weight == 0.25
        assert updated.human_id == original.human_id


class TestGovernanceCommunityImmutability:
    """Test GovernanceCommunity is properly frozen."""

    def test_governance_community_frozen(self):
        """Verify GovernanceCommunity is truly immutable."""
        gc = GovernanceCommunity(
            community_id="comm123",
            name="Test Community",
            platform="discord",
            token_symbol="TEST",
        )

        with pytest.raises(FrozenInstanceError):
            gc.name = "Modified Name"

        with pytest.raises(FrozenInstanceError):
            gc.quorum_pct = 50.0

    def test_governance_community_evolve(self):
        """Test evolve() creates new instance with updated fields."""
        original = GovernanceCommunity(
            community_id="comm123",
            name="Test Community",
            platform="discord",
            token_symbol="TEST",
            voting_period_h=72,
            quorum_pct=25.0,
        )

        updated = original.evolve(
            name="Updated Community",
            voting_period_h=48,
            quorum_pct=30.0,
        )

        assert original.name == "Test Community"
        assert original.voting_period_h == 72
        assert original.quorum_pct == 25.0

        assert updated.name == "Updated Community"
        assert updated.voting_period_h == 48
        assert updated.quorum_pct == 30.0
        assert updated.community_id == original.community_id


class TestGovernanceProposalImmutability:
    """Test GovernanceProposal is properly frozen."""

    def test_governance_proposal_frozen(self):
        """Verify GovernanceProposal is truly immutable."""
        gp = GovernanceProposal(
            proposal_id="prop123",
            community_id="comm123",
            proposer_id="proposer1",
            title="Add feature X",
            description="Description",
            category="feature",
        )

        with pytest.raises(FrozenInstanceError):
            gp.status = "APPROVED"

        with pytest.raises(FrozenInstanceError):
            gp.yes_votes = 100.0

    def test_governance_proposal_evolve(self):
        """Test evolve() creates new instance with updated fields."""
        original = GovernanceProposal(
            proposal_id="prop123",
            community_id="comm123",
            proposer_id="proposer1",
            title="Add feature X",
            description="Description",
            category="feature",
            status="PENDING",
            yes_votes=0.0,
            no_votes=0.0,
        )

        updated = original.evolve(
            status="APPROVED",
            yes_votes=100.0,
            no_votes=20.0,
        )

        assert original.status == "PENDING"
        assert original.yes_votes == 0.0
        assert original.no_votes == 0.0

        assert updated.status == "APPROVED"
        assert updated.yes_votes == 100.0
        assert updated.no_votes == 20.0
        assert updated.proposal_id == original.proposal_id


class TestGovernanceVoteImmutability:
    """Test GovernanceVote is properly frozen."""

    def test_governance_vote_frozen(self):
        """Verify GovernanceVote is truly immutable."""
        gv = GovernanceVote(
            vote_id="vote123",
            proposal_id="prop123",
            voter_id="voter1",
            choice="yes",
            weight=1.0,
        )

        with pytest.raises(FrozenInstanceError):
            gv.choice = "no"

        with pytest.raises(FrozenInstanceError):
            gv.weight = 2.0

    def test_governance_vote_evolve(self):
        """Test evolve() creates new instance with updated fields."""
        original = GovernanceVote(
            vote_id="vote123",
            proposal_id="prop123",
            voter_id="voter1",
            choice="yes",
            weight=1.0,
        )

        # Note: evolved votes would have different vote_id in real scenario
        updated = original.evolve(weight=2.5)

        assert original.weight == 1.0
        assert updated.weight == 2.5
        assert updated.voter_id == original.voter_id


class TestEvolveChaining:
    """Test evolve() method chaining patterns."""

    def test_evolve_chain_multiple_updates(self):
        """Test chaining multiple evolve() calls."""
        original = ValueCreation(
            creation_id="vc123",
            creator_id="creator1",
            creation_type="code",
            description="Original",
            direct_impact=0.1,
            indirect_impact=0.0,
        )

        # Chain evolves
        v2 = original.evolve(direct_impact=0.5)
        v3 = v2.evolve(indirect_impact=0.3)
        v4 = v3.evolve(description="Updated", collective_impact=0.2)

        # Original unchanged
        assert original.direct_impact == 0.1
        assert original.indirect_impact == 0.0

        # Chain preserved
        assert v2.direct_impact == 0.5
        assert v2.indirect_impact == 0.0

        assert v3.direct_impact == 0.5
        assert v3.indirect_impact == 0.3
        assert v3.description == "Original"

        assert v4.direct_impact == 0.5
        assert v4.indirect_impact == 0.3
        assert v4.description == "Updated"
        assert v4.collective_impact == 0.2


class TestImmutabilityInCollections:
    """Test that frozen models work properly in collections."""

    def test_frozen_models_in_list(self):
        """Verify frozen models can be stored in lists."""
        values = [
            ValueCreation(
                creation_id=f"vc{i}",
                creator_id="creator1",
                creation_type="code",
                description=f"Creation {i}",
                direct_impact=0.5 + i * 0.1,
            )
            for i in range(5)
        ]

        assert len(values) == 5
        assert all(isinstance(v, ValueCreation) for v in values)

        # Each is immutable
        for v in values:
            with pytest.raises(FrozenInstanceError):
                v.direct_impact = 0.9

    def test_frozen_models_in_dict(self):
        """Verify frozen models can be used as dict values."""
        proposals = {
            f"prop{i}": GovernanceProposal(
                proposal_id=f"prop{i}",
                community_id="comm123",
                proposer_id=f"proposer{i}",
                title=f"Proposal {i}",
                description=f"Description {i}",
                category="feature",
            )
            for i in range(3)
        }

        assert len(proposals) == 3
        for prop in proposals.values():
            with pytest.raises(FrozenInstanceError):
                prop.status = "APPROVED"
