"""Tests for DirectActionsHandler — real execution dispatch."""

import pytest
from unittest.mock import MagicMock, AsyncMock, patch

from cynic.api.handlers.direct import DirectActionsHandler
from cynic.api.handlers.base import KernelServices
from cynic.core.event_bus import Event, CoreEvent
from cynic.metabolism.universal import ActResult


class TestDirectActionsHandler:
    """DirectActionsHandler dispatches ACT_REQUESTED to UniversalActuator."""

    def test_init(self):
        """DirectActionsHandler initializes with dependencies."""
        svc = MagicMock(spec=KernelServices)
        ua = MagicMock()
        qtable = MagicMock()
        handler = DirectActionsHandler(svc, universal_actuator=ua, qtable=qtable)
        assert handler.name == "direct"
        assert handler._universal_actuator is ua
        assert handler._qtable is qtable

    def test_dependencies(self):
        """DirectActionsHandler declares correct dependencies."""
        svc = MagicMock(spec=KernelServices)
        handler = DirectActionsHandler(svc, universal_actuator=MagicMock(), qtable=MagicMock())
        deps = handler.dependencies()
        assert "escore_tracker" in deps
        assert "universal_actuator" in deps
        assert "qtable" in deps

    def test_subscriptions(self):
        """DirectActionsHandler subscribes to ACT_REQUESTED."""
        svc = MagicMock(spec=KernelServices)
        handler = DirectActionsHandler(svc, universal_actuator=MagicMock(), qtable=MagicMock())
        subs = handler.subscriptions()
        assert len(subs) == 1
        event_type, callback = subs[0]
        assert event_type == CoreEvent.ACT_REQUESTED
        assert callback == handler._on_act_requested

    @pytest.mark.asyncio
    async def test_on_act_requested_success(self):
        """_on_act_requested dispatches to UniversalActuator on success."""
        svc = MagicMock(spec=KernelServices)
        ua = MagicMock()
        qtable = MagicMock()

        # Mock UniversalActuator.dispatch to return success
        result = ActResult(
            action_type="bash",
            success=True,
            output="File modified successfully",
            duration_ms=123.4
        )
        ua.dispatch = AsyncMock(return_value=result)

        handler = DirectActionsHandler(svc, universal_actuator=ua, qtable=qtable)
        event = Event(
            type=CoreEvent.ACT_REQUESTED,
            payload={
                "action_type": "bash",
                "payload": {"args": ["touch", "test.txt"]},
            },
        )

        await handler._on_act_requested(event)

        # Verify dispatch was called
        ua.dispatch.assert_called_once_with("bash", {"args": ["touch", "test.txt"]})

        # Verify QTable.update was called with reward based on success
        assert qtable.update.called
        call_kwargs = qtable.update.call_args.kwargs
        assert call_kwargs["reward"] == 1.0  # 100% success in window

    @pytest.mark.asyncio
    async def test_on_act_requested_failure(self):
        """_on_act_requested reduces reward on failure."""
        svc = MagicMock(spec=KernelServices)
        ua = MagicMock()
        qtable = MagicMock()

        # Mock UniversalActuator.dispatch to return failure
        result = ActResult(
            action_type="bash",
            success=False,
            output="",
            error="Command failed",
            duration_ms=50.0
        )
        ua.dispatch = AsyncMock(return_value=result)

        handler = DirectActionsHandler(svc, universal_actuator=ua, qtable=qtable)
        event = Event(
            type=CoreEvent.ACT_REQUESTED,
            payload={
                "action_type": "bash",
                "payload": {"args": ["false"]},
            },
        )

        await handler._on_act_requested(event)

        # Verify QTable.update was called with reduced reward
        assert qtable.update.called
        call_kwargs = qtable.update.call_args.kwargs
        assert call_kwargs["reward"] == 0.0  # 0% success in window × 0.5

    @pytest.mark.asyncio
    async def test_execution_window_tracking(self):
        """_on_act_requested tracks success rate in rolling window."""
        svc = MagicMock(spec=KernelServices)
        ua = MagicMock()
        qtable = MagicMock()

        handler = DirectActionsHandler(svc, universal_actuator=ua, qtable=qtable)

        # Simulate 3 successes + 1 failure
        results = [
            ActResult("bash", success=True, duration_ms=10.0),
            ActResult("bash", success=True, duration_ms=10.0),
            ActResult("bash", success=True, duration_ms=10.0),
            ActResult("bash", success=False, duration_ms=10.0),
        ]

        for result in results:
            ua.dispatch = AsyncMock(return_value=result)
            event = Event(
                type=CoreEvent.ACT_REQUESTED,
                payload={"action_type": "bash", "payload": {}},
            )
            await handler._on_act_requested(event)

        # After 4 executions, success rate should be 75%
        assert len(handler._execution_window) == 4
        assert sum(handler._execution_window) == 3
        assert handler._execution_window == [True, True, True, False]

    @pytest.mark.asyncio
    async def test_execution_window_rolling_cap(self):
        """Execution window maintains rolling cap of F(7)=13."""
        svc = MagicMock(spec=KernelServices)
        ua = MagicMock()
        qtable = MagicMock()

        handler = DirectActionsHandler(svc, universal_actuator=ua, qtable=qtable)

        # Fill window beyond cap
        result = ActResult("bash", success=True, duration_ms=10.0)
        ua.dispatch = AsyncMock(return_value=result)

        for i in range(20):
            event = Event(
                type=CoreEvent.ACT_REQUESTED,
                payload={"action_type": "bash", "payload": {}},
            )
            await handler._on_act_requested(event)

        # Window should be capped at 13
        assert len(handler._execution_window) == 13
        assert all(handler._execution_window)  # all True

    @pytest.mark.asyncio
    async def test_exception_doesnt_crash(self):
        """Exception in handler doesn't crash event bus."""
        svc = MagicMock(spec=KernelServices)
        ua = MagicMock()
        qtable = MagicMock()

        ua.dispatch = AsyncMock(side_effect=RuntimeError("Dispatch failed"))

        handler = DirectActionsHandler(svc, universal_actuator=ua, qtable=qtable)
        event = Event(
            type=CoreEvent.ACT_REQUESTED,
            payload={"action_type": "bash", "payload": {}},
        )

        # Should not raise
        await handler._on_act_requested(event)
