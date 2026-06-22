"""
ComputeGuard — Disjoncteur thermique (Niveau 1 : Soft Kill applicatif).

Deux types de budget sont gardés simultanément :
  - Wall-clock time  : durée réelle d'exécution d'une tâche (secondes)
  - Compute units    : unité abstraite pour tout usage de ressource intensive
                       (tokens LLM, subprocess longs, appels réseau répétés)

Comportement :
  - Si une tâche dépasse son wall-clock budget → BudgetExceededError (Soft Kill)
  - Si la session dépasse le compute budget → BudgetExceededError (Soft Kill)
  - Le caller (Ouroboros) est responsable du Hard Kill (SIGTERM → subprocess)

L'organisme observe chaque dépassement via kernel.observe() avec domain="circuit_breaker".
Le kernel peut donc détecter les agents qui partent en boucle et escalader.

Défense en profondeur : cette couche (sémantique/métier) est aveugle au CPU/RAM.
La couche OS (resource.setrlimit dans l'executor) garde la porte hardware.
"""

import logging
import threading
import time
from contextlib import contextmanager
from dataclasses import dataclass, field
from typing import Optional

logger = logging.getLogger("compute_guard")


class BudgetExceededError(RuntimeError):
    """Raised when a task exceeds its allowed compute budget.

    The caller MUST:
    1. Stop the current work immediately.
    2. Call kernel.observe(domain="circuit_breaker", ...) to record the trip.
    3. Mark the task as failed with error = str(exc).
    4. NOT retry immediately — let Ouroboros backoff handle scheduling.
    """
    def __init__(self, reason: str, elapsed_secs: float, budget_secs: float):
        self.reason = reason
        self.elapsed_secs = elapsed_secs
        self.budget_secs = budget_secs
        super().__init__(
            f"Circuit breaker tripped: {reason} "
            f"(elapsed={elapsed_secs:.1f}s budget={budget_secs:.1f}s)"
        )


@dataclass
class ComputeGuard:
    """
    Thread-safe compute budget guard.

    Usage:
        guard = ComputeGuard(task_wall_clock_secs=600, session_compute_units=10_000)
        with guard.task_context(task_id="abc"):
            # long-running work here
            guard.tick(units=100)  # report usage periodically

    The guard is also a context manager for wall-clock enforcement via a watchdog thread.
    """

    # Per-task wall-clock limit — Hermes tasks default 600s (10min)
    task_wall_clock_secs: float = 600.0

    # Soft-kill threshold for a single task if it consumes too many units
    # (tokens, API calls, or any domain-specific unit)
    task_compute_units: int = 5_000

    # Cumulative session limit across all tasks — prevents agent loops
    session_compute_units: int = 50_000

    # Internal state (thread-safe via lock)
    _lock: threading.Lock = field(default_factory=threading.Lock)
    _session_units_used: int = field(default=0)
    _task_units_used: int = field(default=0)
    _task_start: Optional[float] = field(default=None)
    _task_id: Optional[str] = field(default=None)
    _watchdog: Optional[threading.Timer] = field(default=None)
    _tripped: bool = field(default=False)

    def __post_init__(self):
        self._lock = threading.Lock()
        self._session_units_used = 0
        self._task_units_used = 0
        self._task_start = None
        self._task_id = None
        self._watchdog = None
        self._tripped = False

    @contextmanager
    def task_context(self, task_id: str):
        """Context manager that enforces wall-clock budget for a task.

        Starts a watchdog timer. If the task exceeds task_wall_clock_secs,
        sets self._tripped = True. The caller must check guard.check() or
        catch BudgetExceededError from guard.tick().

        On exit (normal or exception), cancels the watchdog and resets per-task state.
        """
        with self._lock:
            self._task_id = task_id
            self._task_start = time.monotonic()
            self._task_units_used = 0
            self._tripped = False

        # Watchdog timer: sets tripped flag at wall-clock deadline
        def _watchdog_trip():
            with self._lock:
                if self._task_id == task_id:  # ensure we're still in this task
                    self._tripped = True
            logger.warning(
                "ComputeGuard watchdog tripped: task %s exceeded %.0fs wall-clock budget",
                task_id, self.task_wall_clock_secs,
            )

        watchdog = threading.Timer(self.task_wall_clock_secs, _watchdog_trip)
        watchdog.daemon = True
        watchdog.start()
        with self._lock:
            self._watchdog = watchdog

        try:
            yield self
        finally:
            watchdog.cancel()
            with self._lock:
                self._watchdog = None
                self._task_id = None
                self._task_start = None

    def tick(self, units: int = 1):
        """Report compute usage. Raises BudgetExceededError if any limit is exceeded.

        Call this at natural checkpoints (after each API call, after each subprocess poll, etc.).
        Do NOT call from hot loops — call at semantic boundaries.
        """
        with self._lock:
            # Check watchdog trip first (wall-clock)
            if self._tripped:
                elapsed = time.monotonic() - (self._task_start or time.monotonic())
                raise BudgetExceededError(
                    reason="wall_clock_timeout",
                    elapsed_secs=elapsed,
                    budget_secs=self.task_wall_clock_secs,
                )

            self._task_units_used += units
            self._session_units_used += units

            # Per-task compute budget
            if self._task_units_used > self.task_compute_units:
                raise BudgetExceededError(
                    reason="task_compute_budget",
                    elapsed_secs=time.monotonic() - (self._task_start or time.monotonic()),
                    budget_secs=self.task_wall_clock_secs,
                )

            # Session-level compute budget (anti-loop protection)
            if self._session_units_used > self.session_compute_units:
                raise BudgetExceededError(
                    reason="session_compute_budget",
                    elapsed_secs=time.monotonic() - (self._task_start or time.monotonic()),
                    budget_secs=self.task_wall_clock_secs,
                )

    def check(self):
        """Non-raising check — call in polling loops. Returns True if budget is still OK."""
        try:
            self.tick(units=0)
            return True
        except BudgetExceededError:
            return False

    @property
    def session_units_used(self) -> int:
        with self._lock:
            return self._session_units_used

    @property
    def task_elapsed_secs(self) -> float:
        with self._lock:
            if self._task_start is None:
                return 0.0
            return time.monotonic() - self._task_start

    def reset_session(self):
        """Reset session counter — call between unrelated work batches."""
        with self._lock:
            self._session_units_used = 0
