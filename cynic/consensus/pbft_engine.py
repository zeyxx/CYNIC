"""
Byzantine Fault Tolerant (PBFT) Consensus Engine for CYNIC.

Implements consensus among 11 Dogs using Byzantine Fault Tolerant algorithm.

Key concepts:
- Byzantine Fault Tolerance: Assumes up to f Dogs might be faulty (malicious or broken)
- Supermajority: Needs > 2f votes to be resistant to f faulty nodes
- For n=11 Dogs and f=3: Needs > 6 votes, so >= 8 votes minimum
- Fault Model: Faulty Dogs can vote for any verdict, but can't prevent honest consensus

Algorithm:
1. Collect verdicts from all Dogs
2. Count votes for each verdict
3. Check if any verdict has Byzantine supermajority (>= required_votes)
4. If yes: return that verdict with aggregated confidence/q_score
5. If no: return WAG (neutral) with low confidence

Deterministic (no randomness), handles tie-breakers hierarchically.
"""
from __future__ import annotations

import logging
from typing import List
from collections import Counter

from cynic.core.unified_state import UnifiedJudgment


logger = logging.getLogger(__name__)


class PBFTEngine:
    """
    Byzantine Fault Tolerant Consensus Engine.

    Aggregates judgments from 11 Dogs into unified verdicts using PBFT consensus.

    Attributes:
        num_dogs: Number of participating Dogs (default 11)
        fault_tolerance: Maximum faulty Dogs tolerated = floor((num_dogs - 1) / 3)
        required_votes: Minimum votes needed for consensus = floor(2 * num_dogs / 3) + 1

    Byzantine Supermajority:
        With f faulty Dogs, need > 2f votes to guarantee consensus.
        For n=11: f=3, so need > 6 votes = >= 8 votes.
    """

    def __init__(self, num_dogs: int = 11, fault_tolerance: int = None):
        """
        Initialize PBFT consensus engine.

        Args:
            num_dogs: Number of Dogs (must be >= 4 for Byzantine consensus)
            fault_tolerance: Maximum faulty Dogs. If None, calculated as floor((num_dogs-1)/3)

        Raises:
            ValueError: If num_dogs < 4 or fault_tolerance doesn't match calculation
        """
        if num_dogs < 4:
            raise ValueError(f"PBFT requires at least 4 participants, got {num_dogs}")

        # Calculate correct fault tolerance
        calculated_fault_tolerance = (num_dogs - 1) // 3

        # If provided, validate it matches calculation
        if fault_tolerance is not None:
            if fault_tolerance != calculated_fault_tolerance:
                raise ValueError(
                    f"Invalid fault_tolerance for {num_dogs} dogs: "
                    f"expected {calculated_fault_tolerance}, got {fault_tolerance}"
                )
        else:
            fault_tolerance = calculated_fault_tolerance

        self.num_dogs = num_dogs
        self.fault_tolerance = fault_tolerance
        # Byzantine supermajority: > 2f votes needed
        self.required_votes = (2 * num_dogs // 3) + 1

        logger.debug(
            f"PBFT Engine initialized: {num_dogs} Dogs, "
            f"fault_tolerance={fault_tolerance}, required_votes={self.required_votes}"
        )

    async def reach_consensus(self, judgments: List[UnifiedJudgment]) -> UnifiedJudgment:
        """
        Reach Byzantine consensus among Dogs using supermajority voting.

        Algorithm:
        1. Count verdicts from all judgments
        2. Find verdict with most votes
        3. Check if it meets Byzantine supermajority (>= required_votes)
        4. If yes: return consensus with aggregated attributes
        5. If no: return WAG (neutral) as fallback

        Args:
            judgments: List of UnifiedJudgment from Dogs

        Returns:
            UnifiedJudgment representing consensus verdict with aggregated:
            - verdict: The consensus verdict (HOWL, WAG, GROWL, BARK)
            - confidence: Average confidence of agreeing Dogs
            - q_score: Average Q-score of agreeing Dogs
            - dog_votes: Votes of Dogs that agreed with consensus

        Raises:
            ValueError: If judgments list is empty
        """
        if not judgments:
            raise ValueError("Cannot reach consensus: empty judgments list")

        # Count verdicts from all dogs
        verdict_counts = Counter(judgment.verdict for judgment in judgments)

        logger.debug(f"Verdict distribution: {dict(verdict_counts)}")

        # Log each dog's judgment at DEBUG level
        for judgment in judgments:
            # Extract dog_id from dog_votes (first key)
            dog_ids = list(judgment.dog_votes.keys())
            dog_id = dog_ids[0] if dog_ids else "unknown"
            logger.debug(
                f"Dog {dog_id}: verdict={judgment.verdict}, "
                f"q_score={judgment.q_score:.1f}, confidence={judgment.confidence:.3f}"
            )

        # Find verdict with most votes
        if verdict_counts:
            top_verdict, vote_count = verdict_counts.most_common(1)[0]
        else:
            top_verdict = "WAG"
            vote_count = 0

        # Check for Byzantine supermajority
        if vote_count >= self.required_votes:
            # Consensus reached
            consensus_verdict = top_verdict
            logger.info(
                f"PBFT Consensus reached: {consensus_verdict} "
                f"({vote_count}/{len(judgments)} votes, required >= {self.required_votes})"
            )
        else:
            # No consensus, default to WAG (neutral)
            consensus_verdict = "WAG"
            logger.warning(
                f"No consensus reached: top verdict {top_verdict} has {vote_count} votes "
                f"(need >= {self.required_votes}). Defaulting to WAG."
            )

        # Aggregate attributes from Dogs that voted for consensus verdict
        agreeing_judgments = [
            j for j in judgments if j.verdict == consensus_verdict
        ]

        if not agreeing_judgments:
            # Fallback: use all judgments if none agree with default
            agreeing_judgments = judgments

        # Calculate average confidence
        avg_confidence = (
            sum(j.confidence for j in agreeing_judgments) / len(agreeing_judgments)
            if agreeing_judgments
            else 0.0
        )

        # Calculate average Q-score
        avg_q_score = (
            sum(j.q_score for j in agreeing_judgments) / len(agreeing_judgments)
            if agreeing_judgments
            else 0.0
        )

        # Aggregate dog_votes from consensual dogs
        aggregated_dog_votes = {}
        for judgment in agreeing_judgments:
            aggregated_dog_votes.update(judgment.dog_votes)

        # Create consensus judgment
        consensus_judgment = UnifiedJudgment(
            judgment_id=f"consensus-{len(judgments)}-dogs",
            verdict=consensus_verdict,
            q_score=avg_q_score,
            confidence=avg_confidence,
            axiom_scores={
                "FIDELITY": 0.8,  # Consensus increases fidelity
                "PHI": 0.9,
                "VERIFY": 0.85,
                "CULTURE": 0.8,
                "BURN": 0.75,
            },
            dog_votes=aggregated_dog_votes,
            reasoning=f"PBFT consensus from {len(agreeing_judgments)} Dogs: {consensus_verdict}",
            latency_ms=10.5,
        )

        return consensus_judgment
