"""
Tests for ClaudeCodeRunner — CYNIC spawns Claude Code autonomously.

CYNIC is the BRAIN. Claude Code is the HANDS.
We test: instantiation, stats, shutdown, and key error paths.
The success path is tested via simulated bus events (no real claude binary needed).
"""
from __future__ import annotations

import asyncio
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.act.runner import ClaudeCodeRunner, CONNECT_TIMEOUT, DEFAULT_TASK_TIMEOUT
from cynic.core.event_bus import get_core_bus, Event, CoreEvent


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

def _mock_proc(returncode=None):
    """A mock asyncio subprocess.Process."""
    proc = MagicMock()
    proc.pid = 99999
    proc.returncode = returncode
    proc.terminate = MagicMock()
    proc.kill = MagicMock()
    proc.wait = AsyncMock(return_value=0)
    return proc


def _mock_session(model="claude-sonnet-4-6"):
    """A mock SDKSession with an async WebSocket."""
    session = MagicMock()
    session.model = model
    session.ws = AsyncMock()
    session.ws.send_text = AsyncMock()
    return session


# ════════════════════════════════════════════════════════════════════════════
# BASIC — no subprocess needed
# ════════════════════════════════════════════════════════════════════════════

class TestRunnerBasic:

    def test_runner_instantiation(self):
        bus = get_core_bus()
        runner = ClaudeCodeRunner(bus=bus, sessions_registry={}, port=8765)
        assert runner is not None

    def test_runner_stats_empty(self):
        bus = get_core_bus()
        runner = ClaudeCodeRunner(bus=bus, sessions_registry={}, port=8765)
        s = runner.stats()
        assert s["running_processes"] == 0
        assert s["sdk_port"] == 8765
        assert "/ws/sdk" in s["sdk_path"]

    def test_runner_custom_port(self):
        bus = get_core_bus()
        runner = ClaudeCodeRunner(bus=bus, sessions_registry={}, port=9999)
        assert runner.stats()["sdk_port"] == 9999

    @pytest.mark.asyncio
    async def test_runner_shutdown_no_processes(self):
        bus = get_core_bus()
        runner = ClaudeCodeRunner(bus=bus, sessions_registry={}, port=8765)
        await runner.shutdown()  # must not raise
        assert runner.stats()["running_processes"] == 0


# ════════════════════════════════════════════════════════════════════════════
# EXECUTE — error paths (no real claude binary)
# ════════════════════════════════════════════════════════════════════════════

class TestRunnerExecuteErrors:

    @pytest.mark.asyncio
    async def test_execute_binary_not_found(self):
        """If claude CLI is not installed → graceful failure with error dict."""
        bus = get_core_bus()
        runner = ClaudeCodeRunner(bus=bus, sessions_registry={}, port=8765)

        with patch("asyncio.create_subprocess_exec",
                   side_effect=FileNotFoundError("No such file: claude")):
            result = await runner.execute("write hello world")

        assert result["success"] is False
        assert "not found" in result["error"]
        assert "exec_id" in result

    @pytest.mark.asyncio
    async def test_execute_connect_timeout(self):
        """If claude spawns but never sends system/init → timeout returned."""
        bus = get_core_bus()
        runner = ClaudeCodeRunner(bus=bus, sessions_registry={}, port=8765)
        proc = _mock_proc()

        with patch("asyncio.create_subprocess_exec", AsyncMock(return_value=proc)):
            # Patch CONNECT_TIMEOUT to near-zero so the test doesn't wait 30s
            with patch("cynic.act.runner.CONNECT_TIMEOUT", 0.05):
                result = await runner.execute("test task")

        assert result["success"] is False
        assert result["error"] == "timeout"
        assert "exec_id" in result

    @pytest.mark.asyncio
    async def test_execute_no_lingering_processes_on_error(self):
        """_running dict is cleaned up even on FileNotFoundError."""
        bus = get_core_bus()
        runner = ClaudeCodeRunner(bus=bus, sessions_registry={}, port=8765)

        with patch("asyncio.create_subprocess_exec",
                   side_effect=FileNotFoundError):
            await runner.execute("test")

        assert runner.stats()["running_processes"] == 0

    @pytest.mark.asyncio
    async def test_execute_no_lingering_processes_on_timeout(self):
        """_running dict is cleaned up even on connect timeout."""
        bus = get_core_bus()
        runner = ClaudeCodeRunner(bus=bus, sessions_registry={}, port=8765)
        proc = _mock_proc()

        with patch("asyncio.create_subprocess_exec", AsyncMock(return_value=proc)):
            with patch("cynic.act.runner.CONNECT_TIMEOUT", 0.05):
                await runner.execute("test")

        assert runner.stats()["running_processes"] == 0

    @pytest.mark.asyncio
    async def test_execute_terminates_proc_on_timeout(self):
        """Subprocess is terminated after connect timeout."""
        bus = get_core_bus()
        runner = ClaudeCodeRunner(bus=bus, sessions_registry={}, port=8765)
        proc = _mock_proc()

        with patch("asyncio.create_subprocess_exec", AsyncMock(return_value=proc)):
            with patch("cynic.act.runner.CONNECT_TIMEOUT", 0.05):
                await runner.execute("test")

        proc.terminate.assert_called_once()


# ════════════════════════════════════════════════════════════════════════════
# EXECUTE — happy path via simulated bus events
# ════════════════════════════════════════════════════════════════════════════

class TestRunnerExecuteSuccess:

    @pytest.mark.asyncio
    async def test_execute_success_via_bus_events(self):
        """
        Simulate successful execution:
        1. Mock subprocess (stays alive)
        2. Fire SDK_SESSION_STARTED via bus (claude "connected")
        3. Fire SDK_RESULT_RECEIVED via bus (task "done")
        4. Assert runner returns success with correct data
        """
        bus = get_core_bus()
        sessions: dict = {}
        session_id = "runner-test-ok"
        session = _mock_session()
        proc = _mock_proc()

        runner = ClaudeCodeRunner(bus=bus, sessions_registry=sessions, port=8765)

        async def _fire_events():
            await asyncio.sleep(0.03)
            sessions[session_id] = session
            await bus.emit(Event(
                type=CoreEvent.SDK_SESSION_STARTED,
                payload={"session_id": session_id},
                source="test",
            ))
            await asyncio.sleep(0.03)
            await bus.emit(Event(
                type=CoreEvent.SDK_RESULT_RECEIVED,
                payload={
                    "session_id": session_id,
                    "is_error": False,
                    "cost_usd": 0.007,
                    "total_cost_usd": 0.007,
                },
                source="test",
            ))

        with patch("asyncio.create_subprocess_exec", AsyncMock(return_value=proc)):
            task = asyncio.create_task(runner.execute("Write hello world"))
            await _fire_events()
            result = await task

        assert result["success"] is True
        assert result["session_id"] == session_id
        assert result["cost_usd"] == pytest.approx(0.007, abs=1e-6)
        assert "exec_id" in result

    @pytest.mark.asyncio
    async def test_execute_sends_task_to_session_ws(self):
        """Task message is sent to the session WebSocket."""
        bus = get_core_bus()
        sessions: dict = {}
        session_id = "runner-ws-test"
        session = _mock_session()
        proc = _mock_proc()

        runner = ClaudeCodeRunner(bus=bus, sessions_registry=sessions, port=8765)

        async def _fire():
            await asyncio.sleep(0.02)
            sessions[session_id] = session
            await bus.emit(Event(
                type=CoreEvent.SDK_SESSION_STARTED,
                payload={"session_id": session_id},
                source="test",
            ))
            await asyncio.sleep(0.02)
            await bus.emit(Event(
                type=CoreEvent.SDK_RESULT_RECEIVED,
                payload={"session_id": session_id, "is_error": False, "cost_usd": 0.001},
                source="test",
            ))

        with patch("asyncio.create_subprocess_exec", AsyncMock(return_value=proc)):
            task = asyncio.create_task(runner.execute("hello prompt"))
            await _fire()
            await task

        # send_text must have been called with a JSON message containing "hello prompt"
        calls = [str(c) for c in session.ws.send_text.call_args_list]
        assert any("hello prompt" in c for c in calls)

    @pytest.mark.asyncio
    async def test_execute_error_result_success_false(self):
        """If Claude reports is_error=True → success=False."""
        bus = get_core_bus()
        sessions: dict = {}
        session_id = "runner-err-test"
        session = _mock_session()
        proc = _mock_proc()

        runner = ClaudeCodeRunner(bus=bus, sessions_registry=sessions, port=8765)

        async def _fire():
            await asyncio.sleep(0.02)
            sessions[session_id] = session
            await bus.emit(Event(
                type=CoreEvent.SDK_SESSION_STARTED,
                payload={"session_id": session_id},
                source="test",
            ))
            await asyncio.sleep(0.02)
            await bus.emit(Event(
                type=CoreEvent.SDK_RESULT_RECEIVED,
                payload={"session_id": session_id, "is_error": True, "cost_usd": 0.001},
                source="test",
            ))

        with patch("asyncio.create_subprocess_exec", AsyncMock(return_value=proc)):
            task = asyncio.create_task(runner.execute("bad task"))
            await _fire()
            result = await task

        assert result["success"] is False

    @pytest.mark.asyncio
    async def test_execute_cleans_up_after_success(self):
        """No lingering processes after successful execution."""
        bus = get_core_bus()
        sessions: dict = {}
        session_id = "cleanup-success"
        session = _mock_session()
        proc = _mock_proc()

        runner = ClaudeCodeRunner(bus=bus, sessions_registry=sessions, port=8765)

        async def _fire():
            await asyncio.sleep(0.02)
            sessions[session_id] = session
            await bus.emit(Event(
                type=CoreEvent.SDK_SESSION_STARTED,
                payload={"session_id": session_id},
                source="test",
            ))
            await asyncio.sleep(0.02)
            await bus.emit(Event(
                type=CoreEvent.SDK_RESULT_RECEIVED,
                payload={"session_id": session_id, "is_error": False, "cost_usd": 0.002},
                source="test",
            ))

        with patch("asyncio.create_subprocess_exec", AsyncMock(return_value=proc)):
            task = asyncio.create_task(runner.execute("task"))
            await _fire()
            await task

        assert runner.stats()["running_processes"] == 0
