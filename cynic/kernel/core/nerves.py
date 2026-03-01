"""
CYNIC Nerve Endings — Domain-specific communication facades.

Reduces coupling to the global event bus by providing typed gateways for 
each functional domain.

φ-Law: VERIFY — only valid, domain-typed events pass through these nerves.
"""
from __future__ import annotations

import logging
from typing import Any

from cynic.kernel.core.event_bus import get_bus, Event, CoreEvent
from cynic.kernel.core.events_schema import (
    JudgmentCreatedPayload,
    DecisionMadePayload
)

logger = logging.getLogger("cynic.kernel.nerves")

class DomainNerve:
    """Base facade for a communication domain."""
    def __init__(self, bus_id: str):
        self._bus_id = bus_id

    @property
    def bus(self):
        return get_bus(self._bus_id)

class CognitionNerves(DomainNerve):
    """Gateways for Brain/Mind signals."""
    def __init__(self):
        super().__init__("CORE")

    async def emit_judgment(self, payload: dict[str, Any]):
        """Signal a new judgment has been formed."""
        await self.bus.emit(Event.typed(
            CoreEvent.JUDGMENT_CREATED,
            JudgmentCreatedPayload.model_validate(payload),
            source="cognition_nerves"
        ))

    async def emit_decision(self, payload: DecisionMadePayload):
        """Signal a finalized decision."""
        await self.bus.emit(Event.typed(
            CoreEvent.DECISION_MADE,
            payload,
            source="cognition_nerves"
        ))

class SomaticNerves(DomainNerve):
    """Gateways for Body/Metabolism signals."""
    def __init__(self):
        super().__init__("AUTOMATION")

    async def emit_anomaly(self, error_type: str, value: Any, source: str):
        """Signal internal pain or resource stress."""
        await self.bus.emit(Event.typed(
            CoreEvent.ANOMALY_DETECTED,
            payload={"type": error_type, "value": value, "source": source},
            source="somatic_nerves"
        ))

    async def emit_resource_update(self, resource_type: str, usage_pct: float):
        """Standard signal for hardware usage."""
        await self.bus.emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            payload={"reality": "SOMATIC", "type": resource_type, "value": usage_pct},
            source="somatic_nerves"
        ))

    async def emit_memory_pressure(self, pressure: str, used_pct: float, free_gb: float):
        """Signal RAM stress."""
        from cynic.kernel.core.events_schema import MemoryPressurePayload
        await self.bus.emit(Event.typed(
            CoreEvent.MEMORY_PRESSURE,
            MemoryPressurePayload(
                pressure=pressure,
                used_pct=used_pct,
                memory_pct=used_pct,
                free_gb=free_gb
            ),
            source="somatic_nerves"
        ))

    async def emit_memory_cleared(self, used_pct: float, free_gb: float):
        """Signal RAM recovery."""
        from cynic.kernel.core.events_schema import MemoryClearedPayload
        await self.bus.emit(Event.typed(
            CoreEvent.MEMORY_CLEARED,
            MemoryClearedPayload(memory_pct=used_pct, free_gb=free_gb),
            source="somatic_nerves"
        ))

    async def emit_disk_pressure(self, pressure: str, used_pct: float, free_gb: float):
        """Signal Disk stress."""
        from cynic.kernel.core.events_schema import DiskPressurePayload
        await self.bus.emit(Event.typed(
            CoreEvent.DISK_PRESSURE,
            DiskPressurePayload(
                pressure=pressure,
                used_pct=used_pct,
                disk_pct=used_pct,
                free_gb=free_gb
            ),
            source="somatic_nerves"
        ))

    async def emit_disk_cleared(self, used_pct: float, free_gb: float):
        """Signal Disk recovery."""
        from cynic.kernel.core.events_schema import DiskClearedPayload
        await self.bus.emit(Event.typed(
            CoreEvent.DISK_CLEARED,
            DiskClearedPayload(disk_pct=used_pct, free_gb=free_gb),
            source="somatic_nerves"
        ))

class PerceptionNerves(DomainNerve):
    """Gateways for Input/Sensing signals."""
    def __init__(self):
        super().__init__("CORE")

    async def ingest(self, cell: Cell):
        """Inject a new perception cell into the organism."""
        await self.bus.emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            payload=cell.model_dump(),
            source="perception_nerves"
        ))

# --- SINGLETONS ---
COGNITION = CognitionNerves()
SOMATIC = SomaticNerves()
PERCEPTION = PerceptionNerves()
