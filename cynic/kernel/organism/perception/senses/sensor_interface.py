"""
Sensor Interface — pluggable observ interface for Track C empirical testing.

This interface lets Track C create:
  - EmpiricalSensor: Replays recorded observations
  - SyntheticSensor: Generates deterministic test signals
  - RecordingSensor: Captures real sensor data for replay

Without modifying existing workers.
"""

import time
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any


@dataclass
class Observation:
    """Unified observation from any sensor.

    This is what sensors emit to the perception layer.
    Quality score allows Track C to inject synthetic high/low confidence signals.
    """

    sensor_id: str  # Unique sensor identifier (e.g., "disk", "solana", "mock_empirical")
    timestamp: float  # When observation was made
    data: dict[str, Any]  # Sensor-specific payload
    quality: float = 1.0  # 0.0–1.0 confidence in observation (1.0 = certain)

    def is_synthetic(self) -> bool:
        """Returns True if this observation was synthesized (not from real sensor)."""
        return self.sensor_id.startswith("synthetic_") or self.sensor_id.startswith("mock_")


class Sensor(ABC):
    """Abstract sensor — interface for all perception sources."""

    @property
    @abstractmethod
    def sensor_id(self) -> str:
        """Unique identifier (e.g., 'disk_monitor', 'solana_rpc', 'git_watcher')."""
        ...

    @abstractmethod
    async def startup(self) -> None:
        """Initialize sensor — called once at kernel startup.

        May perform:
          - Connection setup (RPC, database)
          - Warmup queries
          - State validation
        """
        ...

    @abstractmethod
    async def perceive(self) -> Observation | None:
        """Poll sensor for next observation.

        Returns:
            Observation if data available, None if nothing new (caller retries).

        This is called by the perceive worker in a loop.
        """
        ...

    @abstractmethod
    async def shutdown(self) -> None:
        """Cleanup — called once at kernel shutdown.

        May perform:
          - Connection cleanup
          - Graceful shutdown
        """
        ...

    async def check_available(self) -> bool:
        """Quick health check — is sensor reachable? (optional override)."""
        try:
            # Try one perceive call with short timeout
            import asyncio

            obs = await asyncio.wait_for(self.perceive(), timeout=1.0)
            return obs is not None or True  # availability depends on sensor type
        except (TimeoutError, Exception):
            return False

    def stats(self) -> dict[str, Any]:
        """Return sensor statistics for observability (optional override)."""
        return {
            "sensor_id": self.sensor_id,
            "timestamp": time.time(),
        }
