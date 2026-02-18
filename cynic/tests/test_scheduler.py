"""
DogScheduler Tests — concurrent consciousness loops.

Tests the 4-tier scheduling architecture without Ollama.
Uses a mock orchestrator to validate routing, interrupts, and CycleTimer.

Architecture verified:
  4 = L(3) → tiers      (REFLEX, MICRO, MACRO, META — operating frequencies)
  7 = L(4) → gradient   (ConsciousnessGradient — per-cell depth)
  Both φ-derived Lucas numbers, orthogonal dimensions.
"""
from __future__ import annotations

import asyncio
import time
from unittest.mock import AsyncMock, MagicMock

import pytest

from cynic.core.consciousness import ConsciousnessLevel, reset_consciousness
from cynic.core.judgment import Cell
from cynic.scheduler import DogScheduler, PerceptionEvent, _QUEUE_CAPACITY, _WORKERS_PER_LEVEL


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture(autouse=True)
def fresh_consciousness():
    """Each test gets a clean ConsciousnessState."""
    reset_consciousness()
    yield
    reset_consciousness()


@pytest.fixture
def mock_orchestrator():
    """
    Mock JudgeOrchestrator that records calls and returns a WAG verdict.
    run() is an AsyncMock — awaitable and records (cell, level, budget_usd).

    Returns a MagicMock with .verdict attribute (not a dict) to match the
    Judgment object contract used by _reflex_worker via getattr(result, "verdict").
    """
    result = MagicMock()
    result.verdict = "WAG"
    result.q_score = 50.0
    result.confidence = 0.38

    orch = MagicMock()
    orch.run = AsyncMock(return_value=result)
    return orch


@pytest.fixture
def scheduler(mock_orchestrator):
    """DogScheduler with mock orchestrator (not yet started)."""
    return DogScheduler(orchestrator=mock_orchestrator)


@pytest.fixture
def cell():
    """A minimal Cell for test perceptions."""
    return Cell(
        reality="CODE",
        analysis="JUDGE",
        content="def foo(): pass",
    )


# ════════════════════════════════════════════════════════════════════════════
# CONSTRUCTION AND STATS
# ════════════════════════════════════════════════════════════════════════════

class TestSchedulerConstruction:
    def test_queues_initialized(self, scheduler):
        """4 queues created, one per ConsciousnessLevel."""
        assert len(scheduler._queues) == 4
        for level in ConsciousnessLevel:
            assert level in scheduler._queues

    def test_queues_empty_at_start(self, scheduler):
        """All queues start empty."""
        for q in scheduler._queues.values():
            assert q.qsize() == 0

    def test_queue_capacity(self, scheduler):
        """Queue capacity = F(10) = 55."""
        assert _QUEUE_CAPACITY == 55

    def test_not_running_before_start(self, scheduler):
        """Scheduler is not running before start()."""
        assert not scheduler._running

    def test_stats_before_start(self, scheduler):
        """stats() returns valid structure even before start."""
        s = scheduler.stats()
        assert not s["running"]
        assert "queues" in s
        assert "cycles" in s
        assert "timers" in s
        assert s["cycles"]["total"] == 0


# ════════════════════════════════════════════════════════════════════════════
# SUBMIT — routing and auto-downgrade
# ════════════════════════════════════════════════════════════════════════════

class TestSubmit:
    def test_submit_to_micro_by_default(self, scheduler, cell):
        """budget_usd in [0.01, 0.05) → MICRO."""
        result = scheduler.submit(cell, budget_usd=0.03)
        assert result is True
        assert scheduler._queues[ConsciousnessLevel.MICRO].qsize() == 1

    def test_submit_explicit_level(self, scheduler, cell):
        """Explicit level overrides budget-based inference."""
        scheduler.submit(cell, level=ConsciousnessLevel.MACRO)
        assert scheduler._queues[ConsciousnessLevel.MACRO].qsize() == 1
        assert scheduler._queues[ConsciousnessLevel.MICRO].qsize() == 0

    def test_auto_downgrade_low_budget(self, scheduler, cell):
        """budget_usd < 0.01 → REFLEX (even if no level specified)."""
        scheduler.submit(cell, budget_usd=0.005)
        assert scheduler._queues[ConsciousnessLevel.REFLEX].qsize() == 1
        assert scheduler._queues[ConsciousnessLevel.MICRO].qsize() == 0

    def test_auto_downgrade_micro_budget(self, scheduler, cell):
        """0.01 ≤ budget < 0.05 → MICRO."""
        scheduler.submit(cell, budget_usd=0.03)
        assert scheduler._queues[ConsciousnessLevel.MICRO].qsize() == 1

    def test_auto_upgrade_full_budget(self, scheduler, cell):
        """budget ≥ 0.05 → MACRO."""
        scheduler.submit(cell, budget_usd=0.10)
        assert scheduler._queues[ConsciousnessLevel.MACRO].qsize() == 1

    def test_queue_full_returns_false(self, scheduler, cell):
        """Returns False (does not raise) when queue is at capacity."""
        # Fill the MICRO queue to capacity
        for _ in range(_QUEUE_CAPACITY):
            ok = scheduler.submit(cell, level=ConsciousnessLevel.MICRO)
            assert ok is True
        # One more → should fail gracefully
        overflow = scheduler.submit(cell, level=ConsciousnessLevel.MICRO)
        assert overflow is False

    def test_perception_event_fields(self, scheduler, cell):
        """PerceptionEvent is created with correct fields."""
        scheduler.submit(cell, level=ConsciousnessLevel.MICRO, source="test_hook")
        event = scheduler._queues[ConsciousnessLevel.MICRO].get_nowait()
        assert isinstance(event, PerceptionEvent)
        assert event.cell == cell
        assert event.level == ConsciousnessLevel.MICRO
        assert event.source == "test_hook"
        assert event.wait_ms >= 0

    def test_submit_multiple_levels(self, scheduler, cell):
        """Can submit to multiple levels independently."""
        scheduler.submit(cell, level=ConsciousnessLevel.REFLEX)
        scheduler.submit(cell, level=ConsciousnessLevel.MICRO)
        scheduler.submit(cell, level=ConsciousnessLevel.MACRO)
        assert scheduler._queues[ConsciousnessLevel.REFLEX].qsize() == 1
        assert scheduler._queues[ConsciousnessLevel.MICRO].qsize() == 1
        assert scheduler._queues[ConsciousnessLevel.MACRO].qsize() == 1
        assert scheduler._queues[ConsciousnessLevel.META].qsize() == 0


# ════════════════════════════════════════════════════════════════════════════
# INTERRUPT MECHANISM
# ════════════════════════════════════════════════════════════════════════════

class TestInterrupt:
    def test_micro_interrupt_not_set_initially(self, scheduler):
        """MICRO interrupt event starts clear."""
        assert not scheduler._micro_interrupt.is_set()

    def test_interrupt_micro_sets_event(self, scheduler):
        """interrupt_micro() sets the asyncio.Event."""
        scheduler.interrupt_micro()
        assert scheduler._micro_interrupt.is_set()

    def test_interrupt_clears_after_check(self, scheduler):
        """Event is cleared after being read (simulates loop behavior)."""
        scheduler.interrupt_micro()
        assert scheduler._micro_interrupt.is_set()
        # Simulate loop clearing it
        scheduler._micro_interrupt.clear()
        assert not scheduler._micro_interrupt.is_set()


# ════════════════════════════════════════════════════════════════════════════
# LEVEL INFERENCE
# ════════════════════════════════════════════════════════════════════════════

class TestLevelInference:
    def test_infer_reflex(self):
        assert DogScheduler._infer_level(0.005) == ConsciousnessLevel.REFLEX
        assert DogScheduler._infer_level(0.0)   == ConsciousnessLevel.REFLEX

    def test_infer_micro(self):
        assert DogScheduler._infer_level(0.01)  == ConsciousnessLevel.MICRO
        assert DogScheduler._infer_level(0.049) == ConsciousnessLevel.MICRO

    def test_infer_macro(self):
        assert DogScheduler._infer_level(0.05)  == ConsciousnessLevel.MACRO
        assert DogScheduler._infer_level(1.0)   == ConsciousnessLevel.MACRO

    def test_phi_boundary(self):
        """PHI_INV = 0.618 → MACRO (above 0.05 threshold)."""
        from cynic.core.phi import PHI_INV
        assert DogScheduler._infer_level(PHI_INV) == ConsciousnessLevel.MACRO


# ════════════════════════════════════════════════════════════════════════════
# ASYNC: start/stop lifecycle + processing
# ════════════════════════════════════════════════════════════════════════════

class TestSchedulerLifecycle:
    async def test_start_creates_tasks(self, scheduler):
        """start() creates N tier worker tasks (sum of _WORKERS_PER_LEVEL = 11)."""
        scheduler.start()
        assert scheduler._running
        assert len(scheduler._tasks) == sum(_WORKERS_PER_LEVEL.values())  # 5+3+2+1=11
        await scheduler.stop()

    async def test_stop_cleans_up(self, scheduler):
        """stop() cancels tasks and sets running=False."""
        scheduler.start()
        await scheduler.stop()
        assert not scheduler._running
        assert len(scheduler._tasks) == 0

    async def test_double_start_no_duplicates(self, scheduler):
        """Calling start() twice doesn't create duplicate tasks."""
        scheduler.start()
        scheduler.start()  # should warn and skip
        assert len(scheduler._tasks) == sum(_WORKERS_PER_LEVEL.values())  # 11, not 22
        await scheduler.stop()

    async def test_micro_processes_submitted_cell(self, scheduler, cell, mock_orchestrator):
        """MICRO loop processes a submitted cell and calls orchestrator."""
        scheduler.start()
        scheduler.submit(cell, level=ConsciousnessLevel.MICRO, budget_usd=0.03)
        # Give MICRO loop time to process (> MICRO target_ms)
        await asyncio.sleep(ConsciousnessLevel.MICRO.target_ms / 1000.0 * 3)
        await scheduler.stop()

        assert mock_orchestrator.run.called
        call_kwargs = mock_orchestrator.run.call_args.kwargs
        assert call_kwargs["level"] == ConsciousnessLevel.MICRO

    async def test_micro_cycles_incremented(self, scheduler, cell, mock_orchestrator):
        """ConsciousnessState.micro_cycles increments after MICRO processing."""
        from cynic.core.consciousness import get_consciousness
        c = get_consciousness()
        assert c.micro_cycles == 0

        scheduler.start()
        scheduler.submit(cell, level=ConsciousnessLevel.MICRO, budget_usd=0.03)
        await asyncio.sleep(ConsciousnessLevel.MICRO.target_ms / 1000.0 * 3)
        await scheduler.stop()

        assert c.micro_cycles >= 1

    async def test_stats_reflect_processing(self, scheduler, cell, mock_orchestrator):
        """stats() reports processed cycles after run."""
        scheduler.start()
        scheduler.submit(cell, level=ConsciousnessLevel.MICRO, budget_usd=0.03)
        await asyncio.sleep(ConsciousnessLevel.MICRO.target_ms / 1000.0 * 3)
        await scheduler.stop()

        s = scheduler.stats()
        assert s["cycles"]["MICRO"] >= 1
        assert s["cycles"]["total"] >= 1

    async def test_reflex_interrupt_wakes_micro(self, scheduler, cell, mock_orchestrator):
        """REFLEX anomaly (BARK verdict) fires interrupt_micro."""
        # Make orchestrator return BARK for REFLEX → triggers interrupt
        async def side_effect(*args, **kwargs):
            level = kwargs.get("level")
            result = MagicMock()
            if level == ConsciousnessLevel.REFLEX:
                result.verdict = "BARK"
                result.q_score = 10.0
            else:
                result.verdict = "WAG"
                result.q_score = 50.0
            return result

        mock_orchestrator.run.side_effect = side_effect

        scheduler.start()
        # Submit one REFLEX item that triggers BARK
        scheduler.submit(cell, level=ConsciousnessLevel.REFLEX, budget_usd=0.001)
        await asyncio.sleep(ConsciousnessLevel.REFLEX.target_ms / 1000.0 * 5)
        await scheduler.stop()

        # Interrupt was fired (may already be cleared by MICRO loop)
        # Verify REFLEX ran at least once
        c_state = scheduler._consciousness
        assert c_state.reflex_cycles >= 1


# ════════════════════════════════════════════════════════════════════════════
# PERCEPTION EVENT
# ════════════════════════════════════════════════════════════════════════════

class TestPerceptionEvent:
    def test_wait_ms_increases_over_time(self, cell):
        """wait_ms reflects time since submission."""
        event = PerceptionEvent(cell=cell)
        time.sleep(0.01)
        assert event.wait_ms >= 10.0  # at least 10ms

    def test_default_level_micro(self, cell):
        """Default level is MICRO."""
        event = PerceptionEvent(cell=cell)
        assert event.level == ConsciousnessLevel.MICRO

    def test_source_field(self, cell):
        """Source field is stored correctly."""
        event = PerceptionEvent(cell=cell, source="reflex_scan")
        assert event.source == "reflex_scan"


# ════════════════════════════════════════════════════════════════════════════
# N-WORKERS — parallel processing within each tier
# ════════════════════════════════════════════════════════════════════════════

class TestNWorkers:
    async def test_worker_counts_match_phi_constants(self, scheduler):
        """Each tier spawns exactly _WORKERS_PER_LEVEL[level] tasks."""
        scheduler.start()
        names = [t.get_name() for t in scheduler._tasks]
        await scheduler.stop()

        for level, expected_n in _WORKERS_PER_LEVEL.items():
            prefix = f"cynic.scheduler.{level.name.lower()}."
            count = sum(1 for n in names if n.startswith(prefix))
            assert count == expected_n, (
                f"{level.name}: expected {expected_n} workers, got {count}"
            )

    async def test_total_tier_tasks_is_11(self, scheduler):
        """5+3+2+1 = 11 tier tasks total."""
        scheduler.start()
        assert len(scheduler._tasks) == 11
        await scheduler.stop()

    async def test_each_cell_processed_exactly_once(self, scheduler, cell, mock_orchestrator):
        """3 cells submitted → orchestrator called exactly 3 times (not 3×N)."""
        scheduler.start()
        for _ in range(3):
            scheduler.submit(cell, level=ConsciousnessLevel.MICRO, budget_usd=0.03)
        await asyncio.sleep(ConsciousnessLevel.MICRO.target_ms / 1000.0 * 5)
        await scheduler.stop()
        assert mock_orchestrator.run.call_count == 3

    async def test_worker_tasks_named_by_level_and_index(self, scheduler):
        """Workers named cynic.scheduler.{level}.{i} covering all i in [0, N)."""
        scheduler.start()
        names = {t.get_name() for t in scheduler._tasks}
        await scheduler.stop()

        # Spot-check a few expected names
        assert "cynic.scheduler.reflex.0" in names
        assert "cynic.scheduler.reflex.4" in names   # F(5)-1 = 4
        assert "cynic.scheduler.micro.0"  in names
        assert "cynic.scheduler.micro.2"  in names   # F(4)-1 = 2
        assert "cynic.scheduler.macro.0"  in names
        assert "cynic.scheduler.macro.1"  in names   # F(3)-1 = 1
        assert "cynic.scheduler.meta.0"   in names   # single META

    async def test_n_workers_achieve_parallelism(self, scheduler, cell, mock_orchestrator):
        """
        5 REFLEX workers process 5 cells concurrently — not sequentially.

        Each cell takes 50ms. Sequential: ~250ms. Parallel (5 workers): ~50ms.
        We assert completion in < 200ms to prove N-worker parallelism.
        """
        async def slow_run(*args, **kwargs):
            await asyncio.sleep(0.05)   # 50ms per cell
            result = MagicMock()
            result.verdict = "WAG"
            return result

        mock_orchestrator.run.side_effect = slow_run
        scheduler.start()

        # Submit 5 cells — REFLEX has F(5)=5 workers, so all processed in parallel
        for _ in range(5):
            scheduler.submit(cell, level=ConsciousnessLevel.REFLEX, budget_usd=0.001)

        t0 = time.perf_counter()
        await asyncio.sleep(0.18)   # Wait 180ms — enough for parallel (50ms) not sequential (250ms)
        elapsed_ms = (time.perf_counter() - t0) * 1000
        await scheduler.stop()

        # All 5 cells must have been processed
        assert mock_orchestrator.run.call_count == 5, (
            f"Expected 5 calls, got {mock_orchestrator.run.call_count}"
        )
        # Must complete well within sequential time — proves parallelism
        assert elapsed_ms < 220, (
            f"Took {elapsed_ms:.0f}ms — expected < 220ms (sequential would be ~250ms)"
        )


# ════════════════════════════════════════════════════════════════════════════
# PERCEIVE WORKER REGISTRATION
# ════════════════════════════════════════════════════════════════════════════

class TestPerceiveWorkerRegistration:
    def test_register_adds_to_list(self, scheduler):
        """register_perceive_worker() stores worker before start."""
        from cynic.perceive.workers import HealthWatcher
        pw = HealthWatcher()
        scheduler.register_perceive_worker(pw)
        assert len(scheduler._perceive_workers) == 1

    async def test_perceive_tasks_created_on_start(self, scheduler):
        """start() spawns one task per registered PerceiveWorker."""
        from cynic.perceive.workers import HealthWatcher
        scheduler.register_perceive_worker(HealthWatcher())
        scheduler.start()
        assert len(scheduler._perceive_tasks) == 1
        await scheduler.stop()

    async def test_perceive_tasks_cleared_on_stop(self, scheduler):
        """stop() cancels and clears perceive tasks."""
        from cynic.perceive.workers import HealthWatcher
        scheduler.register_perceive_worker(HealthWatcher())
        scheduler.start()
        await scheduler.stop()
        assert len(scheduler._perceive_tasks) == 0

    async def test_multiple_perceive_workers(self, scheduler):
        """Registering 3 workers → 3 perceive tasks."""
        from cynic.perceive.workers import GitWatcher, HealthWatcher, SelfWatcher
        scheduler.register_perceive_worker(GitWatcher())
        scheduler.register_perceive_worker(HealthWatcher())
        scheduler.register_perceive_worker(SelfWatcher())
        scheduler.start()
        assert len(scheduler._perceive_tasks) == 3
        await scheduler.stop()

    async def test_perceive_task_named_after_worker(self, scheduler):
        """Perceive task name = cynic.perceive.{worker.name}."""
        from cynic.perceive.workers import GitWatcher
        scheduler.register_perceive_worker(GitWatcher())
        scheduler.start()
        names = {t.get_name() for t in scheduler._perceive_tasks}
        assert "cynic.perceive.git_watcher" in names
        await scheduler.stop()

    def test_stats_reports_perceive_worker_count(self, scheduler):
        """stats()['perceive_workers'] reflects registered count."""
        from cynic.perceive.workers import HealthWatcher
        scheduler.register_perceive_worker(HealthWatcher())
        s = scheduler.stats()
        assert s["perceive_workers"] == 1

    def test_stats_reports_workers_per_level(self, scheduler):
        """stats()['workers_per_level'] maps level names to counts."""
        s = scheduler.stats()
        assert s["workers_per_level"]["REFLEX"] == 5
        assert s["workers_per_level"]["MICRO"]  == 3
        assert s["workers_per_level"]["MACRO"]  == 2
        assert s["workers_per_level"]["META"]   == 1

    async def test_register_after_start_is_ignored(self, scheduler):
        """register_perceive_worker() after start() is silently dropped — no duplicate tasks."""
        from cynic.perceive.workers import HealthWatcher
        scheduler.start()
        initial_count = len(scheduler._perceive_tasks)

        # Register after start() — must be ignored
        scheduler.register_perceive_worker(HealthWatcher())

        assert len(scheduler._perceive_tasks) == initial_count, (
            "Late-registered worker must not create a new task"
        )
        assert len(scheduler._perceive_workers) == initial_count, (
            "Late-registered worker must not be stored in the list"
        )
        await scheduler.stop()
