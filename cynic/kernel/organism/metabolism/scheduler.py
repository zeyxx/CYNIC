"""
ConsciousnessRhythm " 2 biological scheduler.

Manages 4-tier concurrent consciousness loops as N asyncio workers.
"""

from __future__ import annotations

import asyncio
import logging
import time
from typing import Any, Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus

from cynic.kernel.core.phi import fibonacci
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.organism.metabolism.throttler import MetabolicThrottler

logger = logging.getLogger("cynic.kernel.organism.metabolism.scheduler")

_QUEUE_CAPACITY = 100


class ConsciousnessRhythm:
    """2 biological scheduler " manages concurrent consciousness loops."""

    def __init__(
        self,
        orchestrator: Any,
        body: Optional[Any] = None,
        bus: Optional[EventBus] = None,
        consciousness: Any | None = None,
    ) -> None:
        self._orchestrator = orchestrator
        self.body = body
        self._consciousness = consciousness  # Injected
        self._throttler = MetabolicThrottler(body=self.body)
        self._queues: dict[ConsciousnessLevel, asyncio.Queue] = {
            lvl: asyncio.Queue(maxsize=_QUEUE_CAPACITY) for lvl in ConsciousnessLevel
        }

        self._tasks: list[asyncio.Task] = []
        self._running = False

        # Use provided bus or fallback to orchestrator bus if available
        self.bus = bus or getattr(orchestrator, "bus", None)
        if self.bus is None:
            raise RuntimeError("ConsciousnessRhythm initialized without a bus")

    def start(self) -> None:
        """Launch all tier workers."""
        if self._running:
            return
        self._running = True

        # Initial pulse for somatic awareness (TUI visibility)
        if self.body:
            asyncio.create_task(self.body.pulse())

        # Launch workers for each level
        # REFLEX: 5, MICRO: 3, MACRO: 2, META: 1
        worker_counts = {
            ConsciousnessLevel.REFLEX: 5,
            ConsciousnessLevel.MICRO: 3,
            ConsciousnessLevel.MACRO: 2,
            ConsciousnessLevel.META: 1,
        }

        for level, count in worker_counts.items():
            for i in range(count):
                self._tasks.append(asyncio.create_task(self._worker_loop(level, i)))

        logger.info("ConsciousnessRhythm: Started %d workers.", len(self._tasks))

    async def stop(self) -> None:
        """Shutdown all workers."""
        self._running = False
        for t in self._tasks:
            t.cancel()
        await asyncio.gather(*self._tasks, return_exceptions=True)
        self._tasks.clear()
        logger.info("ConsciousnessRhythm: All workers stopped.")

    async def submit(
        self, cell: Any, level: Optional[ConsciousnessLevel] = None
    ) -> None:
        """Submit a cell for processing."""
        target_level = level or ConsciousnessLevel.REFLEX
        await self._queues[target_level].put(cell)

    async def _worker_loop(self, level: ConsciousnessLevel, worker_id: int):
        """Internal loop for a specific consciousness tier."""
        while self._running:
            try:
                # 0. SOMATIC THROTTLING: Breathe before thinking
                await self._throttler.wait_for_breath(level)

                # 1. Wait for work or metabolic pulse
                # (Meta level handles the breathing pulse)
                if level == ConsciousnessLevel.META:
                    await self._meta_pulse()
                    await asyncio.sleep(float(fibonacci(9)))  # ~34s default
                    continue

                cell = await asyncio.wait_for(self._queues[level].get(), timeout=1.0)
                if cell:
                    await self._orchestrator.run(cell, level=level)
                    self._queues[level].task_done()

            except asyncio.TimeoutError:
                continue
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Worker {level.name}:{worker_id} error: {e}")
                await asyncio.sleep(1)

    async def _meta_pulse(self):
        """Respiration pulse " check body, axioms, and health."""
        if self.body:
            await self.body.pulse()

        # Feedback to State: Record meta cycle
        if hasattr(self._orchestrator, "record_cycle"):
            self._orchestrator.record_cycle(ConsciousnessLevel.META)

        # Meta-analysis triggers
        await self.bus.emit(
            Event.typed(
                CoreEvent.META_CYCLE, {"timestamp": time.time()}, source="scheduler"
            )
        )
