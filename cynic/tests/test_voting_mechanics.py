"""Tests for voting mechanisms"""

import pytest
from governance_bot.voting import Vote, VoteCounter, VotingType, DelegationManager

class TestSimpleVoting:
    """Test simple voting mechanism"""

    def test_simple_majority_approve(self):
        """Test simple majority approval"""
        votes = [
            Vote(voter_id="user_1", proposal_id="prop_1", choice=True),
            Vote(voter_id="user_2", proposal_id="prop_1", choice=True),
            Vote(voter_id="user_3", proposal_id="prop_1", choice=False),
        ]

        approved, rate = VoteCounter.simple_majority(votes)

        assert approved == True
        assert rate == 2/3

    def test_simple_majority_reject(self):
        """Test simple majority rejection"""
        votes = [
            Vote(voter_id="user_1", proposal_id="prop_1", choice=False),
            Vote(voter_id="user_2", proposal_id="prop_1", choice=False),
            Vote(voter_id="user_3", proposal_id="prop_1", choice=True),
        ]

        approved, rate = VoteCounter.simple_majority(votes)

        assert approved == False
        assert rate == 1/3

class TestWeightedVoting:
    """Test weighted voting mechanism"""

    def test_weighted_voting_approve(self):
        """Test weighted voting with reputation"""
        votes = [
            Vote(voter_id="user_1", proposal_id="prop_1", choice=True, weight=0.8),
            Vote(voter_id="user_2", proposal_id="prop_1", choice=True, weight=0.6),
            Vote(voter_id="user_3", proposal_id="prop_1", choice=False, weight=0.5),
        ]

        approved, rate = VoteCounter.weighted_voting(votes)

        assert approved == True
        for_weight = 0.8 + 0.6
        total_weight = 0.8 + 0.6 + 0.5
        assert abs(rate - for_weight/total_weight) < 0.001

class TestDelegation:
    """Test vote delegation"""

    def test_delegate_vote(self):
        """Test delegating vote"""
        manager = DelegationManager()
        manager.delegate_vote("user_1", "user_2")

        delegated = manager.get_delegated_votes("user_2")

        assert "user_1" in delegated

    def test_revoke_delegation(self):
        """Test revoking delegation"""
        manager = DelegationManager()
        manager.delegate_vote("user_1", "user_2")
        manager.revoke_delegation("user_1")

        delegated = manager.get_delegated_votes("user_2")

        assert "user_1" not in delegated

    def test_multiple_delegations(self):
        """Test multiple users delegating to one"""
        manager = DelegationManager()
        manager.delegate_vote("user_1", "delegate")
        manager.delegate_vote("user_2", "delegate")
        manager.delegate_vote("user_3", "delegate")

        delegated = manager.get_delegated_votes("delegate")

        assert len(delegated) == 3
