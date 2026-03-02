"""
ResidualDetector â€” Î³1 entropy sensor.

Measures the variance between Dog judgments.
High variance = High 'Unnameable' content = Emergence potential.

Formula:
  residual = standard_deviation(q_scores) / MAX_Q_SCORE
"""

from __future__ import annotations

import logging
import statistics
from typing import Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.formulas import RESIDUAL_STABLE_HIGH_N
from cynic.kernel.core.judgment import Judgment
from cynic.kernel.core.phi import PHI_INV_2

logger = logging.getLogger("cynic.kernel.brain.cognition.residual")


class ResidualDetector:
    """
    Senses entropy in the judgment collective.
    Trigger for L4 META cycles and self-improvement proposals.
    """

    def __init__(self, bus: EventBus):
        self._history: list[float] = []
        self._high_residual_count = 0
        self._threshold = PHI_INV_2 * 100  # ~38.2
        self._bus = bus

    def start(self):
        """Subscribe to judgment events."""
        self._bus.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment)
        logger.info("ResidualDetector started â€” listening for JUDGMENT_CREATED")

    def observe(self, judgment: Judgment) -> float:
        """Synchronous observation of judgment variance."""
        votes = list(judgment.dog_votes.values())
        if len(votes) < 2:
            return 0.0

        res = statistics.stdev(votes)
        self._history.append(res)
        if len(self._history) > 144:  # F(12)
            self._history.pop(0)

        if res > self._threshold:
            self._high_residual_count += 1
        else:
            self._high_residual_count = 0

        return res

    async def _on_judgment(self, event: Event) -> None:
        """Async handler for judgment events."""
        try:
            # We use event.payload directly or use model_validate if needed
            data = event.dict_payload
            votes = data.get("dog_votes", {})
            if not votes or len(votes) < 2:
                return

            res = self.observe_dict(votes)

            # Update high residual count (same logic as observe())
            if res > self._threshold:
                self._high_residual_count += 1
            else:
                self._high_residual_count = 0

            if res > self._threshold:
                await self._signal_high_entropy(res)
        except Exception as e:
            logger.error("ResidualDetector failed to process event: %s", e)

    def observe_dict(self, votes: dict[str, float]) -> float:
        v_list = list(votes.values())
        res = statistics.stdev(v_list)
        self._history.append(res)
        return res

    async def _signal_high_entropy(self, value: float) -> None:
        """Emit alert if entropy stays high."""
        if self._high_residual_count >= RESIDUAL_STABLE_HIGH_N:
            logger.warning("HIGH RESIDUAL DETECTED: %.2f (entropy stable)", value)
            await self._bus.emit(
                Event.typed(
                    CoreEvent.RESIDUAL_HIGH,
                    payload={"residual": value, "count": self._high_residual_count},
                    source="residual_detector",
                )
            )

    def stats(self) -> dict:
        avg_res = sum(self._history) / len(self._history) if self._history else 0.0
        return {
            "current_residual": round(self._history[-1], 2) if self._history else 0.0,
            "avg_residual": round(avg_res, 2),
            "history_size": len(self._history),
            "threshold": self._threshold,
        }
