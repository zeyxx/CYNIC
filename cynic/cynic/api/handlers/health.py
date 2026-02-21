"""PHASE 7: Health/resource pressure handlers — disk/memory → LOD."""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from cynic.core.event_bus import Event, CoreEvent

from cynic.api.handlers.base import HandlerGroup, KernelServices

if TYPE_CHECKING:
    from cynic.core.storage.gc import StorageGarbageCollector
    from asyncpg import Pool

logger = logging.getLogger("cynic.api.handlers.health")


class HealthHandlers(HandlerGroup):
    """Resource pressure handling — disk/memory → LOD transition."""

    def __init__(
        self, svc: KernelServices, *, storage_gc, db_pool: Optional[Pool]
    ) -> None:
        self._svc = svc
        self._storage_gc = storage_gc
        self._db_pool = db_pool

    @property
    def name(self) -> str:
        return "health"

    def dependencies(self) -> frozenset[str]:
        return frozenset({
            "escore_tracker",
            "lod_controller",
            "storage_gc",
        })

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.DISK_PRESSURE, self._on_disk_pressure),
            (CoreEvent.MEMORY_PRESSURE, self._on_memory_pressure),
            (CoreEvent.DISK_CLEARED, self._on_disk_cleared),
            (CoreEvent.MEMORY_CLEARED, self._on_memory_cleared),
        ]

    async def _on_disk_pressure(self, event: Event) -> None:
        """DISK_PRESSURE → update health cache + assess LOD."""
        try:
            p = event.payload or {}
            disk_pct = float(p.get("disk_pct", 0.0))
            self._svc.health_cache["disk_pct"] = disk_pct
            await self._svc.assess_lod()
            logger.warning("DISK_PRESSURE: %.1f%% → LOD=%s", disk_pct, self._svc.lod_controller.current.name)
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_memory_pressure(self, event: Event) -> None:
        """MEMORY_PRESSURE → update health cache + assess LOD."""
        try:
            p = event.payload or {}
            memory_pct = float(p.get("memory_pct", 0.0))
            self._svc.health_cache["memory_pct"] = memory_pct
            await self._svc.assess_lod()
            logger.warning("MEMORY_PRESSURE: %.1f%% → LOD=%s", memory_pct, self._svc.lod_controller.current.name)
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_disk_cleared(self, event: Event) -> None:
        """DISK_CLEARED → reset health cache + reassess LOD."""
        try:
            p = event.payload or {}
            freed_mb = float(p.get("freed_mb", 0.0))
            self._svc.health_cache["disk_pct"] = 0.0
            await self._svc.assess_lod()
            logger.info("DISK_CLEARED: freed=%.1f MB → LOD=%s", freed_mb, self._svc.lod_controller.current.name)
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_memory_cleared(self, event: Event) -> None:
        """MEMORY_CLEARED → reset health cache + reassess LOD."""
        try:
            p = event.payload or {}
            freed_mb = float(p.get("freed_mb", 0.0))
            self._svc.health_cache["memory_pct"] = 0.0
            await self._svc.assess_lod()
            logger.info("MEMORY_CLEARED: freed=%.1f MB → LOD=%s", freed_mb, self._svc.lod_controller.current.name)
        except EventBusError:
            logger.debug("handler error", exc_info=True)
