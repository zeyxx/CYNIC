"""
CYNIC PerceiveWorker — Abstract base for all autonomous sensory workers.

Workers observe their domain at Fibonacci intervals and submit Cells to the
DogScheduler when something worth judging is detected.
"""
from __future__ import annotations

import asyncio
import logging
from abc import ABC, abstractmethod
from typing import Optional
from collections.abc import Callable

from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci

logger = logging.getLogger("cynic.perceive")


class PerceiveWorker(ABC):
    """
    Autonomous sensor that generates PerceptionEvents for the Scheduler.

    CYNIC starts these as background asyncio tasks in DogScheduler.start().
    Each worker:
      1. Calls sense() to observe its domain
      2. If sense() returns a Cell → calls submit_fn(cell)
      3. Sleeps interval_s seconds
      4. Repeats until task is cancelled

    Usage:
        worker = GitWatcher()
        task = asyncio.ensure_future(worker.run(scheduler.submit))
    """

    #: Target queue level for submitted cells (override in subclasses)
    level: ConsciousnessLevel = ConsciousnessLevel.REFLEX

    #: Seconds between sense() calls (Fibonacci-derived)
    interval_s: float = float(fibonacci(5))  # 5.0s default

    #: Short name for logging and source tagging
    name: str = "perceive_worker"

    @abstractmethod
    async def sense(self) -> Cell | None:
        """
        Observe the worker's domain.

        Returns a Cell if something worth judging was detected, else None.
        Must be non-blocking (use run_in_executor for blocking calls).
        CancelledError must propagate (do not catch it here).
        """
        ...

    async def run(self, submit_fn: Callable) -> None:
        """
        Main loop: sense → submit → sleep → repeat.

        submit_fn: DogScheduler.submit (or any callable matching its signature)
        Runs as a background asyncio task; exits cleanly on CancelledError.
        """
        logger.info(
            "PerceiveWorker %s started (interval=%.1fs, level=%s)",
            self.name, self.interval_s, self.level.name,
        )
        while True:
            try:
                cell = await self.sense()
                if cell is not None:
                    submitted = submit_fn(
                        cell,
                        level=self.level,
                        budget_usd=cell.budget_usd,
                        source=self.name,
                    )
                    if submitted:
                        logger.debug(
                            "%s: submitted %s to %s",
                            self.name, cell.cell_id[:8], self.level.name,
                        )
                    else:
                        logger.debug("%s: queue full, cell dropped", self.name)
            except asyncio.CancelledError:
                logger.info("PerceiveWorker %s cancelled", self.name)
                return
            except Exception as exc:
                logger.warning("PerceiveWorker %s sense() error: %s", self.name, exc)

            # CancelledError raised here exits the loop cleanly
            await asyncio.sleep(self.interval_s)

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(interval={self.interval_s}s, level={self.level.name})"
