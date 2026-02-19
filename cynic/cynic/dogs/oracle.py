"""
CYNIC Oracle Dog — Tiferet (Beauty)

Non-LLM Dog. L3 REFLEX (<5ms Q-table lookup).
Technology: Thompson Sampling via existing QTable (no new dependencies)

Responsibilities:
  - Predict future verdict via Q-table policy consultation
  - Thompson Sampling when Q-table is data-rich
  - Exploit-based prediction when data exceeds PHI_INV_2 threshold
  - Signal uncertainty when Q-table is cold (few visits)

Why Oracle?
  Tiferet = Beauty in balance. Oracle sees the pattern behind the pattern.
  JANITOR scores present code. GUARDIAN detects present anomalies.
  ORACLE predicts what the judgment WILL be based on accumulated learning.

  "Based on 38 similar situations, WAG wins 72% of the time."
  This is temporal intelligence — not just now, but over time.

φ-integration:
  Cold Q-table (0 visits): q_score = 30.9 (GROWL — cautious default)
  Warm Q-table (21+ visits, F(8)): exploit policy, confidence rises
  Confidence = qtable.confidence(state_key) — visits-based, capped at 0.618

  VETO: impossible — Oracle predicts, never blocks

Design contract:
  - qtable injected at construction (optional, degrades gracefully to neutral)
  - Never creates QTable entries (read-only, no side effects)
  - Uses predict_q() and exploit() only — public API, no private access
"""
from __future__ import annotations

import logging
import time
from typing import Any, Dict, List, Optional

from cynic.core.phi import PHI_INV_2, MAX_Q_SCORE, MAX_CONFIDENCE, phi_bound_score
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.dogs.base import (
    AbstractDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.dogs.oracle")

# All possible verdicts (actions in Q-space)
VERDICTS: list[str] = ["BARK", "GROWL", "WAG", "HOWL"]

# Confidence when Q-table is cold (below PHI_INV_2 threshold)
COLD_CONFIDENCE: float = 0.200   # Minimal confidence when predicting from no data
WARM_CONFIDENCE_MIN: float = 0.300  # Floor when some data exists but not rich yet


class OracleDog(AbstractDog):
    """
    Oracle (Tiferet) — Thompson Sampling predictor from Q-Table history.

    Reads existing Q-Table state WITHOUT creating new entries.
    Three prediction modes:
      1. Cold (0 visits): neutral prediction → GROWL (cautious default)
      2. Warm (1-20 visits): exploit policy with low confidence
      3. Rich (21+ visits, F(8)): exploit with rising confidence

    Oracle's vote represents EXPECTED FUTURE QUALITY based on past patterns.
    This is different from GUARDIAN (anomaly) or JANITOR (current smells).
    """

    def __init__(self, qtable=None) -> None:
        """
        qtable: Optional QTable instance for predictions.
                If None, Oracle returns neutral (0.5 quality) predictions.
        """
        super().__init__(DogId.ORACLE)
        self._qtable = qtable
        self._cold_votes: int = 0    # Times Oracle had no data
        self._warm_votes: int = 0    # Times Oracle used learned data
        self._cells_analyzed: int = 0

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.ORACLE,
            sefirot="Tiferet — Beauty",
            consciousness_min=ConsciousnessLevel.REFLEX,
            uses_llm=False,
            supported_realities={"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"},
            supported_analyses={"PERCEIVE", "JUDGE", "DECIDE", "LEARN"},
            technology="QTable Thompson Sampling (read-only)",
            max_concurrent=13,  # F(7) — pure reads, highly parallelizable
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Predict verdict quality via Q-table consultation.

        Reads Q-table (no writes) to predict: "Given past experience with
        this type of situation, what quality outcome is most likely?"
        """
        start = time.perf_counter()
        state_key = cell.state_key()
        self._cells_analyzed += 1

        if self._qtable is None:
            return self._neutral_judgment(cell, start, reason="no-qtable-wired")

        # Read confidence from visits (public API, read-only)
        confidence = self._qtable.confidence(state_key)
        total_visits = self._estimate_visits(state_key)

        if total_visits == 0:
            # Cold start — no data for this state
            self._cold_votes += 1
            return self._neutral_judgment(
                cell, start,
                reason=f"cold-start:{state_key}",
                confidence=COLD_CONFIDENCE,
            )

        # Get best predicted action from exploit policy
        best_action = self._qtable.exploit(state_key)
        pred_q = self._qtable.predict_q(state_key, best_action)

        # Get all Q-values for evidence (read-only)
        all_q = {
            action: round(self._qtable.predict_q(state_key, action) * MAX_Q_SCORE, 1)
            for action in VERDICTS
        }

        # Oracle q_score = predicted quality × MAX_Q_SCORE
        q_score = phi_bound_score(pred_q * MAX_Q_SCORE)

        # Confidence: QTable visits-based, but respect warm/cold boundary
        adj_confidence = max(
            WARM_CONFIDENCE_MIN if total_visits > 0 else COLD_CONFIDENCE,
            min(confidence, MAX_CONFIDENCE),
        )

        self._warm_votes += 1
        reasoning = (
            f"*sniff* Predicts {best_action} (Q={pred_q:.2%}) "
            f"from {total_visits} past judgments | "
            f"conf={adj_confidence:.0%}"
        )

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=q_score,
            confidence=adj_confidence,
            reasoning=reasoning,
            evidence={
                "state_key": state_key,
                "predicted_action": best_action,
                "predicted_q": round(pred_q, 4),
                "total_visits": total_visits,
                "all_q_scores": all_q,
            },
            latency_ms=latency,
            veto=False,  # Oracle never VETOs — it predicts, not blocks
        )
        self.record_judgment(judgment)
        return judgment

    # ── Helpers ────────────────────────────────────────────────────────────

    def _estimate_visits(self, state_key: str) -> int:
        """
        Estimate total visits for a state from the Q-table.

        Uses confidence proxy: confidence = visits / F(8) → visits ≈ confidence × F(8)
        Direct access avoided to prevent side effects.
        """
        if self._qtable is None:
            return 0
        # confidence = visits / F(8) where F(8) = 21
        conf = self._qtable.confidence(state_key)
        # Invert: visits ≈ conf × 21  (approximate, avoids private _table access)
        from cynic.core.phi import fibonacci
        return int(conf * fibonacci(8))

    def _neutral_judgment(
        self,
        cell: Cell,
        start: float,
        reason: str = "no-data",
        confidence: float = COLD_CONFIDENCE,
    ) -> DogJudgment:
        """
        Neutral judgment when Oracle has no data to predict from.

        neutral q_score = 0.5 × MAX_Q_SCORE = 30.9 → GROWL territory (cautious).
        This is correct: Oracle should not pretend to know when it doesn't.
        """
        neutral_q = 0.5 * MAX_Q_SCORE  # 30.9 → GROWL
        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=phi_bound_score(neutral_q),
            confidence=confidence,
            reasoning=f"*head tilt* No prediction data ({reason}) — defaulting to neutral GROWL",
            evidence={"reason": reason, "state_key": cell.state_key()},
            latency_ms=latency,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    async def health_check(self) -> DogHealth:
        total = self._cold_votes + self._warm_votes
        warm_ratio = self._warm_votes / max(total, 1)
        status = (
            HealthStatus.HEALTHY if warm_ratio >= 0.5 else
            HealthStatus.DEGRADED if total > 0 else
            HealthStatus.UNKNOWN
        )
        return DogHealth(
            dog_id=self.dog_id,
            status=status,
            latency_p50_ms=self.avg_latency_ms,
            details=(
                f"Cells: {self._cells_analyzed}, "
                f"Warm votes: {self._warm_votes}, "
                f"Cold votes: {self._cold_votes}, "
                f"Warm ratio: {warm_ratio:.0%}"
            ),
        )
