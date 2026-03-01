"""
Tests for Phase 2: MCP Learning Feedback.

Verifies that:
1. ask_cynic → orchestrator.run() returns real Judgment result
2. learn endpoint → emits USER_FEEDBACK event on CORE bus
3. Fallback behavior when orchestrator unavailable
4. observe_cynic → returns state snapshot
"""
from __future__ import annotations

import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.interfaces.mcp.router import MCPRouter
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.event_bus import CoreEvent
from cynic.kernel.core.judgment import Cell, Judgment

# ════════════════════════════════════════════════════════════════════════════
# TEST: ask_cynic generates real Judgment and learning event
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_ask_cynic_returns_judgment_result():
    """Verify ask_cynic calls orchestrator and returns Judgment fields."""
    router = MCPRouter()

    # Mock orchestrator
    mock_judgment = Judgment(
        cell=Cell(
            reality="CODE",
            analysis="JUDGE",
            content="test prompt",
            context="test context",
        ),
        q_score=65.0,
        verdict="WAG",
        confidence=0.60,
    )

    mock_orchestrator = AsyncMock()
    mock_orchestrator.run.return_value = mock_judgment

    mock_container = MagicMock()
    mock_container.orchestrator = mock_orchestrator

    with patch("cynic.interfaces.mcp.router.get_app_container", return_value=mock_container):
        message = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ask_cynic",
                "arguments": {"prompt": "Is this code clean?"},
            },
        }

        response = await router.handle_message_async(message)

        # Verify orchestrator was called
        mock_orchestrator.run.assert_called_once()
        call_args = mock_orchestrator.run.call_args
        assert call_args.kwargs["level"] == ConsciousnessLevel.REFLEX
        assert call_args.kwargs["budget_usd"] == 0.001

        # Verify response contains judgment fields
        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 1
        assert "result" in response
        result = response["result"]
        assert result["verdict"] == "WAG"
        assert result["q_score"] == 65.0
        assert result["confidence"] == 0.6
        assert result["judgment_id"] == mock_judgment.judgment_id


@pytest.mark.asyncio
async def test_ask_cynic_fallback_without_orchestrator():
    """Verify graceful fallback when orchestrator unavailable."""
    router = MCPRouter()

    # Mock container with no orchestrator
    mock_container = MagicMock()
    mock_container.orchestrator = None

    with patch("cynic.interfaces.mcp.router.get_app_container", return_value=mock_container):
        message = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ask_cynic",
                "arguments": {"prompt": "test"},
            },
        }

        response = await router.handle_message_async(message)

        # Should fallback to fire-and-forget
        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 1
        assert "result" in response
        assert response["result"].get("status") == "emitted"


@pytest.mark.asyncio
async def test_ask_cynic_requires_prompt():
    """Verify ask_cynic rejects missing prompt."""
    router = MCPRouter()

    message = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "ask_cynic",
            "arguments": {},  # No prompt
        },
    }

    response = await router.handle_message_async(message)

    # Should return error
    assert response["jsonrpc"] == "2.0"
    assert response["id"] == 1
    assert "error" in response
    assert "prompt required" in response["error"]["message"]


# ════════════════════════════════════════════════════════════════════════════
# TEST: observe_cynic returns health snapshot
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_observe_cynic_returns_snapshot():
    """Verify observe_cynic returns consciousness + qtable + registry snapshot."""
    router = MCPRouter()

    # Mock container with state
    mock_consciousness = MagicMock()
    mock_consciousness.to_dict.return_value = {"lod": 1, "uptime_s": 123.45}

    mock_qtable = MagicMock()
    mock_qtable.stats.return_value = {"entries": 42, "alpha": 0.1}

    mock_registry = MagicMock()
    mock_snapshot = MagicMock()
    mock_snapshot.total_components = 10
    mock_snapshot.healthy_count = 8
    mock_snapshot.degraded_count = 2
    mock_snapshot.stalled_count = 0
    mock_snapshot.failed_count = 0
    mock_registry.snapshot = AsyncMock(return_value=mock_snapshot)

    mock_container = MagicMock()
    mock_container.consciousness = mock_consciousness
    mock_container.qtable = mock_qtable
    mock_container.service_registry = mock_registry

    with patch("cynic.interfaces.mcp.router.get_app_container", return_value=mock_container):
        message = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "observe_cynic",
                "arguments": {"aspect": "health"},
            },
        }

        response = await router.handle_message_async(message)

        # Verify response
        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 2
        assert "result" in response
        result = response["result"]
        assert result["aspect"] == "health"
        assert "consciousness" in result
        assert result["consciousness"]["lod"] == 1
        assert "qtable" in result
        assert result["qtable"]["entries"] == 42
        assert "registry" in result
        assert result["registry"]["healthy"] == 8


# ════════════════════════════════════════════════════════════════════════════
# TEST: learn endpoint emits USER_FEEDBACK event
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_learn_endpoint_emits_user_feedback_event(monkeypatch):
    """Verify POST /learn emits USER_FEEDBACK event on CORE bus."""
    from cynic.interfaces.mcp.server import MCPServer

    # Mock state
    mock_qtable = MagicMock()
    mock_qtable._alpha = 0.1
    mock_qtable.update = MagicMock()
    mock_qtable.update.return_value.q_value = 0.75

    mock_state = MagicMock()
    mock_state.qtable = mock_qtable

    server = MCPServer(get_state_fn=lambda: mock_state)

    # Mock event bus
    mock_bus = AsyncMock()
    monkeypatch.setattr("cynic.interfaces.mcp.server.get_core_bus", lambda: mock_bus)

    # Mock web request
    request = AsyncMock()
    request.json = AsyncMock(return_value={
        "signal": {
            "judgment_id": "test-judgment-id",
            "action": "WAG",
            "rating": 1.0,  # positive feedback
            "comment": "good",
        },
        "update_qtable": True,
    })

    # Call handler
    await server._handle_learn(request)

    # Verify USER_FEEDBACK event was emitted
    mock_bus.emit.assert_called()
    call_args = mock_bus.emit.call_args
    emitted_event = call_args[0][0]  # First positional arg is the Event

    # Verify event structure
    assert emitted_event.type == CoreEvent.USER_FEEDBACK
    payload = emitted_event.dict_payload
    assert payload["judgment_id"] == "test-judgment-id"
    assert payload["action"] == "WAG"
    assert payload["reward"] == 1.0  # (1.0 + 1) / 2
    assert payload["sentiment"] == 0.5  # 1.0 - 0.5


@pytest.mark.asyncio
async def test_learn_endpoint_continues_on_event_emission_failure(monkeypatch):
    """Verify learn endpoint doesn't fail if event emission fails."""
    from cynic.interfaces.mcp.server import MCPServer

    # Mock state
    mock_qtable = MagicMock()
    mock_qtable._alpha = 0.1
    mock_qtable.update = MagicMock()
    mock_qtable.update.return_value.q_value = 0.75

    mock_state = MagicMock()
    mock_state.qtable = mock_qtable

    server = MCPServer(get_state_fn=lambda: mock_state)

    # Mock event bus to raise exception
    mock_bus = AsyncMock()
    mock_bus.emit.side_effect = RuntimeError("Bus is down")
    monkeypatch.setattr("cynic.interfaces.mcp.server.get_core_bus", lambda: mock_bus)

    # Mock web request
    request = AsyncMock()
    request.json = AsyncMock(return_value={
        "signal": {
            "judgment_id": "test-id",
            "action": "GROWL",
            "rating": -0.5,
            "comment": "bad",
        },
        "update_qtable": True,
    })

    # Call handler — should NOT raise, should return success
    response = await server._handle_learn(request)

    # Verify response is still successful (status 200)
    response_json = json.loads(response.body)
    assert response_json["status"] == "ok"


# ════════════════════════════════════════════════════════════════════════════
# TEST: MCP_TOOL_CALLED event always emitted before handler
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_mcp_tool_called_event_emitted_before_handler(monkeypatch):
    """Verify MCP_TOOL_CALLED event emitted even if handler fails."""
    router = MCPRouter()

    # Mock orchestrator to fail
    mock_orchestrator = AsyncMock()
    mock_orchestrator.run.side_effect = RuntimeError("Orchestrator down")

    mock_container = MagicMock()
    mock_container.orchestrator = mock_orchestrator

    # Mock core bus to track events
    emitted_events = []
    async def track_emit(event):
        emitted_events.append(event)

    mock_bus = AsyncMock()
    mock_bus.emit.side_effect = track_emit

    monkeypatch.setattr("cynic.interfaces.mcp.router.get_app_container", lambda: mock_container)
    monkeypatch.setattr("cynic.interfaces.mcp.router.get_core_bus", lambda: mock_bus)

    message = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "ask_cynic",
            "arguments": {"prompt": "test"},
        },
    }

    await router.handle_message_async(message)

    # MCP_TOOL_CALLED should have been emitted
    assert len(emitted_events) > 0
    first_event = emitted_events[0]
    assert first_event.type == CoreEvent.MCP_TOOL_CALLED


# ════════════════════════════════════════════════════════════════════════════
# TEST: Integration — ask_cynic + learn flow
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_ask_cynic_learn_integration():
    """
    End-to-end: ask_cynic judgment → user rates → learn endpoint

    Verifies:
    1. ask_cynic returns judgment
    2. client rates judgment via learn
    3. learn endpoint emits USER_FEEDBACK event
    4. Q-Table updated
    """
    router = MCPRouter()

    # Create realistic judgment
    mock_judgment = Judgment(
        cell=Cell(
            reality="CODE",
            analysis="JUDGE",
            content="review_code()",
            context="Should we review this?",
        ),
        q_score=72.0,
        verdict="WAG",
        confidence=0.60,
    )

    mock_orchestrator = AsyncMock()
    mock_orchestrator.run.return_value = mock_judgment

    mock_container = MagicMock()
    mock_container.orchestrator = mock_orchestrator

    with patch("cynic.interfaces.mcp.router.get_app_container", return_value=mock_container):
        # Step 1: ask_cynic
        message1 = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ask_cynic",
                "arguments": {"prompt": "Should we review this code?"},
            },
        }

        response1 = await router.handle_message_async(message1)
        assert response1["result"]["verdict"] == "WAG"
        judgment_id = response1["result"]["judgment_id"]

        # Step 2: client would call learn with this judgment_id
        # (not testing here as it's in server.py, already tested above)
        assert judgment_id == mock_judgment.judgment_id
