"""
CYNIC DogScheduler — 4-Tier Concurrent Consciousness + N Workers

Architecture decision (2026-02-18):
  4 = L(3) → TIERS     (operating frequency — when to think)
  7 = L(4) → GRADIENT  (introspection depth  — how deep to think per cell)

  Both φ-derived Lucas numbers. ORTHOGONAL:
    - 4 tiers: managed by this Scheduler (concurrent asyncio loops)
    - 7 gradient: per-judgment budget-driven (ConsciousnessGradient in consciousness.py)

Operating frequencies (Fibonacci-derived):
  REFLEX  F(3) × 3ms  ≈   6ms — non-LLM, continuous pattern matching
  MICRO   F(6) × 8ms  ≈  64ms — Dog voting, quick scoring (includes SCHOLAR)
  MACRO   F(8) × 21ms ≈ 441ms — Full PERCEIVE→EMERGE, temporal MCTS (all 11 Dogs)
  META    F(13) × 60s ≈ 4h    — Organism evolution, Fisher locking, E-Score update

N Workers per tier (parallel cell processing):
  REFLEX: F(5)=5 workers — non-LLM, fast; 5 cells processed simultaneously
  MICRO:  F(4)=3 workers — some LLM, ~500ms; 3 in parallel
  MACRO:  F(3)=2 workers — full LLM, ~2.85s; limited by Ollama capacity
  META:   1 worker       — singleton, periodic evolution

  Multiple workers share one asyncio.Queue per tier.
  asyncio.Queue.get() is atomic: each cell claimed by exactly one worker.
  Result: N× throughput with zero coordination overhead.

PerceiveWorkers (autonomous sensors):
  Registered via register_perceive_worker() before start().
  Started as background tasks alongside tier workers.
  Generate PerceptionEvents without external API calls.
  Workers: GitWatcher, HealthWatcher, SelfWatcher (see perceive/workers.py)
"""
from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional

from cynic.core.consciousness import (
    ConsciousnessLevel,
    ConsciousnessState,
    get_consciousness,
)
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci

logger = logging.getLogger("cynic.scheduler")

# Bounded queue capacity per tier — F(10) = 55
_QUEUE_CAPACITY = fibonacci(10)  # 55

# Workers per tier — Fibonacci-derived for φ alignment
_WORKERS_PER_LEVEL: Dict[ConsciousnessLevel, int] = {
    ConsciousnessLevel.REFLEX: fibonacci(5),  # 5 — non-LLM, fast
    ConsciousnessLevel.MICRO:  fibonacci(4),  # 3 — some LLM, 500ms
    ConsciousnessLevel.MACRO:  fibonacci(3),  # 2 — full LLM, Ollama-bound
    ConsciousnessLevel.META:   1,             # singleton — periodic evolution
}


# ════════════════════════════════════════════════════════════════════════════
# PERCEPTION EVENT — unit of work for the Scheduler
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class PerceptionEvent:
    """
    A unit of work queued for background processing.

    Created by /perceive when run_judgment=False, or by PerceiveWorkers
    when autonomous scanning detects something worth judging.
    """
    cell: Cell
    level: ConsciousnessLevel = ConsciousnessLevel.MICRO
    submitted_at: float = field(default_factory=time.time)
    source: str = "api"
    budget_usd: float = 0.05

    @property
    def wait_ms(self) -> float:
        return (time.time() - self.submitted_at) * 1000.0


# ════════════════════════════════════════════════════════════════════════════
# DOG SCHEDULER
# ════════════════════════════════════════════════════════════════════════════

class DogScheduler:
    """
    Manages 4-tier concurrent consciousness loops as N asyncio workers.

    Usage:
        scheduler = DogScheduler(orchestrator)
        scheduler.register_perceive_worker(GitWatcher())
        scheduler.start()           # begins all tier workers + perceive workers
        # ... serve requests ...
        await scheduler.stop()      # cancels all tasks gracefully

    Submitting work:
        scheduler.submit(cell, level=ConsciousnessLevel.MICRO)
        # or budget-based auto-select:
        scheduler.submit(cell, budget_usd=0.01)  # → MICRO
    """

    def __init__(self, orchestrator: Any) -> None:
        self._orchestrator = orchestrator
        self._consciousness: ConsciousnessState = get_consciousness()

        # One bounded queue per level — shared by all N workers of that tier
        self._queues: Dict[ConsciousnessLevel, asyncio.Queue] = {
            level: asyncio.Queue(maxsize=_QUEUE_CAPACITY)
            for level in ConsciousnessLevel
        }

        # REFLEX → MICRO interrupt (fires when REFLEX detects an anomaly)
        self._micro_interrupt = asyncio.Event()

        # Tier worker tasks (N per level)
        self._tasks: List[asyncio.Task] = []

        # PerceiveWorker tasks (autonomous sensors)
        self._perceive_workers: List[Any] = []   # List[PerceiveWorker]
        self._perceive_tasks: List[asyncio.Task] = []

        self._running = False

    # ── Lifecycle ────────────────────────────────────────────────────────────

    def register_perceive_worker(self, worker: Any) -> None:
        """
        Register an autonomous PerceiveWorker to start alongside tier workers.

        Must be called BEFORE start(). Workers are started in start() and
        cancelled in stop(). Registering after start() has no effect.
        """
        self._perceive_workers.append(worker)

    def start(self) -> None:
        """
        Launch all tier workers (N per level) + all registered PerceiveWorkers.

        Tier workers per level:
          REFLEX: F(5)=5, MICRO: F(4)=3, MACRO: F(3)=2, META: 1
        Total tier workers: 11
        """
        if self._running:
            logger.warning("DogScheduler already running — ignoring start()")
            return
        self._running = True

        # Map each level to its worker coroutine
        worker_fns = {
            ConsciousnessLevel.REFLEX: self._reflex_worker,
            ConsciousnessLevel.MICRO:  self._micro_worker,
            ConsciousnessLevel.MACRO:  self._macro_worker,
            ConsciousnessLevel.META:   self._meta_loop,
        }

        # Spawn N workers per tier (shared queue, atomic get())
        total_workers = 0
        for level, fn in worker_fns.items():
            n = _WORKERS_PER_LEVEL[level]
            for i in range(n):
                task = asyncio.ensure_future(fn(worker_id=i))
                task.set_name(f"cynic.scheduler.{level.name.lower()}.{i}")
                self._tasks.append(task)
                total_workers += 1

        # Spawn PerceiveWorkers (autonomous sensors)
        for pw in self._perceive_workers:
            task = asyncio.ensure_future(pw.run(self.submit))
            task.set_name(f"cynic.perceive.{pw.name}")
            self._perceive_tasks.append(task)

        logger.info(
            "DogScheduler started — %d tier workers, %d perceive workers "
            "(queues: capacity=%d)",
            total_workers, len(self._perceive_tasks), _QUEUE_CAPACITY,
        )

    async def stop(self) -> None:
        """Cancel all tasks and wait for graceful shutdown."""
        self._running = False
        self._micro_interrupt.set()     # Wake any blocked MICRO worker

        all_tasks = self._tasks + self._perceive_tasks
        for task in all_tasks:
            task.cancel()

        if all_tasks:
            await asyncio.gather(*all_tasks, return_exceptions=True)

        self._tasks.clear()
        self._perceive_tasks.clear()
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

        Level auto-selected from budget if not specified:
          budget < 0.01 → REFLEX
          budget < 0.05 → MICRO
          else          → MACRO

        Returns True if enqueued, False if queue is full (item dropped).
        """
        if level is None:
            level = self._infer_level(budget_usd)

        event = PerceptionEvent(cell=cell, level=level, source=source, budget_usd=budget_usd)
        queue = self._queues[level]
        try:
            queue.put_nowait(event)
            logger.debug(
                "Submitted %s to %s queue (depth=%d)",
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
        """Signal MICRO workers to process immediately (called by REFLEX on anomaly)."""
        self._micro_interrupt.set()

    # ── Stats ────────────────────────────────────────────────────────────────

    def stats(self) -> Dict[str, Any]:
        """Queue depths, worker counts, CycleTimer stats, cycle counts."""
        timers = self._consciousness.timers
        return {
            "running": self._running,
            "workers_per_level": {l.name: n for l, n in _WORKERS_PER_LEVEL.items()},
            "perceive_workers": len(self._perceive_workers),
            "queues": {
                level.name: {
                    "depth":    self._queues[level].qsize(),
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

    # ── Tier Workers ─────────────────────────────────────────────────────────

    async def _reflex_worker(self, worker_id: int = 0) -> None:
        """
        L3 REFLEX worker — ~6ms cadence, non-LLM Dogs only.

        Multiple workers share the REFLEX queue. On BARK/GROWL verdict,
        fires interrupt_micro() to wake MICRO workers immediately.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.REFLEX.name)
        timeout_s = ConsciousnessLevel.REFLEX.target_ms / 1000.0

        while self._running:
            event = await self._drain_one(ConsciousnessLevel.REFLEX, timeout=timeout_s)
            if event is None:
                continue

            t0 = time.perf_counter()
            try:
                result = await self._orchestrator.run(
                    cell=event.cell,
                    level=ConsciousnessLevel.REFLEX,
                    budget_usd=event.budget_usd,
                )
                # Anomaly → wake MICRO immediately
                verdict = getattr(result, "verdict", None)
                if verdict in ("BARK", "GROWL"):
                    self.interrupt_micro()
                    logger.debug("REFLEX worker %d: %s anomaly → MICRO interrupt", worker_id, verdict)
            except Exception as exc:
                logger.warning("REFLEX worker %d error: %s", worker_id, exc)
            finally:
                elapsed_ms = (time.perf_counter() - t0) * 1000
                if timer:
                    timer.record(elapsed_ms)
                self._consciousness.increment(ConsciousnessLevel.REFLEX)

    async def _micro_worker(self, worker_id: int = 0) -> None:
        """
        L2 MICRO worker — ~64ms cadence, Dog voting + SCHOLAR.

        Multiple workers share the MICRO queue.
        Worker 0 also responds to REFLEX interrupts (woken by asyncio.Event).
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.MICRO.name)
        timeout_s = ConsciousnessLevel.MICRO.target_ms / 1000.0

        while self._running:
            # Worker 0 also listens for REFLEX interrupt
            if worker_id == 0 and self._micro_interrupt.is_set():
                self._micro_interrupt.clear()
                logger.debug("MICRO worker 0: woken by REFLEX interrupt")

            event = await self._drain_one(ConsciousnessLevel.MICRO, timeout=timeout_s)
            if event is None:
                continue

            t0 = time.perf_counter()
            try:
                await self._orchestrator.run(
                    cell=event.cell,
                    level=ConsciousnessLevel.MICRO,
                    budget_usd=event.budget_usd,
                )
            except Exception as exc:
                logger.warning("MICRO worker %d error: %s", worker_id, exc)
            finally:
                elapsed_ms = (time.perf_counter() - t0) * 1000
                if timer:
                    timer.record(elapsed_ms)
                self._consciousness.increment(ConsciousnessLevel.MICRO)
                logger.debug(
                    "MICRO worker %d: %.1fms (queue=%d)",
                    worker_id, elapsed_ms, self._queues[ConsciousnessLevel.MICRO].qsize(),
                )

    async def _macro_worker(self, worker_id: int = 0) -> None:
        """
        L1 MACRO worker — ~441ms cadence, full 7-step cycle, temporal MCTS.

        Multiple workers share the MACRO queue (limited by Ollama capacity).
        Each MACRO call runs 11 Dogs in parallel, SageDog runs 7 Ollama calls.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.MACRO.name)
        timeout_s = ConsciousnessLevel.MACRO.target_ms / 1000.0

        while self._running:
            event = await self._drain_one(ConsciousnessLevel.MACRO, timeout=timeout_s)
            if event is None:
                continue

            t0 = time.perf_counter()
            try:
                await self._orchestrator.run(
                    cell=event.cell,
                    level=ConsciousnessLevel.MACRO,
                    budget_usd=event.budget_usd,
                )
            except Exception as exc:
                logger.warning("MACRO worker %d error: %s", worker_id, exc)
            finally:
                elapsed_ms = (time.perf_counter() - t0) * 1000
                if timer:
                    timer.record(elapsed_ms)
                self._consciousness.increment(ConsciousnessLevel.MACRO)
                logger.debug(
                    "MACRO worker %d: %.1fms (queue=%d)",
                    worker_id, elapsed_ms, self._queues[ConsciousnessLevel.MACRO].qsize(),
                )

    async def _meta_loop(self, worker_id: int = 0) -> None:
        """
        L4 META — every ~4h (F(13) × 60s ≈ 233 min).

        Single worker. Drains META queue or runs periodic evolution tick.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.META.name)
        timeout_s = ConsciousnessLevel.META.target_ms / 1000.0

        while self._running:
            event = await self._drain_one(ConsciousnessLevel.META, timeout=timeout_s)

            if not self._running:
                break

            t0 = time.perf_counter()
            try:
                if event is not None:
                    await self._orchestrator.run(
                        cell=event.cell,
                        level=ConsciousnessLevel.META,
                        budget_usd=event.budget_usd,
                    )
                else:
                    await self._meta_evolve()
            except Exception as exc:
                logger.warning("META loop error: %s", exc)
            finally:
                elapsed_ms = (time.perf_counter() - t0) * 1000
                if timer:
                    timer.record(elapsed_ms)
                self._consciousness.increment(ConsciousnessLevel.META)
                logger.info("META cycle complete: %.1fms", elapsed_ms)

    # ── Helpers ───────────────────────────────────────────────────────────────

    async def _drain_one(
        self,
        level: ConsciousnessLevel,
        timeout: float,
    ) -> Optional[PerceptionEvent]:
        """
        Wait up to `timeout` seconds for one item from the level's queue.

        Multiple workers calling this concurrently: asyncio.Queue.get() is
        atomic, so each item is claimed by exactly one worker (no duplication).
        Returns None on timeout (worker continues polling loop).
        """
        try:
            item = await asyncio.wait_for(
                self._queues[level].get(), timeout=timeout,
            )
            if item is None:
                return None
            self._queues[level].task_done()
            return item
        except asyncio.TimeoutError:
            return None

    async def _meta_evolve(self) -> None:
        """Periodic evolution tick — calls orchestrator.evolve() if present."""
        evolve_fn: Optional[Callable] = getattr(self._orchestrator, "evolve", None)
        if evolve_fn is not None:
            if asyncio.iscoroutinefunction(evolve_fn):
                await evolve_fn()
            else:
                evolve_fn()
            logger.info("META evolution completed")
        else:
            logger.debug("META evolution: orchestrator has no evolve() hook")

    @staticmethod
    def _infer_level(budget_usd: float) -> ConsciousnessLevel:
        """Infer consciousness level from available budget."""
        if budget_usd < 0.01:
            return ConsciousnessLevel.REFLEX
        if budget_usd < 0.05:
            return ConsciousnessLevel.MICRO
        return ConsciousnessLevel.MACRO
