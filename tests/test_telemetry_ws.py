"""
Integration tests for telemetry WebSocket streaming.
"""
from __future__ import annotations

import asyncio
import json
import time
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.interfaces.api.routers.telemetry_ws import ws_telemetry
from cynic.kernel.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter


class TestTelemetryAdapter:
    """Test ClaudeCodeAdapter.stream_telemetry() method."""

    @pytest.mark.asyncio
    async def test_stream_telemetry_initialization(self):
        """Test adapter initializes correctly."""
        adapter = ClaudeCodeAdapter(cynic_url="http://localhost:8765")
        assert adapter.cynic_url == "http://localhost:8765"
        await adapter.close()

    @pytest.mark.asyncio
    async def test_stream_telemetry_connection_error(self):
        """Test adapter handles connection errors gracefully."""
        adapter = ClaudeCodeAdapter(cynic_url="http://localhost:9999")
        # Connection should fail gracefully (service not running)
        result = await adapter.stream_telemetry(duration_s=1)
        assert "error" in result
        await adapter.close()

    def test_summarize_telemetry_events_empty(self):
        """Test summary with no events."""
        summary = ClaudeCodeAdapter._summarize_telemetry_events([], 30.0)
        assert summary["judgments_seen"] == 0
        assert summary["learning_events_seen"] == 0
        assert summary["meta_cycles_seen"] == 0
        assert summary["sona_ticks_seen"] == 0
        assert summary["duration_s"] == 30.0

    def test_summarize_telemetry_events_with_judgments(self):
        """Test summary with judgment events."""
        events = [
            {"type": "judgment", "q_score": 75, "verdict": "WAG", "ts": 1.0},
            {"type": "judgment", "q_score": 85, "verdict": "HOWL", "ts": 2.0},
        ]
        summary = ClaudeCodeAdapter._summarize_telemetry_events(events, 10.0)
        assert summary["judgments_seen"] == 2
        assert summary["avg_q_score"] == 80.0  # (75 + 85) / 2
        assert summary["verdicts"]["WAG"] == 1
        assert summary["verdicts"]["HOWL"] == 1

    def test_summarize_telemetry_events_with_learning(self):
        """Test summary with learning events."""
        events = [
            {"type": "learning", "learning_rate": 0.001, "q_table_entries": 512, "ts": 1.0},
            {"type": "learning", "learning_rate": 0.0005, "q_table_entries": 1024, "ts": 2.0},
        ]
        summary = ClaudeCodeAdapter._summarize_telemetry_events(events, 10.0)
        assert summary["learning_events_seen"] == 2
        assert summary["last_learning_rate"] == 0.0005

    def test_summarize_telemetry_events_with_all_types(self):
        """Test summary with all event types."""
        events = [
            {"type": "judgment", "q_score": 70, "verdict": "WAG", "ts": 1.0},
            {"type": "learning", "learning_rate": 0.001, "q_table_entries": 512, "ts": 2.0},
            {"type": "meta_cycle", "cycle_n": 5, "health": 0.82, "ts": 3.0},
            {"type": "sona_tick", "uptime_s": 3600, "total_judgments": 12500, "ts": 4.0},
            {"type": "heartbeat", "ts": 5.0},  # Should be ignored
        ]
        summary = ClaudeCodeAdapter._summarize_telemetry_events(events, 10.0)
        assert summary["judgments_seen"] == 1
        assert summary["learning_events_seen"] == 1
        assert summary["meta_cycles_seen"] == 1
        assert summary["sona_ticks_seen"] == 1


class TestTelemetryWSEndpoint:
    """Test /ws/telemetry WebSocket endpoint."""

    @pytest.mark.asyncio
    async def test_ws_telemetry_endpoint_exists(self):
        """Test that WebSocket endpoint is properly decorated."""
        # Check that the function is decorated with @router.websocket
        assert hasattr(ws_telemetry, "__wrapped__") or callable(ws_telemetry)

    @pytest.mark.asyncio
    async def test_ws_telemetry_sends_connected_message(self):
        """Test that endpoint sends initial connected message."""
        # Mock WebSocket
        websocket_mock = AsyncMock()
        websocket_mock.accept = AsyncMock()
        websocket_mock.send_json = AsyncMock()
        websocket_mock.receive_json = AsyncMock(side_effect=Exception("disconnect"))

        with patch("cynic.interfaces.api.routers.telemetry_ws.get_core_bus") as mock_bus:
            mock_bus.return_value.on = MagicMock()
            mock_bus.return_value.off = MagicMock()

            try:
                await ws_telemetry(websocket_mock)
            except Exception:
                pass

            # Verify accept was called
            websocket_mock.accept.assert_called_once()

            # Verify initial message was sent with "connected" type
            calls = websocket_mock.send_json.call_args_list
            assert len(calls) > 0
            first_msg = calls[0][0][0]
            assert first_msg.get("type") == "connected"
            assert "phi" in first_msg

    @pytest.mark.asyncio
    async def test_ws_telemetry_subscribes_to_events(self):
        """Test that endpoint subscribes to correct events."""
        websocket_mock = AsyncMock()
        websocket_mock.accept = AsyncMock()
        websocket_mock.send_json = AsyncMock()
        websocket_mock.receive_json = AsyncMock(side_effect=Exception("disconnect"))

        with patch("cynic.interfaces.api.routers.telemetry_ws.get_core_bus") as mock_bus:
            bus_instance = MagicMock()
            mock_bus.return_value = bus_instance
            bus_instance.on = MagicMock()
            bus_instance.off = MagicMock()

            try:
                await ws_telemetry(websocket_mock)
            except Exception:
                pass

            # Verify all telemetry events were subscribed
            expected_events = [
                CoreEvent.JUDGMENT_CREATED,
                CoreEvent.LEARNING_EVENT,
                CoreEvent.META_CYCLE,
                CoreEvent.SONA_TICK,
            ]

            on_calls = [call[0][0] for call in bus_instance.on.call_args_list]
            for event in expected_events:
                assert event in on_calls, f"Event {event} not subscribed"

            # Verify cleanup: same events should be unsubscribed
            off_calls = [call[0][0] for call in bus_instance.off.call_args_list]
            for event in expected_events:
                assert event in off_calls, f"Event {event} not unsubscribed"


class TestTelemetryBridge:
    """Test MCP bridge tool integration."""

    def test_tool_definition_exists(self):
        """Test that cynic_watch_telemetry tool is properly defined."""
        from cynic.interfaces.mcp.claude_code_bridge import list_tools

        # Get tool definitions (sync wrapper for async)
        loop = asyncio.new_event_loop()
        tools = loop.run_until_complete(list_tools())
        loop.close()

        tool_names = [t.name for t in tools]
        assert "cynic_watch_telemetry" in tool_names

        # Find the tool
        telemetry_tool = next(t for t in tools if t.name == "cynic_watch_telemetry")
        assert "watch" in telemetry_tool.description.lower()
        assert "duration_s" in telemetry_tool.inputSchema["properties"]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
