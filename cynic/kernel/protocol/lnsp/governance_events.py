"""Governance event payloads for LNSP integration."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass
class GovernanceProposalPayload:
    """A governance proposal submitted to the community."""

    proposal_id: str
    title: str
    content: str
    submitter_id: str
    community_id: str
    submission_timestamp: float
    voting_period_hours: int


@dataclass
class GovernanceVotePayload:
    """A vote cast on a governance proposal."""

    proposal_id: str
    voter_id: str
    vote_choice: str  # "YES", "NO", "ABSTAIN"
    timestamp: float
    community_id: str


@dataclass
class GovernanceExecutionPayload:
    """Outcome of executing a governance decision on-chain."""

    proposal_id: str
    success: bool
    tx_hash: str | None
    result: dict[str, Any]
    timestamp: float
    community_id: str


@dataclass
class GovernanceOutcomePayload:
    """Community feedback on a governance decision outcome."""

    proposal_id: str
    accepted: bool  # Community accepted the outcome
    funds_received: bool  # Funds reached treasury (if applicable)
    community_sentiment: float  # 0.0 to 1.0
    feedback_text: str
    timestamp: float
    community_id: str
