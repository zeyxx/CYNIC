"""
CYNIC ConsciousnessRhythm — 4-Tier Concurrent Consciousness + N Workers

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
from typing import Any, List

from collections.abc import Callable

from cynic.core.consciousness import (
    ConsciousnessLevel,
    ConsciousnessState,
    get_consciousness,
)
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci
import httpx

logger = logging.getLogger("cynic.scheduler")

# Bounded queue capacity per tier — F(10) = 55
_QUEUE_CAPACITY = fibonacci(10)  # 55

# Workers per tier — Fibonacci-derived for φ alignment
_WORKERS_PER_LEVEL: dict[ConsciousnessLevel, int] = {
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
# CONSCIOUSNESS RHYTHM
# ════════════════════════════════════════════════════════════════════════════

class ConsciousnessRhythm:
    """
    Manages 4-tier concurrent consciousness loops as N asyncio workers.

    Usage:
        scheduler = ConsciousnessRhythm(orchestrator)
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
        self._queues: dict[ConsciousnessLevel, asyncio.Queue] = {
            level: asyncio.Queue(maxsize=_QUEUE_CAPACITY)
            for level in ConsciousnessLevel
        }

        # REFLEX → MICRO interrupt (fires when REFLEX detects an anomaly)
        self._micro_interrupt = asyncio.Event()

        # Tier worker tasks (N per level)
        self._tasks: list[asyncio.Task] = []
        # Track tier worker metadata for restart (level, worker_id) → coroutine_fn
        self._tier_worker_metadata: dict[tuple, tuple] = {}  # (level, id) → (worker_fn, id)

        # PerceiveWorker tasks (autonomous sensors)
        self._perceive_workers: list[Any] = []   # List[PerceiveWorker]
        self._perceive_tasks: list[asyncio.Task] = []
        # Track perceive worker metadata for restart
        self._perceive_worker_metadata: dict[asyncio.Task, Any] = {}  # task → worker

        self._running = False

    # ── Worker Supervision ──────────────────────────────────────────────────────

    def _on_tier_worker_done(self, task: asyncio.Task, level: ConsciousnessLevel, worker_id: int) -> None:
        """Handle tier worker completion (death/crash) and restart if still running."""
        try:
            # Try to get the exception if the worker crashed
            task.result()
        except asyncio.CancelledError:
            logger.debug(f"Tier worker {level.name}:{worker_id} cancelled (expected)")
            return
        except Exception as exc:
            logger.error(f"Tier worker {level.name}:{worker_id} died: {exc}")

        # Only restart if scheduler is still running (not shutting down)
        if not self._running:
            logger.debug(f"Tier worker {level.name}:{worker_id} not restarted (scheduler stopped)")
            return

        # Restart the worker
        logger.info(f"Restarting tier worker {level.name}:{worker_id}...")
        worker_fn, _ = self._tier_worker_metadata.get((level, worker_id), (None, None))
        if worker_fn is None:
            logger.error(f"Cannot restart {level.name}:{worker_id} — metadata not found")
            return

        new_task = asyncio.ensure_future(worker_fn(worker_id=worker_id))
        new_task.set_name(f"cynic.scheduler.{level.name.lower()}.{worker_id}")
        new_task.add_done_callback(lambda t: self._on_tier_worker_done(t, level, worker_id))

        # Replace in task list
        try:
            idx = self._tasks.index(task)
            self._tasks[idx] = new_task
        except ValueError:
            self._tasks.append(new_task)

    def _on_perceive_worker_done(self, task: asyncio.Task, worker: Any) -> None:
        """Handle perceive worker completion (death/crash) and restart if still running."""
        try:
            task.result()
        except asyncio.CancelledError:
            logger.debug(f"Perceive worker {worker.name} cancelled (expected)")
            return
        except Exception as exc:
            logger.error(f"Perceive worker {worker.name} died: {exc}")

        # Only restart if scheduler is still running
        if not self._running:
            logger.debug(f"Perceive worker {worker.name} not restarted (scheduler stopped)")
            return

        # Restart the worker
        logger.info(f"Restarting perceive worker {worker.name}...")
        new_task = asyncio.ensure_future(worker.run(self.submit))
        new_task.set_name(f"cynic.senses.{worker.name}")
        new_task.add_done_callback(lambda t: self._on_perceive_worker_done(t, worker))

        # Replace in task list
        try:
            idx = self._perceive_tasks.index(task)
            self._perceive_tasks[idx] = new_task
        except ValueError:
            self._perceive_tasks.append(new_task)

    # ── Lifecycle ────────────────────────────────────────────────────────────

    def register_perceive_worker(self, worker: Any) -> None:
        """
        Register an autonomous PerceiveWorker to start alongside tier workers.

        Must be called BEFORE start(). Workers are started in start() and
        cancelled in stop(). Registering after start() is ignored with a warning.
        """
        if self._running:
            logger.warning(
                "register_perceive_worker(%s) called after start() — ignored. "
                "Register workers before calling start().",
                getattr(worker, "name", repr(worker)),
            )
            return
        self._perceive_workers.append(worker)

    def start(self) -> None:
        """
        Launch all tier workers (N per level) + all registered PerceiveWorkers.

        Tier workers per level:
          REFLEX: F(5)=5, MICRO: F(4)=3, MACRO: F(3)=2, META: 1
        Total tier workers: 11
        """
        if self._running:
            logger.warning("ConsciousnessRhythm already running — ignoring start()")
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
                # Store metadata for restart
                self._tier_worker_metadata[(level, i)] = (fn, i)
                # Add done callback for supervision
                task.add_done_callback(lambda t, lvl=level, wid=i: self._on_tier_worker_done(t, lvl, wid))
                self._tasks.append(task)
                total_workers += 1

        # Spawn PerceiveWorkers (autonomous sensors)
        for pw in self._perceive_workers:
            task = asyncio.ensure_future(pw.run(self.submit))
            task.set_name(f"cynic.senses.{pw.name}")
            # Store metadata for restart
            self._perceive_worker_metadata[task] = pw
            # Add done callback for supervision
            task.add_done_callback(lambda t, w=pw: self._on_perceive_worker_done(t, w))
            self._perceive_tasks.append(task)

        logger.info(
            "ConsciousnessRhythm started — %d tier workers, %d perceive workers "
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
        self._tier_worker_metadata.clear()
        self._perceive_worker_metadata.clear()
        logger.info("ConsciousnessRhythm stopped")

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

    def stats(self) -> dict[str, Any]:
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

    def total_queue_depth(self) -> int:
        """Total cells waiting across all queues — used by LOD health check."""
        return sum(q.qsize() for q in self._queues.values())

    # ── Tier Workers ─────────────────────────────────────────────────────────

    async def _reflex_worker(self, worker_id: int = 0) -> None:
        """
        L3 REFLEX worker — ~6ms cadence, non-LLM Dogs only.

        Multiple workers share the REFLEX queue. On BARK/GROWL verdict,
        fires interrupt_micro() to wake MICRO workers immediately.

        Rate limiting: When queue empty, sleeps adaptively to prevent busy-waiting.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.REFLEX.name)
        timeout_s = ConsciousnessLevel.REFLEX.target_ms / 1000.0

        while self._running:
            event = await self._drain_one(ConsciousnessLevel.REFLEX, timeout=timeout_s)
            if event is None:
                # Queue empty: sleep to prevent CPU thrashing
                backoff_ms = self._get_backoff_ms(ConsciousnessLevel.REFLEX)
                if backoff_ms > 0:
                    await asyncio.sleep(backoff_ms / 1000.0)
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
            except httpx.RequestError as exc:
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

        Rate limiting: When queue empty, sleeps adaptively to prevent CPU thrashing.
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
                # Queue empty: sleep to prevent CPU thrashing
                backoff_ms = self._get_backoff_ms(ConsciousnessLevel.MICRO)
                if backoff_ms > 0:
                    await asyncio.sleep(backoff_ms / 1000.0)
                continue

            t0 = time.perf_counter()
            try:
                await self._orchestrator.run(
                    cell=event.cell,
                    level=ConsciousnessLevel.MICRO,
                    budget_usd=event.budget_usd,
                )
            except httpx.RequestError as exc:
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

        Rate limiting: When queue empty, sleeps adaptively to prevent CPU thrashing.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.MACRO.name)
        timeout_s = ConsciousnessLevel.MACRO.target_ms / 1000.0

        while self._running:
            event = await self._drain_one(ConsciousnessLevel.MACRO, timeout=timeout_s)
            if event is None:
                # Queue empty: sleep to prevent CPU thrashing
                backoff_ms = self._get_backoff_ms(ConsciousnessLevel.MACRO)
                if backoff_ms > 0:
                    await asyncio.sleep(backoff_ms / 1000.0)
                continue

            t0 = time.perf_counter()
            try:
                await self._orchestrator.run(
                    cell=event.cell,
                    level=ConsciousnessLevel.MACRO,
                    budget_usd=event.budget_usd,
                )
            except httpx.RequestError as exc:
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

        Rate limiting: When queue empty, sleeps adaptively to prevent CPU thrashing.
        """
        timer = self._consciousness.timers.get(ConsciousnessLevel.META.name)
        timeout_s = ConsciousnessLevel.META.target_ms / 1000.0

        while self._running:
            event = await self._drain_one(ConsciousnessLevel.META, timeout=timeout_s)

            if not self._running:
                break

            # If queue empty and no event, sleep to prevent CPU thrashing
            if event is None:
                backoff_ms = self._get_backoff_ms(ConsciousnessLevel.META)
                if backoff_ms > 0:
                    await asyncio.sleep(backoff_ms / 1000.0)
                    continue

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
            except httpx.RequestError as exc:
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
        except TimeoutError:
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

    def _get_backoff_ms(self, level: ConsciousnessLevel) -> float:
        """
        Adaptive sleep backoff based on queue depth.

        Prevents busy-waiting when queue is empty by sleeping for an interval
        inversely proportional to queue fill percentage.

        Logic:
        - Queue empty (0%):     sleep = target_ms / num_workers
        - Queue 25% full:       sleep = target_ms / 2
        - Queue 50% full:       sleep = target_ms / 4
        - Queue 75%+ full:      sleep = 0 (urgent, no backoff)

        This balances responsiveness (low latency when busy) with efficiency
        (minimal CPU when idle).
        """
        queue = self._queues[level]
        depth = queue.qsize()
        capacity = _QUEUE_CAPACITY
        num_workers = _WORKERS_PER_LEVEL[level]
        target_ms = level.target_ms

        if depth == 0:
            # Queue empty: sleep for (target_ms / num_workers)
            # Prevents tight polling loop
            backoff = target_ms / num_workers
        elif depth < capacity * 0.25:
            # Queue 0-25% full: sleep for (target_ms / 2)
            backoff = target_ms / 2
        elif depth < capacity * 0.50:
            # Queue 25-50% full: sleep for (target_ms / 4)
            backoff = target_ms / 4
        else:
            # Queue 50%+ full: no sleep (urgent processing)
            backoff = 0

        return backoff

    @staticmethod
    def _infer_level(budget_usd: float) -> ConsciousnessLevel:
        """Infer consciousness level from available budget."""
        if budget_usd < 0.01:
            return ConsciousnessLevel.REFLEX
        if budget_usd < 0.05:
            return ConsciousnessLevel.MICRO
        return ConsciousnessLevel.MACRO
