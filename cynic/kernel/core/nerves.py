"""
CYNIC Nerve Endings — Domain-specific communication facades.

Reduces coupling to the global event bus by providing typed gateways for
each functional domain. Requires explicit instance_id for multi-tenancy.
"""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from cynic.kernel.core.judgment import Cell

from cynic.kernel.core.event_bus import CoreEvent, Event, get_bus
from cynic.kernel.core.events_schema import DecisionMadePayload, JudgmentCreatedPayload

logger = logging.getLogger("cynic.kernel.nerves")


class DomainNerve:
    """Base facade for a communication domain."""

    def __init__(self, bus_id: str, instance_id: str):
        self._bus_id = bus_id
        self._instance_id = instance_id

    @property
    def bus(self):
        return get_bus(self._bus_id, self._instance_id)


class CognitionNerves(DomainNerve):
    """Gateways for Brain/Mind signals."""

    def __init__(self, instance_id: str):
        super().__init__("CORE", instance_id)

    async def emit_judgment(self, payload: dict[str, Any]):
        """Signal a new judgment has been formed."""
        await self.bus.emit(
            Event.typed(
                CoreEvent.JUDGMENT_CREATED,
                JudgmentCreatedPayload.model_validate(payload),
                source="cognition_nerves",
            )
        )

    async def emit_decision(self, payload: DecisionMadePayload):
        """Signal a finalized decision."""
        await self.bus.emit(
            Event.typed(CoreEvent.DECISION_MADE, payload, source="cognition_nerves")
        )


class SomaticNerves(DomainNerve):
    """Gateways for Body/Metabolism signals."""

    def __init__(self, instance_id: str):
        super().__init__("AUTOMATION", instance_id)

    async def emit_anomaly(self, error_type: str, value: Any, source: str):
        """Signal internal pain or resource stress."""
        await self.bus.emit(
            Event.typed(
                CoreEvent.ANOMALY_DETECTED,
                payload={"type": error_type, "value": value, "source": source},
                source="somatic_nerves",
            )
        )


class PerceptionNerves(DomainNerve):
    """Gateways for Input/Sensing signals."""

    def __init__(self, instance_id: str):
        super().__init__("CORE", instance_id)

    async def ingest(self, cell: Cell):
        """Inject a new perception cell into the organism."""
        await self.bus.emit(
            Event.typed(
                CoreEvent.PERCEPTION_RECEIVED, payload=cell.model_dump(), source="perception_nerves"
            )
        )
