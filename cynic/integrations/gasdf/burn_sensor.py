"""GASdf burn statistics sensor for LNSP integration."""
from __future__ import annotations

import dataclasses
from typing import Any

from cynic.protocol.lnsp.layer1 import Sensor
from cynic.protocol.lnsp.messages import create_raw_observation
from cynic.protocol.lnsp.types import LNSPMessage, ObservationType

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

        Returns:
            LNSPMessage with ECOSYSTEM_EVENT observation type,
            or None if unable to fetch stats.

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

            # Create observation with stats data
            data: dict[str, Any] = {
                "data": stats_dict,
                "source": "gasdf",
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
