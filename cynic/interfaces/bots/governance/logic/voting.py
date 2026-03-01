"""Advanced voting mechanisms"""

import math
from dataclasses import dataclass
from enum import Enum


class VotingType(str, Enum):
    """Types of voting mechanisms"""
    SIMPLE = "simple"  # 1 vote per user
    QUADRATIC = "quadratic"  # Cost increases quadratically
    WEIGHTED = "weighted"  # Weight based on reputation
    DELEGATED = "delegated"  # Votes can be delegated

@dataclass
class Vote:
    """Single vote record"""
    voter_id: str
    proposal_id: str
    choice: bool  # True = for, False = against
    weight: float = 1.0  # Vote weight (reputation-based)
    delegated_from: str = None  # If delegated, who it came from

class VoteCounter:
    """Count and tally votes with different mechanisms"""

    @staticmethod
    def simple_majority(votes: list[Vote]) -> tuple[bool, float]:
        """Simple majority voting"""
        if not votes:
            return False, 0.5

        for_votes = sum(1 for v in votes if v.choice)
        total = len(votes)

        approval_rate = for_votes / total
        return for_votes > total / 2, approval_rate

    @staticmethod
    def quadratic_voting(votes: list[Vote]) -> tuple[bool, float]:
        """Quadratic voting: cost increases as sqrt(votes)"""
        if not votes:
            return False, 0.5

        for_votes = sum(math.sqrt(1) for v in votes if v.choice)
        against_votes = sum(math.sqrt(1) for v in votes if not v.choice)
        total = for_votes + against_votes

        if total == 0:
            return False, 0.5

        approval_rate = for_votes / total
        return for_votes > total / 2, approval_rate

    @staticmethod
    def weighted_voting(votes: list[Vote]) -> tuple[bool, float]:
        """Weighted voting: vote weight = reputation"""
        if not votes:
            return False, 0.5

        for_weight = sum(v.weight for v in votes if v.choice)
        against_weight = sum(v.weight for v in votes if not v.choice)
        total_weight = for_weight + against_weight

        if total_weight == 0:
            return False, 0.5

        approval_rate = for_weight / total_weight
        return for_weight > total_weight / 2, approval_rate

class DelegationManager:
    """Manage vote delegation"""

    def __init__(self):
        self.delegations: dict[str, str] = {}  # voter_id -> delegate_id

    def delegate_vote(self, voter_id: str, delegate_id: str):
        """Delegate vote to another user"""
        self.delegations[voter_id] = delegate_id

    def revoke_delegation(self, voter_id: str):
        """Revoke vote delegation"""
        if voter_id in self.delegations:
            del self.delegations[voter_id]

    def get_delegated_votes(self, delegate_id: str) -> list[str]:
        """Get all voters delegated to this user"""
        return [v for v, d in self.delegations.items() if d == delegate_id]
