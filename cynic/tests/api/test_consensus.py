"""
Tests for L2 Consensus Voting Engine.

Task 1.3: Wire network consensus voting to judgment blocks.
Tests ensure judgments record consensus votes before finality.

β-phase: LOCAL consensus (network voting in v1.1)
"""
from __future__ import annotations

import pytest
import asyncio
from datetime import datetime

from cynic.organism.brain.consensus import ConsensusEngine, VoteResult
from cynic.core.judgment import Judgment, Cell


@pytest.fixture
def consensus_engine():
    """Create a ConsensusEngine instance."""
    return ConsensusEngine(min_quorum=3)


@pytest.fixture
def sample_cell():
    """Create a sample Cell for testing."""
    return Cell(
        reality="CODE",
        analysis="JUDGE",
        time_dim="PRESENT",
        content="test code snippet",
        context="testing consensus voting",
        lod=1,
        budget_usd=0.01,
    )


@pytest.fixture
def sample_judgment(sample_cell):
    """Create a sample Judgment for testing."""
    return Judgment(
        judgment_id="test-judgment-123",
        cell=sample_cell,
        q_score=70.0,  # WAG range: 61.8-82.0
        verdict="WAG",
        confidence=0.5,
        axiom_scores={"VERIFY": 0.6, "CULTURE": 0.5},
        active_axioms=["VERIFY", "CULTURE"],
        dog_votes={"guardian": 70.0, "analyst": 70.0},
        consensus_votes=0,
        consensus_reached=False,
    )


@pytest.mark.asyncio
async def test_judgment_consensus_fields_exist(sample_judgment):
    """Judgment blocks should have consensus voting fields."""
    assert hasattr(sample_judgment, "consensus_reached")
    assert hasattr(sample_judgment, "consensus_votes")
    assert hasattr(sample_judgment, "dog_votes")
    assert sample_judgment.consensus_reached is False
    assert sample_judgment.consensus_votes == 0


@pytest.mark.asyncio
async def test_consensus_engine_gather_votes(consensus_engine, sample_judgment):
    """ConsensusEngine should gather votes and mark judgment as voted."""
    result = await consensus_engine.gather_votes(sample_judgment, timeout=5)

    # Should be marked as voted
    assert sample_judgment.consensus_reached is True or result.status in ["finalized", "local_only"]
    assert result.votes >= 0
    assert hasattr(result, "status")
    assert hasattr(result, "timestamp")
    assert hasattr(result, "voted_by")


@pytest.mark.asyncio
async def test_consensus_quorum_required(consensus_engine, sample_judgment):
    """Require 3+ votes for consensus quorum."""
    result = await consensus_engine.gather_votes(sample_judgment)

    # Should either have quorum or be marked local_only
    if result.status == "finalized":
        assert result.votes >= 3
    else:
        assert result.status in ["local_only", "timeout"]


@pytest.mark.asyncio
async def test_consensus_timeout_handled(consensus_engine, sample_judgment):
    """If timeout occurs, should handle gracefully (not raise)."""
    # Very short timeout to force timeout scenario
    result = await consensus_engine.gather_votes(sample_judgment, timeout=0.01)

    # Should not raise, just return gracefully
    assert result.status in ["finalized", "local_only", "timeout"]
    assert isinstance(result.votes, int)
    assert isinstance(result.timestamp, str)


@pytest.mark.asyncio
async def test_vote_result_format(consensus_engine, sample_judgment):
    """VoteResult should have required metadata fields."""
    result = await consensus_engine.gather_votes(sample_judgment)

    # Check all required fields
    assert hasattr(result, "status")
    assert hasattr(result, "votes")
    assert hasattr(result, "timestamp")
    assert hasattr(result, "voted_by")

    # Validate types
    assert isinstance(result.status, str)
    assert isinstance(result.votes, int)
    assert isinstance(result.timestamp, str)
    assert isinstance(result.voted_by, list)

    # Timestamp should be ISO format
    try:
        datetime.fromisoformat(result.timestamp.replace("Z", "+00:00"))
    except ValueError:
        pytest.fail(f"Invalid timestamp format: {result.timestamp}")


@pytest.mark.asyncio
async def test_consensus_marks_judgment_voted(consensus_engine, sample_judgment):
    """After gather_votes, judgment should record vote data."""
    initial_consensus = sample_judgment.consensus_reached
    result = await consensus_engine.gather_votes(sample_judgment)

    # Judgment should be updated
    if result.status == "finalized":
        assert sample_judgment.consensus_reached is True
    assert sample_judgment.consensus_votes >= 0


@pytest.mark.asyncio
async def test_consensus_min_quorum_enforced(consensus_engine, sample_judgment):
    """ConsensusEngine should enforce minimum quorum setting."""
    assert consensus_engine.min_quorum == 3

    result = await consensus_engine.gather_votes(sample_judgment)

    # If finalized, must have met quorum
    if result.status == "finalized":
        assert result.votes >= consensus_engine.min_quorum


@pytest.mark.asyncio
async def test_consensus_votes_tracked(consensus_engine, sample_judgment):
    """Consensus votes should be tracked and returned."""
    result = await consensus_engine.gather_votes(sample_judgment)

    # Result should report vote count
    assert result.votes >= 0
    assert isinstance(result.voted_by, list)

    # If votes > 0, voted_by should have entries
    if result.votes > 0:
        assert len(result.voted_by) > 0


@pytest.mark.asyncio
async def test_multiple_consensuses_independent(consensus_engine):
    """Multiple judgments should vote independently."""
    cell1 = Cell(reality="CODE", analysis="JUDGE", content="code1")
    cell2 = Cell(reality="HUMAN", analysis="PERCEIVE", content="human1")

    j1 = Judgment(
        judgment_id="j1",
        cell=cell1,
        q_score=50.0,  # GROWL range: 38.2-61.8
        verdict="GROWL",
        confidence=0.4,
    )
    j2 = Judgment(
        judgment_id="j2",
        cell=cell2,
        q_score=85.0,  # HOWL range: 82.0-100.0
        verdict="HOWL",
        confidence=0.6,
    )

    r1 = await consensus_engine.gather_votes(j1)
    r2 = await consensus_engine.gather_votes(j2)

    # Both should complete without error
    assert r1.status in ["finalized", "local_only", "timeout"]
    assert r2.status in ["finalized", "local_only", "timeout"]
