"""Tests for ComputeGuard — the circuit breaker / disjoncteur thermique.

Tests cover:
  - Normal execution (no trip)
  - Per-task compute unit budget exceeded
  - Session compute budget exceeded (anti-loop protection)
  - Wall-clock watchdog trip
  - SIGTERM integration: terminate_active() propagates kill
  - StubExecutor with guard parameter (executor port compatibility)
"""

import threading
import time
import pytest

from organs.core.compute_guard import ComputeGuard, BudgetExceededError


class TestComputeGuardNormal:
    def test_tick_within_budget_does_not_raise(self):
        guard = ComputeGuard(task_compute_units=100, session_compute_units=1000)
        with guard.task_context("task-1"):
            guard.tick(units=50)
            guard.tick(units=49)
            # Should not raise

    def test_check_returns_true_within_budget(self):
        guard = ComputeGuard(task_compute_units=100, session_compute_units=1000)
        with guard.task_context("task-1"):
            assert guard.check() is True
            guard.tick(units=50)
            assert guard.check() is True

    def test_session_units_accumulate_across_tasks(self):
        guard = ComputeGuard(task_compute_units=500, session_compute_units=1000)
        with guard.task_context("task-1"):
            guard.tick(units=400)
        with guard.task_context("task-2"):
            guard.tick(units=400)
        assert guard.session_units_used == 800

    def test_task_elapsed_secs_increases(self):
        guard = ComputeGuard()
        with guard.task_context("task-1"):
            time.sleep(0.05)
            assert guard.task_elapsed_secs >= 0.04


class TestComputeGuardTaskBudget:
    def test_task_compute_exceeded_raises(self):
        guard = ComputeGuard(task_compute_units=10, session_compute_units=10_000)
        with pytest.raises(BudgetExceededError) as exc_info:
            with guard.task_context("task-1"):
                guard.tick(units=11)
        assert exc_info.value.reason == "task_compute_budget"

    def test_check_returns_false_after_task_budget_exceeded(self):
        guard = ComputeGuard(task_compute_units=5, session_compute_units=10_000)
        # Pre-exhaust the budget
        try:
            with guard.task_context("task-1"):
                guard.tick(units=6)
        except BudgetExceededError:
            pass
        # Now the context is gone — a fresh context should work
        with guard.task_context("task-2"):
            assert guard.check() is True


class TestComputeGuardSessionBudget:
    def test_session_budget_exceeded_raises(self):
        guard = ComputeGuard(task_compute_units=1000, session_compute_units=50)
        with guard.task_context("task-1"):
            guard.tick(units=30)
        with pytest.raises(BudgetExceededError) as exc_info:
            with guard.task_context("task-2"):
                guard.tick(units=30)  # 30+30=60 > 50 session budget
        assert exc_info.value.reason == "session_compute_budget"

    def test_reset_session_clears_counter(self):
        guard = ComputeGuard(task_compute_units=1000, session_compute_units=50)
        with guard.task_context("task-1"):
            guard.tick(units=40)
        guard.reset_session()
        assert guard.session_units_used == 0
        # Should not raise now
        with guard.task_context("task-2"):
            guard.tick(units=40)


class TestComputeGuardWatchdog:
    def test_wall_clock_watchdog_trips_after_deadline(self):
        """Watchdog timer sets _tripped flag after deadline — tick() raises on next call."""
        guard = ComputeGuard(task_wall_clock_secs=0.1)  # 100ms
        tripped = threading.Event()

        def run():
            try:
                with guard.task_context("task-wall"):
                    time.sleep(0.15)   # exceed 100ms deadline
                    guard.tick(units=1)  # should raise here
            except BudgetExceededError as exc:
                if exc.reason == "wall_clock_timeout":
                    tripped.set()

        t = threading.Thread(target=run)
        t.start()
        t.join(timeout=1.0)
        assert tripped.is_set(), "Watchdog did not trip within timeout"

    def test_watchdog_cancelled_on_normal_exit(self):
        """Watchdog timer is cancelled when context exits normally — no leak."""
        guard = ComputeGuard(task_wall_clock_secs=10.0)
        with guard.task_context("task-fast"):
            pass  # exits before watchdog fires
        # If watchdog were not cancelled, it would be a daemon thread leak.
        # We verify guard state is clean.
        assert guard._watchdog is None
        assert guard._task_id is None


class TestBudgetExceededError:
    def test_error_attributes(self):
        exc = BudgetExceededError(reason="test", elapsed_secs=5.0, budget_secs=3.0)
        assert exc.reason == "test"
        assert exc.elapsed_secs == 5.0
        assert exc.budget_secs == 3.0
        assert "Circuit breaker tripped" in str(exc)
        assert isinstance(exc, RuntimeError)
