"""
SONA Emitter — Organism Self-Assessment Loop (Component 10/11)

SONA = φ-scaled organism self-assessment
Frequency: F(9) = 34 minutes = 2,040 seconds
Level: META (consciousness level 4)

"Every 34 minutes, the organism checks its own heartbeat."

The SONA_TICK is THE UNNAMEABLE's feedback loop — if judgments are failing
to converge, learning is stagnating, or anomalies spike, SONA_TICK triggers
meta-cognition to realign.

Lifecycle:
  sona = SonaEmitter(bus, pool)
  sona.start()           # Begin periodic SONA_TICK emission
  # ... run ...
  await sona.stop()      # Cancel gracefully
"""
from __future__ import annotations

import asyncio
import logging
import time
from typing import Any, Optional

from cynic.core.phi import fibonacci
from cynic.core.event_bus import (
    get_core_bus, Event, CoreEvent, EventBus,
)
from cynic.core.events_schema import SonaTickPayload


logger = logging.getLogger("cynic.organism.sona_emitter")

# F(9) = 34 minutes = 2,040 seconds (SONA heartbeat)
SONA_INTERVAL_S: float = float(fibonacci(9))  # 34


class SonaEmitter:
    """
    Emits SONA_TICK events at META consciousness level.

    The SONA (Self-Assessment Organism) is the feedback loop that tells CYNIC
    whether it's healthy or in crisis. Every F(9) seconds, SONA emits a packet
    of telemetry: Q-table population, judgment flow rate, EWC consolidation,
    uptime, etc.

    Subscribers (consumers of SONA_TICK):
      - Meta-cognition: Adjust learning rates based on health
      - E-Score updater: Recalibrate agent reputation
      - Budget manager: Check if costs are sustainable
      - The human: (dashboard shows SONA ticks)
    """

    def __init__(self, bus: EventBus | None = None, db_pool: Any = None) -> None:
        self._bus = bus or get_core_bus()
        self._db_pool = db_pool
        self._task: Optional[asyncio.Task] = None
        self._running = False
        self._tick_count = 0
        self._start_time = time.time()

    def start(self) -> None:
        """
        Begin SONA_TICK emission at F(9)-second intervals.

        Safe to call multiple times — idempotent.
        """
        if self._running:
            return

        self._running = True
        self._start_time = time.time()
        self._task = asyncio.create_task(self._run_loop())
        self._task.set_name("cynic.organism.sona_emitter")
        self._task.add_done_callback(self._on_task_done)
        logger.info(f"SonaEmitter started (interval={SONA_INTERVAL_S}s)")

    async def stop(self) -> None:
        """Cancel the SONA_TICK emission loop gracefully."""
        if not self._running:
            return

        self._running = False
        if self._task:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass

        logger.info(f"SonaEmitter stopped (emitted {self._tick_count} ticks)")

    async def _run_loop(self) -> None:
        """
        Main SONA heartbeat loop.

        Every F(9) seconds, collect telemetry and emit SONA_TICK.
        """
        while self._running:
            try:
                await asyncio.sleep(SONA_INTERVAL_S)

                if not self._running:
                    break

                # Emit SONA_TICK with current organism state
                await self._emit_sona_tick()

            except asyncio.CancelledError:
                break
            except Exception as exc:
                logger.error(f"SonaEmitter error in _run_loop: {exc}", exc_info=True)
                # Continue ticking even if one emission fails
                if not self._running:
                    break

    async def _emit_sona_tick(self) -> None:
        """Collect telemetry and emit a SONA_TICK event."""
        self._tick_count += 1
        uptime_s = time.time() - self._start_time

        # Telemetry payload: these fields are populated by orchestrator
        # For now, use sensible defaults. The actual q_table_entries, etc.
        # will be fetched from the orchestrator or DB in Phase 2.
        payload = SonaTickPayload(
            instance_id="",        # Set by organism.py if multi-instance
            q_table_entries=0,     # Fetched from Q-Learning pool
            total_judgments=0,     # Fetched from judgment history
            learning_rate=0.0,     # Current α from Thompson Sampling
            ewc_consolidated=0,    # EWC fisher entries locked
            uptime_s=uptime_s,     # Organism runtime
            interval_s=SONA_INTERVAL_S,
            tick_number=self._tick_count,
        )

        await self._bus.emit(Event.typed(
            CoreEvent.SONA_TICK,
            payload,
            source="sona_emitter",
        ))

        logger.debug(
            f"SONA_TICK #{self._tick_count} emitted (uptime={uptime_s:.1f}s, "
            f"q_entries={payload.q_table_entries})"
        )

    def _on_task_done(self, task: asyncio.Task) -> None:
        """Handle task completion (cancellation or crash)."""
        try:
            task.result()
        except asyncio.CancelledError:
            logger.debug("SonaEmitter task cancelled (expected)")
        except Exception as exc:
            logger.error(f"SonaEmitter task crashed: {exc}", exc_info=True)

    # ── Stats / Observability ───────────────────────────────────────────────

    def stats(self) -> dict[str, Any]:
        """Return SONA telemetry."""
        return {
            "running": self._running,
            "tick_count": self._tick_count,
            "uptime_s": time.time() - self._start_time,
            "interval_s": SONA_INTERVAL_S,
            "next_tick_in_s": SONA_INTERVAL_S - (time.time() - self._start_time) % SONA_INTERVAL_S,
        }
