"""Tests for MCP event flow — tools/call → organism event bus.

Verifies that MCPRouter emits MCP_TOOL_CALLED events to the CORE bus
when it receives tools/call JSON-RPC messages.  This is the integration
seam between the MCP WebSocket protocol layer and the organism's
event-driven architecture.

TDD: tests written FIRST, then router modified to pass.
"""
from __future__ import annotations

import asyncio
from typing import Any

import pytest

from cynic.core.event_bus import CoreEvent, Event, EventBus, reset_all_buses


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture(autouse=True)
def _clean_buses() -> None:
    """Reset global bus singletons between tests."""
    reset_all_buses()
    yield
    reset_all_buses()


def _make_tools_call_message(
    tool_name: str = "ask_cynic",
    arguments: dict[str, Any] | None = None,
    msg_id: int = 42,
) -> dict[str, Any]:
    """Build a minimal tools/call JSON-RPC message."""
    return {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "id": msg_id,
        "params": {
            "name": tool_name,
            "arguments": arguments or {},
        },
    }


# ════════════════════════════════════════════════════════════════════════════
# EVENT EMISSION ON TOOL CALL
# ════════════════════════════════════════════════════════════════════════════

class TestMCPRouterEmitsEvents:
    """MCPRouter must emit MCP_TOOL_CALLED to the CORE bus on tools/call."""

    @pytest.mark.asyncio
    async def test_tool_call_emits_mcp_tool_called_event(self) -> None:
        """A tools/call message should produce an MCP_TOOL_CALLED event."""
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        await router.bridge.startup()

        # Capture events from the CORE bus
        captured: list[Event] = []

        async def capture(event: Event) -> None:
            captured.append(event)

        bus = router.bridge._get_bus()
        bus.on(CoreEvent.MCP_TOOL_CALLED, capture)

        message = _make_tools_call_message(
            tool_name="ask_cynic",
            arguments={"question": "what is phi?"},
            msg_id=42,
        )
        await router.handle_message_async(message)
        await asyncio.sleep(0.05)

        # The bridge already emits one event; the router should emit
        # an additional event with request_id and source context
        router_events = [
            e for e in captured
            if e.payload.get("request_id") == 42
        ]
        assert len(router_events) == 1, (
            f"Expected 1 router-emitted event with request_id=42, got {len(router_events)}"
        )

        event = router_events[0]
        assert event.type == CoreEvent.MCP_TOOL_CALLED
        assert event.payload["tool_name"] == "ask_cynic"
        assert event.payload["arguments"]["question"] == "what is phi?"
        assert event.payload["request_id"] == 42
        assert event.payload["source"] == "websocket"

    @pytest.mark.asyncio
    async def test_event_source_is_mcp_router(self) -> None:
        """The router event's source field should identify the emitter."""
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        await router.bridge.startup()

        captured: list[Event] = []

        async def capture(event: Event) -> None:
            captured.append(event)

        bus = router.bridge._get_bus()
        bus.on(CoreEvent.MCP_TOOL_CALLED, capture)

        await router.handle_message_async(_make_tools_call_message())
        await asyncio.sleep(0.05)

        router_events = [
            e for e in captured if e.payload.get("source") == "websocket"
        ]
        assert len(router_events) >= 1
        assert router_events[0].source == "mcp_router"

    @pytest.mark.asyncio
    async def test_event_emitted_before_bridge_call(self) -> None:
        """The router event should be emitted before the bridge processes the call.

        We verify ordering by checking that the router event's timestamp is
        less than or equal to the bridge event's timestamp.
        """
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        await router.bridge.startup()

        timestamps: list[float] = []

        async def capture(event: Event) -> None:
            timestamps.append(event.timestamp)

        bus = router.bridge._get_bus()
        bus.on(CoreEvent.MCP_TOOL_CALLED, capture)

        await router.handle_message_async(_make_tools_call_message())
        await asyncio.sleep(0.05)

        assert len(timestamps) >= 2, (
            f"Expected at least 2 MCP_TOOL_CALLED events (router + bridge), got {len(timestamps)}"
        )
        # Router event should come first (emitted before bridge.handle_call)
        assert timestamps[0] <= timestamps[1]

    @pytest.mark.asyncio
    async def test_different_tools_produce_correct_event_payload(self) -> None:
        """Each tool call should carry the correct tool_name in the event."""
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        await router.bridge.startup()

        captured: list[Event] = []

        async def capture(event: Event) -> None:
            captured.append(event)

        bus = router.bridge._get_bus()
        bus.on(CoreEvent.MCP_TOOL_CALLED, capture)

        await router.handle_message_async(
            _make_tools_call_message(tool_name="observe_cynic", msg_id=100)
        )
        await asyncio.sleep(0.05)

        router_events = [
            e for e in captured if e.payload.get("request_id") == 100
        ]
        assert len(router_events) == 1
        assert router_events[0].payload["tool_name"] == "observe_cynic"

    @pytest.mark.asyncio
    async def test_failed_bridge_call_still_emits_event(self) -> None:
        """Even if the bridge call fails, the router event should have been emitted.

        The event is emitted BEFORE the bridge processes the call, so failures
        in bridge.handle_call do not prevent the event from reaching the bus.
        """
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        # Do NOT start the bridge — handle_call will raise RuntimeError

        captured: list[Event] = []

        async def capture(event: Event) -> None:
            captured.append(event)

        from cynic.core.event_bus import get_core_bus
        bus = get_core_bus()
        bus.on(CoreEvent.MCP_TOOL_CALLED, capture)

        response = await router.handle_message_async(
            _make_tools_call_message(msg_id=77)
        )
        await asyncio.sleep(0.05)

        # Response should be an error (bridge not running)
        assert "error" in response

        # But the event should still have been emitted
        router_events = [
            e for e in captured if e.payload.get("request_id") == 77
        ]
        assert len(router_events) == 1, (
            "Router should emit event even when bridge call fails"
        )


# ════════════════════════════════════════════════════════════════════════════
# PAYLOAD SCHEMA VALIDATION
# ════════════════════════════════════════════════════════════════════════════

class TestMcpToolCalledPayload:
    """The McpToolCalledPayload schema should validate router events."""

    def test_payload_validates_from_dict(self) -> None:
        from cynic.core.events_schema import McpToolCalledPayload

        payload = McpToolCalledPayload.model_validate({
            "tool_name": "ask_cynic",
            "arguments": {"prompt": "test"},
            "request_id": 42,
            "source": "websocket",
        })
        assert payload.tool_name == "ask_cynic"
        assert payload.arguments == {"prompt": "test"}
        assert payload.request_id == 42
        assert payload.source == "websocket"

    def test_payload_defaults(self) -> None:
        from cynic.core.events_schema import McpToolCalledPayload

        payload = McpToolCalledPayload()
        assert payload.tool_name == ""
        assert payload.arguments == {}
        assert payload.request_id is None
        assert payload.source == "websocket"

    def test_payload_allows_extra_fields(self) -> None:
        from cynic.core.events_schema import McpToolCalledPayload

        payload = McpToolCalledPayload.model_validate({
            "tool_name": "ask_cynic",
            "extra_field": "should not be rejected",
        })
        assert payload.tool_name == "ask_cynic"
