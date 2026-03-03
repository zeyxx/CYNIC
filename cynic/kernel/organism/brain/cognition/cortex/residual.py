"""
ResidualDetector - Gamma1 entropy sensor.

Measures the variance between Dog judgments.
High variance = High 'Unnameable' content = Emergence potential.
"""

from __future__ import annotations

import logging
import statistics
from typing import Optional, Any

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus, get_bus

logger = logging.getLogger("cynic.kernel.brain.cognition.residual")

class ResidualDetector:
    """
    Subscribes to JUDGMENT_CREATED events and calculates the variance
    between dog scores to detect emergence potential.
    """
    def __init__(self, bus: Optional[EventBus] = None):
        self._history: list[float] = []
        self._high_residual_count = 0
        self._threshold = 38.2  # PHI_INV_2 * 100
        self._bus = bus or get_bus("CORE")
        self._active = False

    def start(self) -> None:
        """Start listening for judgments."""
        if self._active:
            return
        self._bus.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment)
        self._active = True
        logger.info("ResidualDetector started")

    def stop(self) -> None:
        """Stop listening."""
        if not self._active:
            return
        try:
            self._bus.off(CoreEvent.JUDGMENT_CREATED, self._on_judgment)
        except Exception:
            pass
        self._active = False
        logger.info("ResidualDetector stopped")

    async def _on_judgment(self, event: Event) -> None:
        """Analyze the variance of a new judgment."""
        payload = event.dict_payload
        dog_votes = payload.get("dog_votes", {})
        
        if not dog_votes or len(dog_votes) < 2:
            return

        scores = list(dog_votes.values())
        residual = statistics.stdev(scores) if len(scores) > 1 else 0.0
        
        self._history.append(residual)
        if len(self._history) > 100:
            self._history.pop(0)

        if residual > self._threshold:
            self._high_residual_count += 1
            await self._bus.emit(Event.typed(
                CoreEvent.ANOMALY_DETECTED,
                {
                    "type": "high_residual",
                    "residual": round(residual, 2),
                    "judgment_id": payload.get("judgment_id")
                },
                source="residual_detector"
            ))

    def stats(self) -> dict[str, Any]:
        """Return detector metrics."""
        avg_res = sum(self._history) / len(self._history) if self._history else 0.0
        return {
            "current_residual": round(self._history[-1], 2) if self._history else 0.0,
            "avg_residual": round(avg_res, 2),
            "high_residual_count": self._high_residual_count,
            "threshold": self._threshold,
        }
