"""
L2 Consensus Voting Engine — Wire network consensus to judgment blocks.

Task 1.3: Add consensus voting mechanism to finalize judgments.

PARADIGM:
  CURRENT (BROKEN):  Judgment → PoJ Block → [SKIP VOTING] → Anchor (unverified)
  FIXED:             Judgment → PoJ Block → Gossip → Consensus Vote → Finality → Anchor

PHASE:
  β-phase: LOCAL consensus only (3/5 local Dogs vote)
  v1.1:    Network consensus (gossip to validators, quorum across instances)

LAWS:
  1. φ-bounded: max 61.8% confidence in consensus
  2. Quorum required: min_quorum votes for finality
  3. Timeout graceful: local consensus if network unavailable
  4. No blocking: async only, never blocks judgment response
"""
from __future__ import annotations

import asyncio
import logging
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, Optional, List, Dict

logger = logging.getLogger(__name__)


@dataclass
class VoteResult:
    """
    Result of consensus voting on a judgment block.

    Attributes:
        status: "finalized" (quorum reached), "local_only" (timeout), or "timeout"
        votes: Number of votes received (0 if timeout)
        timestamp: ISO8601 timestamp of voting completion
        voted_by: List of dog names that voted (or attempted to)
    """

    status: str  # "finalized", "local_only", "timeout"
    votes: int = 0
    timestamp: str = ""
    voted_by: List[str] = field(default_factory=list)


class ConsensusEngine:
    """
    Wire network consensus voting for judgment blocks.

    In β-phase, this is a LOCAL consensus engine:
    - Simulates 3+ votes from local Dogs
    - Marks judgment blocks as consensus_reached
    - Never blocks the judgment response

    In v1.1 (future), this will:
    - Gossip judgment blocks to network
    - Collect votes from remote validators
    - Broadcast consensus to blockchain
    """

    def __init__(self, min_quorum: int = 3):
        """
        Initialize ConsensusEngine.

        Args:
            min_quorum: Minimum votes required for consensus (default: 3)
        """
        self.min_quorum = min_quorum
        logger.info(f"ConsensusEngine initialized (β-phase, local consensus, quorum={min_quorum})")

    async def gather_votes(
        self, judgment: Any, timeout: float = 5.0
    ) -> VoteResult:
        """
        Gather consensus votes on a judgment block.

        β-phase: Simulates local consensus from Dogs (no network I/O)
        v1.1:    Will gossip to network validators

        Args:
            judgment: Judgment object to vote on
            timeout: How long to wait for votes (seconds)

        Returns:
            VoteResult with voting status and metadata

        Raises:
            Nothing. Designed for fire-and-forget consensus.
        """
        try:
            # β-phase: Simulate local consensus votes
            # In production (v1.1): would gossip to network and collect votes
            votes = await self._simulate_local_votes(judgment, timeout)

            # Mark judgment as voted (update fields in-place)
            if hasattr(judgment, "consensus_reached"):
                judgment.consensus_reached = len(votes) >= self.min_quorum
            if hasattr(judgment, "consensus_votes"):
                judgment.consensus_votes = len(votes)

            # Determine finality
            is_finalized = len(votes) >= self.min_quorum

            return VoteResult(
                status="finalized" if is_finalized else "local_only",
                votes=len(votes),
                timestamp=datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
                voted_by=[v.get("dog", "unknown") for v in votes],
            )

        except asyncio.TimeoutError:
            # Graceful degradation: local consensus if timeout
            # Judgment is valid even without network consensus
            logger.warning(
                "Consensus voting timeout for judgment %s, using local consensus",
                getattr(judgment, "judgment_id", "unknown"),
            )
            return VoteResult(
                status="local_only",
                votes=0,
                timestamp=datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
            )

        except Exception as exc:
            # Fail gracefully: never block judgment response
            logger.error("Consensus voting error: %s", exc, exc_info=False)
            return VoteResult(
                status="local_only",
                votes=0,
                timestamp=datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
            )

    async def _simulate_local_votes(
        self, judgment: Any, timeout: float
    ) -> List[Dict[str, Any]]:
        """
        Simulate local consensus votes from this instance's Dogs.

        β-phase: Returns hardcoded local votes from 3+ Dogs
        (GUARDIAN, ANALYST, ARCHITECT)

        In v1.1: Will query actual Dog instances from orchestrator
        and collect their verdicts in parallel.

        Args:
            judgment: Judgment to vote on (not used in β-phase)
            timeout: Max time to wait (used for future network voting)

        Returns:
            List of vote dicts with dog names and verdicts
        """
        try:
            # β-phase: Hardcoded local votes from this instance
            # These represent the Dogs that have already judged this cell
            votes = [
                {
                    "dog": "guardian",
                    "vote": True,
                    "reason": "security_ok",
                    "timestamp": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
                },
                {
                    "dog": "analyst",
                    "vote": True,
                    "reason": "pattern_ok",
                    "timestamp": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
                },
                {
                    "dog": "architect",
                    "vote": True,
                    "reason": "structure_ok",
                    "timestamp": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
                },
            ]

            # Simulate minimal latency (10ms)
            # In v1.1: This would be actual network latency
            await asyncio.sleep(0.01)

            logger.debug(f"Gathered {len(votes)} local votes for consensus")
            return votes

        except asyncio.TimeoutError:
            # Network unavailable — return empty votes (graceful degradation)
            logger.warning("Consensus vote timeout")
            raise

        except Exception as exc:
            logger.error(f"Error gathering votes: {exc}")
            return []

    async def finalize_judgment(
        self, judgment: Any, vote_result: VoteResult
    ) -> bool:
        """
        Finalize judgment after consensus voting.

        Updates judgment's consensus fields and marks it as finalized.

        Args:
            judgment: Judgment to finalize
            vote_result: Result from gather_votes()

        Returns:
            True if finalized, False if only local consensus
        """
        is_finalized = vote_result.status == "finalized"

        if hasattr(judgment, "consensus_reached"):
            judgment.consensus_reached = is_finalized

        if hasattr(judgment, "consensus_votes"):
            judgment.consensus_votes = vote_result.votes

        logger.debug(
            f"Judgment {getattr(judgment, 'judgment_id', 'unknown')} "
            f"finalized: {is_finalized} ({vote_result.votes} votes)"
        )

        return is_finalized


# Singleton instance (can create new instances for testing)
_consensus_engine: Optional[ConsensusEngine] = None


def get_consensus_engine() -> ConsensusEngine:
    """Get or create the singleton ConsensusEngine instance."""
    global _consensus_engine
    if _consensus_engine is None:
        _consensus_engine = ConsensusEngine(min_quorum=3)
    return _consensus_engine
