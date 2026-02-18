"""
CYNIC DogScheduler — 4 Concurrent Consciousness Loops

Architecture decision (2026-02-18):
  4 = L(3) → TIERS     (operating frequency — when to think)
  7 = L(4) → GRADIENT  (introspection depth  — how deep to think per cell)

  Both are φ-derived Lucas numbers. They are ORTHOGONAL:
    - 4 tiers: managed by this Scheduler (concurrent asyncio loops)
    - 7 gradient: per-judgment budget-driven (ConsciousnessGradient in consciousness.py)

  The 4 tiers map onto the 7-gradient at even positions:
    REFLEX=0  (L3 tier) → MICRO=2 (L2 tier) → MACRO=4 (L1 tier) → META=6 (L4 tier)
    Odd positions 1,3,5 = natural transitions between tiers

Operating frequencies (Fibonacci-derived):
  REFLEX  F(3) × 3ms  ≈   6ms — non-LLM, continuous pattern matching
  MICRO   F(6) × 8ms  ≈  64ms — Dog voting, quick scoring (includes SCHOLAR)
  MACRO   F(8) × 21ms ≈ 441ms — Full PERCEIVE→EMERGE, temporal MCTS (all 11 Dogs)
  META    F(13) × 60s ≈ 4h    — Organism evolution, Fisher locking, E-Score update

Key behaviors:
  - 4 independent asyncio tasks, each reading from its bounded Queue
  - REFLEX can interrupt MICRO via asyncio.Event (expedited processing)
  - Low budget → submit() auto-downgrades to MICRO or REFLEX
  - CycleTimer tracks actual vs target latency per tier
  - META runs periodic evolution (ResidualDetector.evolve(), every 4h)
  - Graceful stop: drains in-flight, cancels idle loops
"""
from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Coroutine, Dict, List, Optional

from cynic.core.consciousness import (
    ConsciousnessLevel,
    ConsciousnessState,
    CycleTimer,
    get_consciousness,
)
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci, PHI_INV

logger = logging.getLogger("cynic.scheduler")

# Bounded queue capacity per tier — F(10) = 55 (Fibonacci closure: F(n)×L(n) = F(2n))
_QUEUE_CAPACITY = fibonacci(10)  # 55


# ════════════════════════════════════════════════════════════════════════════
# PERCEPTION EVENT — unit of work for the Scheduler
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class PerceptionEvent:
    """
    A unit of work queued for background processing.

    Created by /perceive when run_judgment=False, or by the REFLEX loop
    when continuous scanning detects an anomaly worth escalating.
    """
    cell: Cell
    level: ConsciousnessLevel = ConsciousnessLevel.MICRO
    submitted_at: float = field(default_factory=time.time)
    source: str = "api"  # e.g. "api", "reflex_scan", "meta_evolution"
    budget_usd: float = 0.05

    @property
    def wait_ms(self) -> float:
        return (time.time() - self.submitted_at) * 1000.0


# ════════════════════════════════════════════════════════════════════════════
# DOG SCHEDULER
# ════════════════════════════════════════════════════════════════════════════

class DogScheduler:
    """
    Manages 4 concurrent consciousness loops as asyncio Tasks.

    Usage (from lifespan / build_kernel):
        scheduler = DogScheduler(orchestrator)
        scheduler.start()        # begins all 4 loops
        # ... serve requests ...
        await scheduler.stop()   # drains + cancels

    Submitting work:
        scheduler.submit(cell, level=ConsciousnessLevel.MICRO)
        # or let submit() pick the level based on budget:
        scheduler.submit(cell, budget_usd=0.01)  # → auto-downgrades to REFLEX
    """

    def __init__(self, orchestrator: Any) -> None:
        """
        orchestrator: JudgeOrchestrator — called per perception event.
        """
        self._orchestrator = orchestrator
        self._consciousness: ConsciousnessState = get_consciousness()

        # One bounded queue per level
        self._queues: Dict[ConsciousnessLevel, asyncio.Queue] = {
            level: asyncio.Queue(maxsize=_QUEUE_CAPACITY)
            for level in ConsciousnessLevel
        }

        # REFLEX → MICRO interrupt event (fires when REFLEX finds something urgent)
        self._micro_interrupt = asyncio.Event()

        # Running asyncio Tasks
        self._tasks: List[asyncio.Task] = []
        self._running = False

    # ── Lifecycle ────────────────────────────────────────────────────────────

    def start(self) -> None:
        """Launch all 4 consciousness loops as asyncio background tasks."""
        if self._running:
            logger.warning("DogScheduler already running — ignoring start()")
            return
        self._running = True
        loop_fns = {
            ConsciousnessLevel.REFLEX: self._reflex_loop,
            ConsciousnessLevel.MICRO:  self._micro_loop,
            ConsciousnessLevel.MACRO:  self._macro_loop,
            ConsciousnessLevel.META:   self._meta_loop,
        }
        for level, fn in loop_fns.items():
            task = asyncio.ensure_future(fn())
            task.set_name(f"cynic.scheduler.{level.name.lower()}")
            self._tasks.append(task)
        logger.info(
            "DogScheduler started — %d loops active (queues: capacity=%d)",
            len(self._tasks), _QUEUE_CAPACITY,
        )

    async def stop(self) -> None:
        """Signal all loops to stop and wait for graceful shutdown."""
        self._running = False
        # Wake loops blocked on queue.get() so they can check _running
        for q in self._queues.values():
            try:
                q.put_nowait(None)  # sentinel
            except asyncio.QueueFull:
                pass
        self._micro_interrupt.set()

        if self._tasks:
            await asyncio.gather(*self._tasks, return_exceptions=True)
            self._tasks.clear()
        logger.info("DogScheduler stopped")

    # ── Submit ───────────────────────────────────────────────────────────────

    def submit(
        self,
        cell: Cell,
        level: Optional[ConsciousnessLevel] = None,
        budget_usd: float = 0.05,
        source: str = "api",
    ) -> bool:
        """
        Enqueue a perception event for background processing.

        If level is None, infers from budget:
          budget < 0.01 → REFLEX
          budget < 0.05 → MICRO
          else          → MACRO

        Returns True if successfully enqueued, False if queue is full.
        """
        if level is None:
            level = self._infer_level(budget_usd)

        event = PerceptionEvent(cell=cell, level=level, source=source, budget_usd=budget_usd)
        queue = self._queues[level]
        try:
            queue.put_nowait(event)
            logger.debug(
                "Submitted %s to %s queue (depth=%d, wait=0ms)",
                cell.cell_id[:8], level.name, queue.qsize(),
            )
            return True
        except asyncio.QueueFull:
            logger.warning(
                "%s queue full (%d/%d) — dropping %s",
                level.name, queue.qsize(), _QUEUE_CAPACITY, cell.cell_id[:8],
            )
            return False

    def interrupt_micro(self) -> None:
        """
        Signal the MICRO loop to process immediately (called by REFLEX on anomaly).
        Clears automatically after MICRO wakes.
        """
        self._micro_interrupt.set()

    # ── Stats ────────────────────────────────────────────────────────────────

    def stats(self) -> Dict[str, Any]:
        """Returns queue depths, CycleTimer stats, and loop health."""
        timers = self._consciousness.timers
        return {
            "running": self._running,
            "queues": {
                level.name: {
                    "depth": self._queues[level].qsize(),
                    "capacity": _QUEUE_CAPACITY,
                }
                for level in ConsciousnessLevel
            },
            "timers": {
                level.name: timers[level.name].to_dict()
                for level in ConsciousnessLevel
                if level.name in timers
            },
            "cycles": {
                "REFLEX": self._consciousness.reflex_cycles,
                "MICRO":  self._consciousness.micro_cycles,
                "MACRO":  self._consciousness.macro_cycles,
                "META":   self._consciousness.meta_cycles,
                "total":  self._consciousness.total_cycles,
            },
        }

    # ── Level loops ──────────────────────────────────────────────────────────

    async def _reflex_loop(self) -> None:
        """
        L3 REFLEX — ~6ms cadence, non-LLM Dogs only.

        Processes queued REFLEX events. On anomaly detection, sets
        micro_interrupt to wake MICRO immediately.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.REFLEX.name)
        sleep_s = ConsciousnessLevel.REFLEX.target_ms / 1000.0

        while self._running:
            event = await self._drain_one(ConsciousnessLevel.REFLEX, timeout=sleep_s)
            if event is None:
                continue

            timer and timer.start()
            try:
                result = await self._orchestrator.run(
                    cell=event.cell,
                    level=ConsciousnessLevel.REFLEX,
                    budget_usd=event.budget_usd,
                )
                # If anomaly detected → wake MICRO immediately
                if result and result.get("verdict") in ("BARK", "GROWL"):
                    self.interrupt_micro()
                    logger.debug("REFLEX anomaly → MICRO interrupted")
            except Exception as exc:
                logger.warning("REFLEX loop error: %s", exc)
            finally:
                elapsed = timer.stop() if timer else 0.0
                self._consciousness.increment(ConsciousnessLevel.REFLEX)
                logger.debug("REFLEX cycle: %.1fms", elapsed)

    async def _micro_loop(self) -> None:
        """
        L2 MICRO — ~64ms cadence, Dog voting + SCHOLAR.

        Woken early by REFLEX interrupt when anomaly detected.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.MICRO.name)
        sleep_s = ConsciousnessLevel.MICRO.target_ms / 1000.0

        while self._running:
            # Wait for either: item in queue OR REFLEX interrupt OR timeout
            try:
                event = await asyncio.wait_for(
                    self._queues[ConsciousnessLevel.MICRO].get(),
                    timeout=sleep_s,
                )
            except asyncio.TimeoutError:
                # Timeout — check for REFLEX interrupt, then continue
                if self._micro_interrupt.is_set():
                    self._micro_interrupt.clear()
                    logger.debug("MICRO woken by REFLEX interrupt")
                continue

            if event is None:  # sentinel (stop signal)
                break

            if self._micro_interrupt.is_set():
                self._micro_interrupt.clear()

            timer and timer.start()
            try:
                await self._orchestrator.run(
                    cell=event.cell,
                    level=ConsciousnessLevel.MICRO,
                    budget_usd=event.budget_usd,
                )
            except Exception as exc:
                logger.warning("MICRO loop error: %s", exc)
            finally:
                elapsed = timer.stop() if timer else 0.0
                self._consciousness.increment(ConsciousnessLevel.MICRO)
                self._queues[ConsciousnessLevel.MICRO].task_done()
                logger.debug("MICRO cycle: %.1fms (queue=%d)", elapsed, self._queues[ConsciousnessLevel.MICRO].qsize())

    async def _macro_loop(self) -> None:
        """
        L1 MACRO — ~441ms cadence, full 7-step cycle with temporal MCTS.

        All 11 Dogs. SageDog runs 7 parallel Ollama calls.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.MACRO.name)
        sleep_s = ConsciousnessLevel.MACRO.target_ms / 1000.0

        while self._running:
            event = await self._drain_one(ConsciousnessLevel.MACRO, timeout=sleep_s)
            if event is None:
                continue

            timer and timer.start()
            try:
                await self._orchestrator.run(
                    cell=event.cell,
                    level=ConsciousnessLevel.MACRO,
                    budget_usd=event.budget_usd,
                )
            except Exception as exc:
                logger.warning("MACRO loop error: %s", exc)
            finally:
                elapsed = timer.stop() if timer else 0.0
                self._consciousness.increment(ConsciousnessLevel.MACRO)
                logger.debug("MACRO cycle: %.1fms (queue=%d)", elapsed, self._queues[ConsciousnessLevel.MACRO].qsize())

    async def _meta_loop(self) -> None:
        """
        L4 META — every ~4h (F(13) × 60s ≈ 233 min).

        Organism evolution: ResidualDetector.evolve(), Fisher locking,
        E-Score update, pattern consolidation.

        Also drains any items queued at META level.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.META.name)
        sleep_s = ConsciousnessLevel.META.target_ms / 1000.0

        while self._running:
            # Wait for the full META interval (or a queued META event)
            event = await self._drain_one(ConsciousnessLevel.META, timeout=sleep_s)

            if not self._running:
                break

            timer and timer.start()
            try:
                if event is not None:
                    # Process explicit META event
                    await self._orchestrator.run(
                        cell=event.cell,
                        level=ConsciousnessLevel.META,
                        budget_usd=event.budget_usd,
                    )
                else:
                    # Periodic evolution tick
                    await self._meta_evolve()
            except Exception as exc:
                logger.warning("META loop error: %s", exc)
            finally:
                elapsed = timer.stop() if timer else 0.0
                self._consciousness.increment(ConsciousnessLevel.META)
                logger.info("META cycle complete: %.1fms", elapsed)

    # ── Helpers ───────────────────────────────────────────────────────────────

    async def _drain_one(
        self,
        level: ConsciousnessLevel,
        timeout: float,
    ) -> Optional[PerceptionEvent]:
        """
        Wait up to `timeout` seconds for one item from the level's queue.
        Returns None on timeout or sentinel.
        """
        try:
            item = await asyncio.wait_for(
                self._queues[level].get(), timeout=timeout,
            )
            if item is None:
                return None  # sentinel → stop
            self._queues[level].task_done()
            return item
        except asyncio.TimeoutError:
            return None

    async def _meta_evolve(self) -> None:
        """
        Periodic organism evolution (every ~4h).

        Calls orchestrator's evolution hook if available.
        This is where Fisher locking, E-Score consolidation, and
        pattern compression happen.
        """
        evolve_fn: Optional[Callable] = getattr(
            self._orchestrator, "evolve", None,
        )
        if evolve_fn is not None:
            if asyncio.iscoroutinefunction(evolve_fn):
                await evolve_fn()
            else:
                evolve_fn()
            logger.info("META evolution completed")
        else:
            logger.debug("META evolution: orchestrator has no evolve() hook yet")

    @staticmethod
    def _infer_level(budget_usd: float) -> ConsciousnessLevel:
        """Infer consciousness level from available budget."""
        if budget_usd < 0.01:
            return ConsciousnessLevel.REFLEX
        if budget_usd < 0.05:
            return ConsciousnessLevel.MICRO
        return ConsciousnessLevel.MACRO
