"""
Internal Sensor — The Proprioception of CYNIC.

Converts system anomalies and hardware stress into internal perceptions.
This is what allows the organism to 'feel' its own state.
"""

from __future__ import annotations

import logging
import time

from typing import Optional
from cynic.kernel.core.event_bus import CoreEvent, Event, get_core_bus, EventBus
from cynic.kernel.core.judgment import Cell

logger = logging.getLogger("cynic.senses.internal")


class InternalSensor:
    """
    Listens to system-level anomalies and re-injects them as perceptions.
    """

    def __init__(self, bus: EventBus):
        self._bus = bus

    def start(self, bus: Optional[EventBus] = None):
        """Subscribe to anomalies."""
        target_bus = bus or self._bus
        target_bus.on(CoreEvent.ANOMALY_DETECTED, self._on_anomaly)
        logger.info("InternalSensor: Proprioception active — listening for anomalies")

    async def _on_anomaly(self, event: Event) -> None:
        """Translate technical anomaly to conscious perception."""
        data = event.dict_payload
        anomaly_type = data.get("type", "UNKNOWN_ERROR")
        value = data.get("value", "N/A")

        # Create a perception cell for the internal state
        content = f"INTERNAL_SIGNAL: {anomaly_type} detected. Level: {value}. System health may be compromised."

        cell = Cell(
            cell_id=f"internal-{int(time.time())}",
            reality="INTERNAL",
            analysis="JUDGE",
            content=content,
            context=f"Self-observation signal from {event.source}",
            novelty=0.8,
            risk=0.9,
        )

        from cynic.kernel.core.events_schema import PerceptionReceivedPayload
        await self._bus.emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            PerceptionReceivedPayload(
                reality="INTERNAL",
                analysis="ANOMALY",
                data=f"Organism feels stress: {anomaly_type}={value}",
                risk=0.9,
                run_judgment=True
            ),
            source="internal_sensor"
        ))

        logger.info("InternalSensor: Anomaly translated to INTERNAL perception.")
