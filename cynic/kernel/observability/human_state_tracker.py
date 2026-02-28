"""Track human state: energy, focus, intentions, feedback."""
from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass
from typing import Optional

logger = logging.getLogger(__name__)


@dataclass
class HumanState:
    """Current human state snapshot."""

    energy: float  # [0, 10]
    focus: float  # [0, 10]
    intentions: list[str]
    values: list[str]
    feedback: list[str]
    growth_areas: dict[str, float]
    timestamp: float


class HumanStateTracker:
    """Tracks human energy, focus, intentions, and feedback."""

    def __init__(self):
        """Initialize tracker with default state."""
        self._energy = 5.0
        self._focus = 5.0
        self._intentions = []
        self._values = []
        self._feedback = []
        self._growth_areas = {}
        self._last_activity_time = time.time()

    async def get_state(self) -> HumanState:
        """Get current human state snapshot."""
        return HumanState(
            energy=self._energy,
            focus=self._focus,
            intentions=self._intentions.copy(),
            values=self._values.copy(),
            feedback=self._feedback.copy(),
            growth_areas=self._growth_areas.copy(),
            timestamp=time.time(),
        )

    async def get_snapshot(self) -> HumanState:
        """Alias for get_state to match SymbioticStateManager interface."""
        return await self.get_state()

    async def report_feedback(
        self,
        feedback_type: str,
        message: str,
        confidence: float,
    ) -> None:
        """Report human feedback about CYNIC."""
        feedback_entry = f"[{feedback_type}] {message} (conf: {confidence:.2f})"
        self._feedback.append(feedback_entry)
        self._last_activity_time = time.time()
        logger.info(f"Human feedback: {feedback_entry}")

    async def set_energy(self, level: float) -> None:
        """Set human energy level [0, 10]."""
        self._energy = max(0, min(10, level))
        self._last_activity_time = time.time()

    async def set_focus(self, level: float) -> None:
        """Set human focus level [0, 10]."""
        self._focus = max(0, min(10, level))
        self._last_activity_time = time.time()

    async def set_intentions(self, intentions: list[str]) -> None:
        """Set human's current intentions."""
        self._intentions = intentions
        self._last_activity_time = time.time()

    async def set_values(self, values: list[str]) -> None:
        """Set human's core values."""
        self._values = values
        self._last_activity_time = time.time()
