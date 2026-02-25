"""Test governance event payloads."""
from __future__ import annotations

from cynic.protocol.lnsp.governance_events import (
    GovernanceExecutionPayload,
    GovernanceOutcomePayload,
    GovernanceProposalPayload,
    GovernanceVotePayload,
)


def test_governance_proposal_payload():
    """Test GovernanceProposalPayload creation."""
    payload = GovernanceProposalPayload(
        proposal_id="prop_001",
        title="Burn 10% treasury",
        content="Reduce inflation by burning 10% annually",
        submitter_id="user_123",
        community_id="dogecoin",
        submission_timestamp=1708982400.0,
        voting_period_hours=48,
    )
    assert payload.proposal_id == "prop_001"
    assert payload.voting_period_hours == 48
    assert payload.community_id == "dogecoin"


def test_governance_vote_payload():
    """Test GovernanceVotePayload creation."""
    payload = GovernanceVotePayload(
        proposal_id="prop_001",
        voter_id="user_456",
        vote_choice="YES",
        timestamp=1708982500.0,
        community_id="dogecoin",
    )
    assert payload.vote_choice == "YES"
    assert payload.proposal_id == "prop_001"


def test_governance_execution_payload():
    """Test GovernanceExecutionPayload creation."""
    payload = GovernanceExecutionPayload(
        proposal_id="prop_001",
        success=True,
        tx_hash="0xabc123...",
        result={"burned_amount": 1000000},
        timestamp=1708982600.0,
        community_id="dogecoin",
    )
    assert payload.success is True
    assert payload.tx_hash == "0xabc123..."


def test_governance_outcome_payload():
    """Test GovernanceOutcomePayload creation."""
    payload = GovernanceOutcomePayload(
        proposal_id="prop_001",
        accepted=True,
        funds_received=True,
        community_sentiment=0.89,
        feedback_text="Great decision, community is happy!",
        timestamp=1708982700.0,
        community_id="dogecoin",
    )
    assert payload.accepted is True
    assert payload.community_sentiment == 0.89
