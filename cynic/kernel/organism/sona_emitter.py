"""
SONA Emitter — Organism Self-Assessment Loop (Component 10/11)

SONA = φ-scaled organism self-assessment
Frequency: F(9) = 34 minutes = 2,040 seconds
Level: META (consciousness level 4)
"""

from __future__ import annotations

import asyncio
import logging
import time
from typing import Any, Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.events_schema import SonaTickPayload
from cynic.kernel.core.phi import fibonacci

logger = logging.getLogger("cynic.kernel.organism.sona_emitter")

# F(9) = 34
SONA_INTERVAL_S: float = float(fibonacci(9)) 


class SonaEmitter:
    """
    Periodic self-assessment heartbeat.
    Triggers internal reflection and cross-domain synchronization.
    """

    def __init__(self, bus: EventBus, db_pool: Any = None, instance_id: str = "") -> None:
        self._instance_id = instance_id
        self._bus = bus
        self._running = False
        self._task: Optional[asyncio.Task] = None
        self._tick_count = 0
        self._start_time = time.time()
        
        self._qtable: Any | None = None
        self._orchestrator: Any | None = None
        self._escore_tracker: Any | None = None

    def set_qtable(self, qtable: Any) -> None:
        """Inject QTable for telemetry. Called by organism.py after construction."""
        self._qtable = qtable

    def set_orchestrator(self, orchestrator: Any) -> None:
        """Inject JudgeOrchestrator for judgment count. Called by organism.py."""
        self._orchestrator = orchestrator

    def set_escore_tracker(self, escore_tracker: Any) -> None:
        """Inject EScoreTracker for reputation broadcast."""
        self._escore_tracker = escore_tracker

    async def start(self) -> None:
        """Launch the background loop."""
        if self._task is not None:
            return

        self._running = True
        self._task = asyncio.create_task(self._loop())
        logger.info("SonaEmitter started (interval=%.1fs)", SONA_INTERVAL_S)

    async def stop(self) -> None:
        """Stop the background loop."""
        self._running = False
        if self._task:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass
            self._task = None
        logger.info("SonaEmitter stopped (emitted %d ticks)", self._tick_count)

    async def _loop(self) -> None:
        """Main SONA cycle."""
        while self._running:
            try:
                # 1. EMIT HEARTBEAT
                await self._emit_sona_tick()
                
                # 2. BROADCAST REPUTATION
                if self._escore_tracker:
                    await self._escore_tracker.broadcast_reputation()

                self._tick_count += 1
                await asyncio.sleep(SONA_INTERVAL_S)
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error("SonaEmitter cycle error: %s", e)
                await asyncio.sleep(5)

    async def _emit_sona_tick(self) -> None:
        """Emit detailed system-wide state snapshot."""
        stats = self._get_sona_stats()
        await self._bus.emit(
            Event.typed(
                CoreEvent.SONA_TICK,
                SonaTickPayload(**stats),
                source="sona_emitter"
            )
        )

    def _get_sona_stats(self) -> dict[str, Any]:
        """Collect current metrics for self-assessment."""
        return {
            "timestamp": time.time(),
            "tick_count": self._tick_count,
            "uptime_s": time.time() - self._start_time,
            "interval_s": SONA_INTERVAL_S,
            "next_tick_in_s": SONA_INTERVAL_S - (time.time() - self._start_time) % SONA_INTERVAL_S,
        }
