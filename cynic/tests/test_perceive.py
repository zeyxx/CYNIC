"""
CYNIC PerceiveWorker Tests

Unit tests for all 3 autonomous sensory workers:
  GitWatcher     — git change detection, dedup
  HealthWatcher  — timer degradation monitoring
  SelfWatcher    — Q-Table learning health check

No real git, no real DB, no real timers — all mocked.
Also tests PerceiveWorker.run() base loop behavior.
"""
from __future__ import annotations

import asyncio
import subprocess
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.core.consciousness import ConsciousnessLevel, reset_consciousness
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci
from cynic.perceive.workers import GitWatcher, HealthWatcher, SelfWatcher


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture(autouse=True)
def fresh_consciousness():
    reset_consciousness()
    yield
    reset_consciousness()


# ════════════════════════════════════════════════════════════════════════════
# GitWatcher
# ════════════════════════════════════════════════════════════════════════════

class TestGitWatcher:
    @pytest.mark.asyncio
    async def test_returns_none_when_no_changes(self):
        """Clean working tree (empty stdout) → sense() returns None."""
        watcher = GitWatcher()
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="")
            cell = await watcher.sense()
        assert cell is None

    @pytest.mark.asyncio
    async def test_returns_cell_on_uncommitted_changes(self):
        """Uncommitted files detected → returns CODE×PERCEIVE Cell."""
        watcher = GitWatcher()
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout=" M foo.py\n?? bar.py\n",
            )
            cell = await watcher.sense()
        assert cell is not None
        assert isinstance(cell, Cell)
        assert cell.reality == "CODE"
        assert cell.analysis == "PERCEIVE"
        assert cell.content["changed_files"] == 2
        assert "foo.py" in cell.content["git_status"]

    @pytest.mark.asyncio
    async def test_dedup_same_output(self):
        """Same git output on second call → returns None (no new info)."""
        watcher = GitWatcher()
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout=" M foo.py\n")
            cell1 = await watcher.sense()
            cell2 = await watcher.sense()
        assert cell1 is not None
        assert cell2 is None

    @pytest.mark.asyncio
    async def test_new_changes_break_dedup(self):
        """Different git output after dedup → third call returns Cell again."""
        watcher = GitWatcher()
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout=" M foo.py\n")
            await watcher.sense()   # Cell
            await watcher.sense()   # None (dup)
            mock_run.return_value = MagicMock(returncode=0, stdout=" M foo.py\nM bar.py\n")
            cell3 = await watcher.sense()
        assert cell3 is not None
        assert cell3.content["changed_files"] == 2

    @pytest.mark.asyncio
    async def test_clean_tree_resets_hash(self):
        """Clean tree after changes → _last_hash is reset to None."""
        watcher = GitWatcher()
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout=" M foo.py\n")
            await watcher.sense()
            assert watcher._last_hash is not None
            # Now clean
            mock_run.return_value = MagicMock(returncode=0, stdout="")
            await watcher.sense()
            assert watcher._last_hash is None

    @pytest.mark.asyncio
    async def test_returns_none_on_nonzero_returncode(self):
        """Non-zero returncode (git error) → returns None."""
        watcher = GitWatcher()
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=128, stdout="fatal error")
            cell = await watcher.sense()
        assert cell is None

    @pytest.mark.asyncio
    async def test_returns_none_on_timeout(self):
        """subprocess.TimeoutExpired → returns None gracefully."""
        watcher = GitWatcher()
        with patch("subprocess.run", side_effect=subprocess.TimeoutExpired("git", 3.0)):
            cell = await watcher.sense()
        assert cell is None

    @pytest.mark.asyncio
    async def test_returns_none_on_file_not_found(self):
        """FileNotFoundError (git not on PATH) → returns None."""
        watcher = GitWatcher()
        with patch("subprocess.run", side_effect=FileNotFoundError("git")):
            cell = await watcher.sense()
        assert cell is None

    def test_cell_complexity_scales_with_file_count(self):
        """complexity = min(file_count / 50, 1.0) — more files → higher complexity."""
        import asyncio
        watcher = GitWatcher()
        # 50 files → complexity = 1.0 (capped)
        big_output = "\n".join(f"M file{i}.py" for i in range(50))
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout=big_output)
            cell = asyncio.get_event_loop().run_until_complete(watcher.sense())
        assert cell is not None
        assert cell.complexity == 1.0

    def test_level_is_reflex(self):
        assert GitWatcher.level == ConsciousnessLevel.REFLEX

    def test_name_is_git_watcher(self):
        assert GitWatcher.name == "git_watcher"

    def test_interval_fibonacci_5(self):
        assert GitWatcher.interval_s == float(fibonacci(5))  # 5.0s

    def test_budget_is_minimal(self):
        """GitWatcher uses minimal $0.001 budget (non-LLM)."""
        assert GitWatcher().interval_s > 0
        # Inspect the hardcoded budget in a fresh sense() result
        import asyncio
        watcher = GitWatcher()
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="M f.py\n")
            cell = asyncio.get_event_loop().run_until_complete(watcher.sense())
        assert cell is not None
        assert cell.budget_usd == 0.001


# ════════════════════════════════════════════════════════════════════════════
# HealthWatcher
# ════════════════════════════════════════════════════════════════════════════

def _make_consciousness(health: str, *, cycle_count: int = 42):
    """Fake ConsciousnessState with one timer at the given health."""
    timer = MagicMock()
    timer.health = health
    timer.p95_ms = 100.0
    timer.to_dict = lambda: {"health": health, "p95_ms": 100.0}

    consciousness = MagicMock()
    consciousness.timers = {"MACRO": timer}
    consciousness.total_cycles = cycle_count
    return consciousness


class TestHealthWatcher:
    @pytest.mark.asyncio
    async def test_returns_none_when_all_healthy(self):
        """All timers GOOD → sense() returns None (no news is good news)."""
        consciousness = _make_consciousness("GOOD")
        watcher = HealthWatcher(get_consciousness_fn=lambda: consciousness)
        cell = await watcher.sense()
        assert cell is None

    @pytest.mark.asyncio
    async def test_returns_none_when_excellent(self):
        """EXCELLENT timer → also returns None."""
        consciousness = _make_consciousness("EXCELLENT")
        watcher = HealthWatcher(get_consciousness_fn=lambda: consciousness)
        cell = await watcher.sense()
        assert cell is None

    @pytest.mark.asyncio
    async def test_returns_cell_when_degraded(self):
        """DEGRADED timer → returns CYNIC×PERCEIVE Cell."""
        consciousness = _make_consciousness("DEGRADED")
        watcher = HealthWatcher(get_consciousness_fn=lambda: consciousness)
        cell = await watcher.sense()
        assert cell is not None
        assert cell.reality == "CYNIC"
        assert cell.analysis == "PERCEIVE"
        assert cell.content["worst_health"] == "DEGRADED"
        assert cell.metadata["degraded_count"] == 1

    @pytest.mark.asyncio
    async def test_returns_cell_when_critical(self):
        """CRITICAL timer → returns Cell (highest severity)."""
        consciousness = _make_consciousness("CRITICAL")
        watcher = HealthWatcher(get_consciousness_fn=lambda: consciousness)
        cell = await watcher.sense()
        assert cell is not None
        assert cell.content["worst_health"] == "CRITICAL"

    @pytest.mark.asyncio
    async def test_risk_0_2_for_degraded(self):
        """DEGRADED → risk=0.2 (moderate concern)."""
        consciousness = _make_consciousness("DEGRADED")
        watcher = HealthWatcher(get_consciousness_fn=lambda: consciousness)
        cell = await watcher.sense()
        assert cell.risk == pytest.approx(0.2)

    @pytest.mark.asyncio
    async def test_risk_0_5_for_critical(self):
        """CRITICAL → risk=0.5 (serious concern)."""
        consciousness = _make_consciousness("CRITICAL")
        watcher = HealthWatcher(get_consciousness_fn=lambda: consciousness)
        cell = await watcher.sense()
        assert cell.risk == pytest.approx(0.5)

    @pytest.mark.asyncio
    async def test_total_cycles_in_content(self):
        """total_cycles included in cell content."""
        consciousness = _make_consciousness("DEGRADED", cycle_count=123)
        watcher = HealthWatcher(get_consciousness_fn=lambda: consciousness)
        cell = await watcher.sense()
        assert cell.content["total_cycles"] == 123

    def test_level_is_reflex(self):
        assert HealthWatcher.level == ConsciousnessLevel.REFLEX

    def test_name_is_health_watcher(self):
        assert HealthWatcher.name == "health_watcher"

    def test_interval_fibonacci_8(self):
        assert HealthWatcher.interval_s == float(fibonacci(8))  # 21.0s


# ════════════════════════════════════════════════════════════════════════════
# SelfWatcher
# ════════════════════════════════════════════════════════════════════════════

class TestSelfWatcher:
    @pytest.mark.asyncio
    async def test_returns_none_without_qtable_getter(self):
        """No qtable_getter provided → sense() returns None."""
        watcher = SelfWatcher(qtable_getter=None)
        cell = await watcher.sense()
        assert cell is None

    @pytest.mark.asyncio
    async def test_returns_cell_with_qtable_stats(self):
        """Valid qtable_getter → returns CYNIC×LEARN Cell with Q-table stats."""
        qtable = MagicMock()
        qtable.stats.return_value = {
            "states": 10,
            "total_updates": 42,
            "pending_flush": 3,
            "max_confidence": 0.5,
            "unique_states": 8,
        }
        watcher = SelfWatcher(qtable_getter=lambda: qtable)
        cell = await watcher.sense()
        assert cell is not None
        assert cell.reality == "CYNIC"
        assert cell.analysis == "LEARN"
        assert cell.content["states"] == 10
        assert cell.content["total_updates"] == 42
        assert cell.content["pending_flush"] == 3

    @pytest.mark.asyncio
    async def test_returns_none_when_qtable_raises(self):
        """Exception from qtable.stats() → returns None gracefully."""
        def broken_getter():
            raise RuntimeError("Q-table corrupted")

        watcher = SelfWatcher(qtable_getter=broken_getter)
        cell = await watcher.sense()
        assert cell is None

    @pytest.mark.asyncio
    async def test_returns_none_when_getter_returns_bad_object(self):
        """stats() raises AttributeError (wrong type) → returns None."""
        watcher = SelfWatcher(qtable_getter=lambda: "not_a_qtable")
        cell = await watcher.sense()
        assert cell is None

    @pytest.mark.asyncio
    async def test_context_reflects_state_count(self):
        """context string includes state count from stats."""
        qtable = MagicMock()
        qtable.stats.return_value = {
            "states": 7, "total_updates": 21,
            "pending_flush": 0, "max_confidence": 0.3, "unique_states": 5,
        }
        watcher = SelfWatcher(qtable_getter=lambda: qtable)
        cell = await watcher.sense()
        assert "7" in cell.context       # states count in context
        assert "21" in cell.context      # total_updates in context

    def test_level_is_micro(self):
        assert SelfWatcher.level == ConsciousnessLevel.MICRO

    def test_name_is_self_watcher(self):
        assert SelfWatcher.name == "self_watcher"

    def test_interval_fibonacci_10(self):
        assert SelfWatcher.interval_s == float(fibonacci(10))  # 55.0s

    def test_budget_higher_than_git_watcher(self):
        """SelfWatcher uses $0.003 (slightly more than GitWatcher's $0.001)."""
        # Inspect via a successful sense() call
        qtable = MagicMock()
        qtable.stats.return_value = {
            "states": 1, "total_updates": 1, "pending_flush": 0,
            "max_confidence": 0.1, "unique_states": 1,
        }
        import asyncio
        watcher = SelfWatcher(qtable_getter=lambda: qtable)
        cell = asyncio.get_event_loop().run_until_complete(watcher.sense())
        assert cell is not None
        assert cell.budget_usd == 0.003


# ════════════════════════════════════════════════════════════════════════════
# PerceiveWorker.run() — base loop behavior
# ════════════════════════════════════════════════════════════════════════════

class TestPerceiveWorkerRun:
    @pytest.mark.asyncio
    async def test_run_calls_submit_fn_with_cell(self):
        """run() calls submit_fn with correct kwargs when sense() returns a Cell."""
        cell = Cell(
            reality="CODE", analysis="PERCEIVE", content="test", time_dim="PRESENT",
            budget_usd=0.001,
        )

        call_count = 0
        watcher = GitWatcher()
        watcher.interval_s = 0.001  # near-zero sleep

        async def fake_sense():
            nonlocal call_count
            call_count += 1
            if call_count == 1:
                return cell
            await asyncio.sleep(10)   # Block until task cancelled

        watcher.sense = fake_sense
        submit_fn = MagicMock(return_value=True)

        task = asyncio.ensure_future(watcher.run(submit_fn))
        await asyncio.sleep(0.05)
        task.cancel()
        try:
            await task
        except asyncio.CancelledError:
            pass

        assert submit_fn.called
        kwargs = submit_fn.call_args.kwargs
        assert kwargs["level"] == ConsciousnessLevel.REFLEX
        assert kwargs["source"] == "git_watcher"

    @pytest.mark.asyncio
    async def test_run_skips_submit_when_sense_returns_none(self):
        """run() does NOT call submit_fn when sense() returns None."""
        submit_fn = MagicMock(return_value=True)
        watcher = HealthWatcher()
        watcher.interval_s = 0.001

        async def fake_sense():
            return None

        watcher.sense = fake_sense

        task = asyncio.ensure_future(watcher.run(submit_fn))
        await asyncio.sleep(0.05)
        task.cancel()
        try:
            await task
        except asyncio.CancelledError:
            pass

        assert not submit_fn.called

    @pytest.mark.asyncio
    async def test_run_exits_cleanly_on_cancelled_error_in_sense(self):
        """CancelledError raised inside sense() → run() exits cleanly."""
        watcher = GitWatcher()

        async def fake_sense():
            raise asyncio.CancelledError()

        watcher.sense = fake_sense

        task = asyncio.ensure_future(watcher.run(MagicMock()))
        await asyncio.sleep(0.05)

        # Task should have terminated on its own
        assert task.done()

    @pytest.mark.asyncio
    async def test_run_survives_non_cancelled_exception(self):
        """ValueError in sense() is caught; run() continues on next iteration."""
        call_count = 0

        async def unreliable_sense():
            nonlocal call_count
            call_count += 1
            if call_count < 3:
                raise ValueError("temporary failure")
            await asyncio.sleep(10)   # Prevent further calls; cancelled here

        watcher = GitWatcher()
        watcher.sense = unreliable_sense
        watcher.interval_s = 0.001

        task = asyncio.ensure_future(watcher.run(MagicMock()))
        await asyncio.sleep(0.05)
        task.cancel()
        try:
            await task
        except asyncio.CancelledError:
            pass

        assert call_count >= 3   # Continued despite errors

    @pytest.mark.asyncio
    async def test_run_logs_dropped_cell(self):
        """submit_fn returning False (queue full) is handled gracefully."""
        cell = Cell(
            reality="CODE", analysis="PERCEIVE", content="x", time_dim="PRESENT",
            budget_usd=0.001,
        )
        submit_fn = MagicMock(return_value=False)   # Queue full!

        call_count = 0
        watcher = GitWatcher()
        watcher.interval_s = 0.001

        async def fake_sense():
            nonlocal call_count
            call_count += 1
            if call_count == 1:
                return cell
            await asyncio.sleep(10)

        watcher.sense = fake_sense

        task = asyncio.ensure_future(watcher.run(submit_fn))
        await asyncio.sleep(0.05)
        task.cancel()
        try:
            await task
        except asyncio.CancelledError:
            pass

        # submit_fn was called even though it returned False — no crash
        assert submit_fn.called
