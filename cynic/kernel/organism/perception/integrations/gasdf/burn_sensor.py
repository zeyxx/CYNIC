"""GASdf burn statistics sensor for LNSP integration."""

from __future__ import annotations

import dataclasses
from typing import Any

from cynic.kernel.protocol.lnsp.layer1 import Sensor
from cynic.kernel.protocol.lnsp.messages import create_raw_observation
from cynic.kernel.protocol.lnsp.types import LNSPMessage, ObservationType

from .client import GASdfClient
from .types import GASdfError


class GASdfBurnSensor(Sensor):
    """Sensor that polls GASdf burn statistics and emits LNSP observations.

    Observation type: ECOSYSTEM_EVENT (GASdf stats are external ecosystem data).

    This sensor periodically polls the GASdf /v1/stats endpoint to track
    cumulative burn statistics, which are emitted as LNSP Layer 1 observations
    for aggregation and judgment in the governance pipeline.

    Attributes:
        client: GASdfClient instance for API communication
    """

    def __init__(
        self,
        sensor_id: str,
        client: GASdfClient,
        instance_id: str = "instance:local",
    ) -> None:
        """Initialize GASdfBurnSensor.

        Args:
            sensor_id: Unique identifier for this sensor
            client: GASdfClient instance for API calls
            instance_id: Organism instance ID (default: "instance:local")
        """
        super().__init__(sensor_id, instance_id)
        self.client = client
        self.last_observed_stats: dict[str, Any] = {}

    async def observe(self) -> LNSPMessage | None:
        """Poll GASdf stats and emit observation if changed.

        Monitors community treasury health through fee burn statistics,
        which indicate governance quality and community engagement.

        Returns:
            LNSPMessage with ECOSYSTEM_EVENT observation type,
            or None if unable to fetch stats or no change detected.

        Raises:
            No exceptions raised - errors are logged but not propagated.
        """
        try:
            # Fetch current stats from GASdf
            stats = await self.client.get_stats()
            stats_dict = dataclasses.asdict(stats)

            # Only emit if stats have changed
            if stats_dict == self.last_observed_stats:
                return None

            self.last_observed_stats = stats_dict

            # Calculate treasury health metrics
            treasury_health = self._assess_treasury_health(stats)
            burn_rate = self._calculate_burn_rate(stats)

            # Create observation with enriched data
            data: dict[str, Any] = {
                "data": stats_dict,
                "source": "gasdf",
                "treasury_health": treasury_health,
                "burn_rate": burn_rate,
                "observations": {
                    "total_burned": stats.total_burned,
                    "total_transactions": stats.total_transactions,
                    "average_fee": (
                        stats.total_burned // stats.total_transactions
                        if stats.total_transactions > 0
                        else 0
                    ),
                    "treasury_growing": stats.total_burned > 0,
                },
            }

            return create_raw_observation(
                observation_type=ObservationType.ECOSYSTEM_EVENT,
                data=data,
                source=self.sensor_id,
                instance_id=self.instance_id,
            )

        except GASdfError:
            # Silently skip observation on API errors
            # (network transients, service downtime, etc.)
            return None

    def _assess_treasury_health(self, stats: Any) -> str:
        """Assess community treasury health based on burn statistics.

        Args:
            stats: GASdfStats object with burn data

        Returns:
            Health assessment: excellent/good/fair/poor/unknown
        """
        if not hasattr(stats, "total_burned"):
            return "unknown"

        total_burned = stats.total_burned
        if total_burned > 10_000_000:  # >10M burned
            return "excellent"
        elif total_burned > 1_000_000:  # >1M burned
            return "good"
        elif total_burned > 100_000:  # >100K burned
            return "fair"
        else:
            return "poor"

    def _calculate_burn_rate(self, stats: Any) -> float:
        """Calculate burn rate (tokens burned per transaction).

        Args:
            stats: GASdfStats object with burn data

        Returns:
            Average burn per transaction, or 0 if no transactions
        """
        if not hasattr(stats, "total_transactions"):
            return 0.0

        if stats.total_transactions == 0:
            return 0.0

        return float(stats.total_burned) / float(stats.total_transactions)
