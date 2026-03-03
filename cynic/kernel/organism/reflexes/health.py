"""PHASE 7: Health/resource pressure handlers " disk/memory ' LOD."""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any, Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus, EventBusError
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import CognitionServices

if TYPE_CHECKING:
    pass  # Or relevant storage interface

logger = logging.getLogger("cynic.kernel.organism.reflexes.health")


class HealthHandlers(HandlerGroup):
    """Health/resource pressure handlers " disk/memory ' LOD."""

    def __init__(
        self,
        cognition: CognitionServices,
        *,
        storage_gc,
        db_pool: Any | None,
        bus: Optional[EventBus] = None,
    ) -> None:
        super().__init__(bus=bus)
        self._cognition = cognition
        self._storage_gc = storage_gc
        self._db_pool = db_pool

    @property
    def name(self) -> str:
        return "health"

    def dependencies(self) -> frozenset[str]:
        return frozenset(
            {
                "escore_tracker",
                "lod_controller",
                "storage_gc",
            }
        )

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.DISK_PRESSURE, self._on_disk_pressure),
            (CoreEvent.MEMORY_PRESSURE, self._on_memory_pressure),
            (CoreEvent.DISK_CLEARED, self._on_disk_cleared),
            (CoreEvent.MEMORY_CLEARED, self._on_memory_cleared),
        ]

    async def _on_disk_pressure(self, event: Event) -> None:
        """DISK_PRESSURE ' update health cache + assess LOD."""
        try:
            p = event.dict_payload or {}
            disk_pct = float(p.get("disk_pct", 0.0))
            self._cognition.health_cache["disk_pct"] = disk_pct
            await self._cognition.assess_lod()
            logger.warning(
                "DISK_PRESSURE: %.1f%% ' LOD=%s",
                disk_pct,
                self._cognition.lod_controller.current.name,
            )
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_memory_pressure(self, event: Event) -> None:
        """MEMORY_PRESSURE ' update health cache + prune Q-Table + assess LOD."""
        try:
            p = event.dict_payload or {}
            memory_pct = float(p.get("memory_pct", 0.0))
            self._cognition.health_cache["memory_pct"] = memory_pct
            await self._cognition.assess_lod()

            # Active Defense against Infinity (BURN Axiom)
            if (
                memory_pct > 85.0
                and hasattr(self._cognition, "qtable")
                and self._cognition.qtable
            ):
                # Keep top 2000 entries, burn the rest
                pruned_count = self._cognition.qtable.prune(max_entries=2000)
                if pruned_count > 0:
                    logger.warning(
                        "MEMORY_PRESSURE (%.1f%%) ' BURN AXIOM ACTIVE: Pruned %d low-value Q-Entries to survive.",
                        memory_pct,
                        pruned_count,
                    )
            else:
                logger.warning(
                    "MEMORY_PRESSURE: %.1f%% ' LOD=%s",
                    memory_pct,
                    self._cognition.lod_controller.current.name,
                )

        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_disk_cleared(self, event: Event) -> None:
        """DISK_CLEARED ' reset health cache + reassess LOD."""
        try:
            p = event.dict_payload or {}
            freed_mb = float(p.get("freed_mb", 0.0))
            self._cognition.health_cache["disk_pct"] = 0.0
            await self._cognition.assess_lod()
            logger.info(
                "DISK_CLEARED: freed=%.1f MB ' LOD=%s",
                freed_mb,
                self._cognition.lod_controller.current.name,
            )
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_memory_cleared(self, event: Event) -> None:
        """MEMORY_CLEARED ' reset health cache + reassess LOD."""
        try:
            p = event.dict_payload or {}
            freed_mb = float(p.get("freed_mb", 0.0))
            self._cognition.health_cache["memory_pct"] = 0.0
            await self._cognition.assess_lod()
            logger.info(
                "MEMORY_CLEARED: freed=%.1f MB ' LOD=%s",
                freed_mb,
                self._cognition.lod_controller.current.name,
            )
        except EventBusError:
            logger.debug("handler error", exc_info=True)
