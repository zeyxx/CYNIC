"""
EmpiricalSensor â€” Replays fixed observation sequences for Track C testing.

Used to inject deterministic test signals without requiring real sensors
(disk, git, solana, etc.). Observations are marked as synthetic for
tracking/filtering.
"""

from __future__ import annotations

import logging

from cynic.kernel.organism.perception.senses.sensor_interface import Observation, Sensor

logger = logging.getLogger("cynic.kernel.organism.perception.senses.empirical_sensor")


class EmpiricalSensor(Sensor):
    """
    Replays a fixed sequence of Observations.

    Used by Track C to inject synthetic observations with controlled
    quality scores (0.0â€“1.0 confidence) to test perception â†’ judgment
    pipelines.
    """

    def __init__(self, observations: list[Observation], name: str = "empirical"):
        """
        Initialize with observation queue.

        Args:
            observations: List of Observation objects to replay
            name: Sensor name for identification (default: "empirical")
        """
        self._observations = list(observations)
        self._index = 0
        self._name = name

    @property
    def sensor_id(self) -> str:
        """Synthetic sensor identifier."""
        return f"synthetic_{self._name}"

    async def startup(self) -> None:
        """No-op â€” no I/O required."""
        self._index = 0
        logger.debug(
            f"EmpiricalSensor({self._name}) started: {len(self._observations)} observations queued"
        )

    async def perceive(self) -> Observation | None:
        """
        Pop next observation from queue.

        Returns:
            Next Observation if available, None when queue exhausted.
        """
        if self._index >= len(self._observations):
            return None

        obs = self._observations[self._index]
        self._index += 1
        return obs

    async def shutdown(self) -> None:
        """No-op â€” no resources to clean up."""
        logger.debug(
            f"EmpiricalSensor({self._name}) shutdown: {self._index}/{len(self._observations)} observations replayed"
        )

    def stats(self) -> dict:
        """Return sensor statistics."""
        return {
            "sensor_id": self.sensor_id,
            "observations_total": len(self._observations),
            "observations_replayed": self._index,
            "observations_remaining": max(0, len(self._observations) - self._index),
        }
