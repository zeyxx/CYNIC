"""
CYNIC PerceiveWorkers — Autonomous Sensory Inputs

Without PerceiveWorkers, CYNIC is 100% reactive (waits for API calls).
With them, CYNIC autonomously observes its environment at defined intervals.

Architecture:
  PerceiveWorker.run(submit_fn) → background asyncio task
  sense() returns Optional[Cell] → if not None, submit_fn(cell) is called
  submit_fn = DogScheduler.submit (injected, no circular import)

Workers:
  GitWatcher     — CODE×PERCEIVE/REFLEX   — git changes every F(5)=5s
  HealthWatcher  — CYNIC×PERCEIVE/REFLEX  — timer degradation every F(8)=21s
  SelfWatcher    — CYNIC×LEARN/MICRO      — Q-Table health every F(10)=55s

φ-ratio target: 20% autonomous / 80% reactive
"""
from __future__ import annotations

import asyncio
import logging
import subprocess
import time
from abc import ABC, abstractmethod
from typing import Any, Callable, Dict, Optional

from cynic.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci

logger = logging.getLogger("cynic.perceive")


# ════════════════════════════════════════════════════════════════════════════
# ABSTRACT BASE
# ════════════════════════════════════════════════════════════════════════════

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
    async def sense(self) -> Optional[Cell]:
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


# ════════════════════════════════════════════════════════════════════════════
# GIT WATCHER — CODE×PERCEIVE/REFLEX every 5s
# ════════════════════════════════════════════════════════════════════════════

class GitWatcher(PerceiveWorker):
    """
    Monitors git working tree for uncommitted changes.

    Submits CODE×PERCEIVE at REFLEX level when new changes are detected.
    Deduplicates: only submits when the change set actually differs from last check.

    interval: F(5)=5s — git status is fast (<50ms), harmless to run often.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(5))   # 5.0s
    name = "git_watcher"

    def __init__(self, cwd: Optional[str] = None) -> None:
        self._cwd = cwd
        self._last_hash: Optional[int] = None

    async def sense(self) -> Optional[Cell]:
        loop = asyncio.get_event_loop()
        try:
            result = await loop.run_in_executor(
                None,
                lambda: subprocess.run(
                    ["git", "status", "--porcelain"],
                    capture_output=True, text=True, timeout=3.0,
                    cwd=self._cwd,
                ),
            )
        except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
            return None

        if result.returncode != 0 or not result.stdout.strip():
            self._last_hash = None
            return None

        changes = result.stdout.strip()
        change_hash = hash(changes)
        if change_hash == self._last_hash:
            return None     # No new changes since last check

        self._last_hash = change_hash
        lines = changes.splitlines()

        return Cell(
            reality="CODE",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={"git_status": changes[:1500], "changed_files": len(lines)},
            context=f"Git watcher: {len(lines)} changed file(s) detected",
            risk=0.0,
            complexity=min(len(lines) / 50.0, 1.0),
            budget_usd=0.001,
            metadata={"source": "git_watcher", "file_count": len(lines)},
        )


# ════════════════════════════════════════════════════════════════════════════
# HEALTH WATCHER — CYNIC×PERCEIVE/REFLEX every 21s
# ════════════════════════════════════════════════════════════════════════════

class HealthWatcher(PerceiveWorker):
    """
    Monitors CycleTimer health across all 4 consciousness levels.

    Submits CYNIC×PERCEIVE at REFLEX level only when a timer is
    DEGRADED or CRITICAL (not on every tick — only on bad news).
    This closes the self-monitoring loop: "CYNIC sees its own slowness."

    interval: F(8)=21s — enough resolution to detect degradation early.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(8))   # 21.0s
    name = "health_watcher"

    def __init__(self, get_consciousness_fn: Optional[Callable] = None) -> None:
        self._get_consciousness = get_consciousness_fn or get_consciousness

    async def sense(self) -> Optional[Cell]:
        consciousness = self._get_consciousness()
        degraded: Dict[str, Any] = {
            name: timer.to_dict()
            for name, timer in consciousness.timers.items()
            if timer.health in ("DEGRADED", "CRITICAL")
        }

        if not degraded:
            return None

        severity_rank = {"EXCELLENT": 0, "GOOD": 1, "UNKNOWN": 1, "DEGRADED": 2, "CRITICAL": 3}
        worst = max(
            consciousness.timers.values(),
            key=lambda t: severity_rank.get(t.health, 0),
        )

        return Cell(
            reality="CYNIC",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "degraded_levels": degraded,
                "worst_health": worst.health,
                "worst_p95_ms": worst.p95_ms,
                "total_cycles": consciousness.total_cycles,
            },
            context=(
                f"Health watcher: {len(degraded)} level(s) degraded "
                f"— worst={worst.health} p95={worst.p95_ms:.0f}ms"
            ),
            risk=0.2 if worst.health == "DEGRADED" else 0.5,
            complexity=0.3,
            budget_usd=0.001,
            metadata={"source": "health_watcher", "degraded_count": len(degraded)},
        )


# ════════════════════════════════════════════════════════════════════════════
# SELF WATCHER — CYNIC×LEARN/MICRO every 55s
# ════════════════════════════════════════════════════════════════════════════

class SelfWatcher(PerceiveWorker):
    """
    CYNIC observes its own Q-Table learning health.

    Submits CYNIC×LEARN at MICRO level every ~55s.
    Creates a self-judgment loop: "How well am I learning?"
    The judgment system judges its own learning state → feeds more Q-Learning.

    interval: F(10)=55s — ~1 minute is the right granularity for learning checks.
    """

    level = ConsciousnessLevel.MICRO
    interval_s = float(fibonacci(10))  # 55.0s
    name = "self_watcher"

    def __init__(self, qtable_getter: Optional[Callable] = None) -> None:
        self._qtable_getter = qtable_getter

    async def sense(self) -> Optional[Cell]:
        if self._qtable_getter is None:
            return None

        try:
            qtable = self._qtable_getter()
            stats = qtable.stats()
        except Exception:
            return None

        return Cell(
            reality="CYNIC",
            analysis="LEARN",
            time_dim="PRESENT",
            content={
                "states": stats.get("states", 0),
                "total_updates": stats.get("total_updates", 0),
                "pending_flush": stats.get("pending_flush", 0),
                "max_confidence": stats.get("max_confidence", 0.0),
                "unique_states": stats.get("unique_states", 0),
            },
            context=(
                f"Self-watcher: {stats.get('states', 0)} states learned, "
                f"{stats.get('total_updates', 0)} total updates"
            ),
            risk=0.0,
            complexity=0.2,
            budget_usd=0.003,
            metadata={"source": "self_watcher"},
        )
