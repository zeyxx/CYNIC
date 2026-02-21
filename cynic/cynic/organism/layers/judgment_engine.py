"""Layer 1: Judgment Engine — Unified Will (Not Averaging)

The organism doesn't take votes from dogs and average them.
The organism judges. Dogs provide INPUT. The engine DECIDES.

Key principle:
- Dogs are INPUT SENSORS (not judges)
- Engine is DECISION MAKER (unified will)
- Axioms (Layer 0) are CONSTRAINTS (vetoes)

This replaces orchestrator.run() which averaged Q-scores.
"""

from __future__ import annotations

import asyncio
import logging
from dataclasses import dataclass
from typing import Any, Callable, Optional
from math import log, exp

from cynic.core.phi import MAX_Q_SCORE, MAX_CONFIDENCE, PHI_INV, PHI_INV_2
from cynic.organism.layers.identity import OrganismIdentity

logger = logging.getLogger(__name__)


@dataclass
class DogInput:
    """Raw judgment from a dog (input sensor)."""
    dog_id: str
    q_score: float
    confidence: float
    verdict: str
    justification: str = ""
    metadata: dict = None


@dataclass
class UnifiedJudgment:
    """Single judgment from the engine (unified will).

    NOT a vote. NOT an average. An EXECUTIVE DECISION.
    """
    q_score: float
    verdict: str
    confidence: float
    justification: str
    precedent: str
    cost_usd: float
    dog_votes: dict[str, float]  # For transparency: which dogs voted which way
    algorithm: str  # "geometric_mean", "highest_confidence", "pbft", etc.
    layer0_violations: list[str]  # Should be empty


class JudgmentEngine:
    """Unified will. Not averaging. Not voting.

    The organism's decision maker.

    Public API:
    - __init__(identity: OrganismIdentity, ...)
    - judge(dog_inputs: list[DogInput], context: dict) -> UnifiedJudgment
    """

    # Verdict thresholds (from MEMORY.md critical rules)
    BARK_MAX = 38.2    # Q < 38.2
    GROWL_MAX = 61.8   # Q < 61.8
    WAG_MAX = 82.0     # Q < 82.0
    HOWL_MIN = 82.0    # Q >= 82.0

    def __init__(
        self,
        identity: OrganismIdentity,
        algorithm: str = "geometric_mean",
        axiom_penalty: float = 0.5,
    ):
        """Initialize judgment engine.

        Args:
            identity: Layer 0 (OrganismIdentity with axiom constraints)
            algorithm: How to compute unified will from dog inputs
                - "geometric_mean": log-weighted average (φ-respecting)
                - "highest_confidence": pick dog with highest confidence
                - "pbft": Byzantine fault tolerant consensus
            axiom_penalty: If dog violates axiom, multiply confidence by this (0.0-1.0)
        """
        self.identity = identity
        self.algorithm = algorithm
        self.axiom_penalty = axiom_penalty

        if not (0.0 <= axiom_penalty <= 1.0):
            raise ValueError(f"axiom_penalty must be in [0.0, 1.0], got {axiom_penalty}")

    def _apply_axiom_penalties(self, dog_inputs: list[DogInput]) -> list[DogInput]:
        """Apply Layer 0 constraints to dog inputs.

        If a dog violates axioms, penalize its confidence.
        Dogs that violate FIDELITY (high confidence) are especially penalized.
        """
        penalized = []
        for dog_input in dog_inputs:
            # Create a mock judgment-like object from dog input
            class DogJudgmentView:
                def __init__(self, di):
                    self.q_score = di.q_score
                    self.confidence = di.confidence
                    self.justification = di.justification or "dog_input"
                    self.precedent = f"dog_{di.dog_id}"
                    self.cost_usd = 0.001  # Assume minimal dog cost

            view = DogJudgmentView(dog_input)
            violations = self.identity.validate_judgment(view)

            # Penalize if violated
            if violations:
                logger.debug(
                    "Dog %s violated axioms: %s. Penalizing confidence %.3f → %.3f",
                    dog_input.dog_id,
                    violations,
                    dog_input.confidence,
                    dog_input.confidence * self.axiom_penalty,
                )
                dog_input.confidence *= self.axiom_penalty

            penalized.append(dog_input)

        return penalized

    def _compute_geometric_mean(self, dog_inputs: list[DogInput]) -> float:
        """Compute Q-Score as φ-bounded geometric mean.

        Formula (from MEMORY.md):
            log_sum = sum(weight[p] * log(max(score, 0.1)) for each dog)
            geo_mean = exp(log_sum / total_weight)
            return clamp(geo_mean, 0, 100)

        This respects φ and prevents extreme values.
        """
        if not dog_inputs:
            return 50.0  # Neutral default

        # Simple equal weights (could be confidence-weighted)
        weights = {di.dog_id: 1.0 for di in dog_inputs}
        total_weight = sum(weights.values())

        log_sum = 0.0
        for dog_input in dog_inputs:
            weight = weights[dog_input.dog_id]
            score = max(dog_input.q_score, 0.1)  # Avoid log(0)
            log_sum += weight * log(score)

        geo_mean = exp(log_sum / total_weight)
        # Clamp to [0, 100], no rescaling
        return min(max(geo_mean, 0.0), MAX_Q_SCORE)

    def _compute_highest_confidence(self, dog_inputs: list[DogInput]) -> float:
        """Pick the dog with highest confidence, use its Q-Score.

        Simple but strong: trust the most confident dog (after axiom penalties).
        Note: This method is called AFTER _apply_axiom_penalties, so confidences are already adjusted.
        """
        if not dog_inputs:
            return 50.0

        best = max(dog_inputs, key=lambda di: di.confidence)
        return best.q_score

    def _compute_pbft_consensus(self, dog_inputs: list[DogInput]) -> float:
        """Byzantine Fault Tolerant consensus.

        Reject outliers, take median of remaining.
        Requires f < n/3 Byzantine dogs (n=11, f<3 tolerated).
        """
        if not dog_inputs:
            return 50.0

        if len(dog_inputs) < 4:
            # Too few dogs for PBFT, fall back to geometric mean
            return self._compute_geometric_mean(dog_inputs)

        scores = sorted([di.q_score for di in dog_inputs])
        # Reject bottom 1 and top 1 as outliers
        trimmed = scores[1:-1]

        if not trimmed:
            return 50.0

        # Return median of trimmed scores
        mid = len(trimmed) // 2
        if len(trimmed) % 2 == 0:
            return (trimmed[mid - 1] + trimmed[mid]) / 2.0
        return float(trimmed[mid])

    def _q_score_to_verdict(self, q_score: float) -> str:
        """Convert Q-Score to verdict.

        BARK < 38.2
        GROWL [38.2, 61.8)
        WAG [61.8, 82.0)
        HOWL [82.0, 100]
        """
        if q_score < self.BARK_MAX:
            return "BARK"
        elif q_score < self.GROWL_MAX:
            return "GROWL"
        elif q_score < self.WAG_MAX:
            return "WAG"
        else:
            return "HOWL"

    def _compute_unified_confidence(self, dog_inputs: list[DogInput]) -> float:
        """Compute unified confidence from dog inputs.

        Average confidence, capped at φ⁻¹ (0.618).
        High agreement increases confidence; disagreement decreases it.
        """
        if not dog_inputs:
            return 0.0

        avg_conf = sum(di.confidence for di in dog_inputs) / len(dog_inputs)
        # Compute disagreement (variance)
        variance = sum((di.confidence - avg_conf) ** 2 for di in dog_inputs) / len(dog_inputs)
        disagreement_penalty = 1.0 - min(variance / 0.1, 1.0)  # 0 variance = 1.0, high variance = 0

        unified_conf = avg_conf * disagreement_penalty
        return min(unified_conf, MAX_CONFIDENCE)  # φ-bound

    async def judge(
        self,
        dog_inputs: list[DogInput],
        context: Optional[dict] = None,
    ) -> UnifiedJudgment:
        """Compute unified judgment from dog inputs.

        Args:
            dog_inputs: Raw judgments from all dogs
            context: Optional context (precedent, cost info, etc.)

        Returns:
            UnifiedJudgment (single, unified, Layer 0 validated)
        """
        if not dog_inputs:
            # No dogs available — neutral judgment
            judgment = UnifiedJudgment(
                q_score=50.0,
                verdict="WAG",
                confidence=0.0,
                justification="No dog inputs available",
                precedent="fallback",
                cost_usd=0.0,
                dog_votes={},
                algorithm="fallback",
                layer0_violations=[],
            )
            return judgment

        # Step 1: Apply Layer 0 axiom penalties
        penalized = self._apply_axiom_penalties(dog_inputs)

        # Step 2: Compute Q-Score using selected algorithm
        if self.algorithm == "geometric_mean":
            q_score = self._compute_geometric_mean(penalized)
        elif self.algorithm == "highest_confidence":
            q_score = self._compute_highest_confidence(penalized)
        elif self.algorithm == "pbft":
            q_score = self._compute_pbft_consensus(penalized)
        else:
            raise ValueError(f"Unknown algorithm: {self.algorithm}")

        # Step 3: Compute verdict from Q-Score
        verdict = self._q_score_to_verdict(q_score)

        # Step 4: Compute unified confidence
        confidence = self._compute_unified_confidence(penalized)

        # Step 5: Build unified judgment
        judgment = UnifiedJudgment(
            q_score=q_score,
            verdict=verdict,
            confidence=confidence,
            justification=f"Engine unified {len(dog_inputs)} dog inputs via {self.algorithm}",
            precedent=context.get("precedent", "engine_judgment") if context else "engine_judgment",
            cost_usd=context.get("cost_usd", 0.01) if context else 0.01,
            dog_votes={di.dog_id: di.q_score for di in dog_inputs},
            algorithm=self.algorithm,
            layer0_violations=[],
        )

        # Step 6: Validate against Layer 0 axioms (immune check)
        violations = self.identity.validate_judgment(judgment)
        if violations:
            logger.warning(
                "Unified judgment violated axioms: %s. Q=%.1f, Conf=%.3f. "
                "This should not happen — Layer 0 constraint bug.",
                violations,
                judgment.q_score,
                judgment.confidence,
            )
            judgment.layer0_violations = violations

        return judgment
