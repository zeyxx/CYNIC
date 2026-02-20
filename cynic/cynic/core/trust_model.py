"""
TrustModel: Bidirectional confidence tracking for human↔CYNIC symbiosis.

CRITICAL: This is the foundation for all later phases.
- Measures human trust in CYNIC (accept_rate - reject_rate)
- Measures CYNIC utility (Q-Score, completion rate)
- Enables SYMBIOSIS axiom activation (requires both ≥ φ⁻² = 38.2%)
- Enables co-decision blending (blended_conf = human_conf × cynic_conf)

φ-bounded: All confidence values ∈ [0, MAX_CONFIDENCE=0.618]
"""

from dataclasses import dataclass, field
from typing import Optional, Tuple, List
from datetime import datetime, timedelta
import math

from cynic.core.phi import PHI_INV_2, MAX_CONFIDENCE, phi_bound_score


@dataclass
class TrustMetrics:
    """Snapshot of current trust state."""
    human_accept_count: int = 0
    human_reject_count: int = 0
    human_accept_rate: float = 0.0  # (accepts - rejects) / total
    human_confidence: float = 0.0  # φ-bounded [0, MAX_CONFIDENCE]

    machine_q_score_avg: float = 50.0  # Average Q-Score over last N decisions
    machine_completion_rate: float = 0.0  # Actions completed successfully
    machine_utility: float = 0.0  # Blended metric

    alignment_score: float = 0.0  # Min(human, machine) when both agree

    symbiosis_ready: bool = False  # Both ≥ 38.2%?
    last_updated: datetime = field(default_factory=datetime.now)


class TrustModel:
    """
    Tracks bidirectional trust for human-CYNIC co-decision making.

    Key insight: CYNIC doesn't just measure "how confident am I?"
    It measures "how much do I trust the human, and how much do they trust me?"
    """

    def __init__(self):
        self.metrics = TrustMetrics()
        self.accept_history: List[Tuple[datetime, bool, float]] = []  # (time, accepted, human_confidence)
        self.q_score_history: List[Tuple[datetime, float]] = []  # (time, q_score)
        self.alpha_ema = 0.1  # Exponential moving average smoothing

    async def update_from_action_feedback(
        self,
        action_id: str,
        accepted: bool,
        user_confidence: float,
        q_score: float,
        outcome_success: Optional[bool] = None,
    ) -> TrustMetrics:
        """
        Learning event: human accepted/rejected an action.

        Args:
            action_id: For logging/tracing
            accepted: True if human accepted, False if rejected
            user_confidence: How confident was the human in accepting?
            q_score: CYNIC's confidence in the action (from Judgment)
            outcome_success: (optional) Did the action succeed?

        Returns:
            Updated TrustMetrics
        """
        now = datetime.now()

        # 1. Update accept/reject counts
        if accepted:
            self.metrics.human_accept_count += 1
        else:
            self.metrics.human_reject_count += 1

        # 2. Recalculate human acceptance rate
        total = self.metrics.human_accept_count + self.metrics.human_reject_count
        self.metrics.human_accept_rate = (
            (self.metrics.human_accept_count - self.metrics.human_reject_count) / total
            if total > 0 else 0.0
        )

        # 3. Update human confidence via EMA
        # Acceptance increases confidence, rejection decreases it
        feedback_signal = 1.0 if accepted else -0.5
        user_conf_boosted = min(max(user_confidence * feedback_signal, 0.0), MAX_CONFIDENCE)

        new_conf = (1 - self.alpha_ema) * self.metrics.human_confidence + \
                   self.alpha_ema * user_conf_boosted
        self.metrics.human_confidence = min(max(new_conf, 0.0), MAX_CONFIDENCE)

        # 4. Track history
        self.accept_history.append((now, accepted, user_confidence))

        # 5. If outcome provided, update machine utility
        if outcome_success is not None:
            self.q_score_history.append((now, q_score))
            self._update_machine_utility()

        # 6. Check symbiosis readiness
        self._check_symbiosis_ready()

        self.metrics.last_updated = now
        return self.metrics

    def _update_machine_utility(self):
        """Recalculate machine utility from recent Q-scores."""
        if not self.q_score_history:
            return

        # Average of last 10 Q-scores
        recent = self.q_score_history[-10:]
        avg_q = sum(q for _, q in recent) / len(recent)

        # Completion rate: fraction of accepted actions that succeeded
        successful = sum(
            1 for _, accepted, _ in self.accept_history[-10:]
            if accepted
        )
        completion_rate = successful / len(self.accept_history[-10:]) if self.accept_history else 0.5

        # Blended utility: average Q × completion rate
        self.metrics.machine_utility = phi_bound_score(
            (avg_q / 100.0) * completion_rate  # Normalize Q to [0, 1]
        )

    def _check_symbiosis_ready(self):
        """Can SYMBIOSIS axiom activate?"""
        self.metrics.symbiosis_ready = (
            self.metrics.human_confidence >= PHI_INV_2 and
            self.metrics.machine_utility >= PHI_INV_2
        )

    async def blend_confidence(
        self,
        human_confidence: float,
        cynic_q_score: float,
    ) -> float:
        """
        Co-decision confidence: both must be confident.

        Blended = human_conf × cynic_conf
        If either unsure, result is very unsure (conservative).

        NOTE: We DON'T clamp inputs, we clamp the product.
        This preserves the uncertainty principle: if 0.8 × 0.8 > MAX_CONFIDENCE,
        the output is clamped, not the inputs.
        """
        # Both confidence values should be in [0, 1]
        human_norm = min(max(human_confidence, 0.0), 1.0)
        cynic_norm = min(max(cynic_q_score / 100.0, 0.0), 1.0)

        # Product may exceed MAX_CONFIDENCE, clamp it
        blended = human_norm * cynic_norm
        return min(max(blended, 0.0), MAX_CONFIDENCE)

    async def should_cynic_escalate(
        self,
        human_confidence: float,
        cynic_q_score: float,
        action_risk_level: str,  # "low", "medium", "high"
    ) -> Tuple[bool, str]:
        """
        Should CYNIC escalate instead of co-deciding?

        Returns:
            (should_escalate, reason)
        """
        blended = await self.blend_confidence(human_confidence, cynic_q_score)

        # Risk-adjusted thresholds
        risk_thresholds = {
            "low": 0.3,      # Confident enough? > 30%
            "medium": 0.5,   # > 50%
            "high": 0.7,     # > 70%
        }
        threshold = risk_thresholds.get(action_risk_level, 0.5)

        if blended < threshold:
            return True, f"Blended confidence {blended:.1%} < threshold {threshold:.1%} for {action_risk_level} risk"

        return False, ""

    def get_summary(self) -> str:
        """Human-readable summary of trust state."""
        return (
            f"Human trust: {self.metrics.human_confidence:.1%} "
            f"(accepts={self.metrics.human_accept_count}, rejects={self.metrics.human_reject_count}) | "
            f"Machine utility: {self.metrics.machine_utility:.1%} | "
            f"Symbiosis ready: {self.metrics.symbiosis_ready}"
        )

    # For testing/debugging
    async def reset(self):
        """Clear all history. For testing only."""
        self.metrics = TrustMetrics()
        self.accept_history = []
        self.q_score_history = []
