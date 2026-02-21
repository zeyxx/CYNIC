"""MCP integration tests — full-stack validation.

Validates the MCP subsystem works end-to-end:
- WebSocket tools/list flow
- Metrics updated after tool calls
- Health endpoint values
- Bridge accessible from organism structure

Uses real MCPRouter + MCPBridge instances (no full organism startup).
"""
from __future__ import annotations

from unittest.mock import MagicMock

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient

from cynic.api.routers.mcp_observability import router as obs_router
from cynic.api.routers.mcp_websocket import router as ws_router
from cynic.api.state import AppContainer, get_app_container
from cynic.mcp.router import MCPRouter
from cynic.mcp.service import MCPBridge, MCPTool
from cynic.organism.organism import SensoryCore


def _make_test_bridge() -> MCPBridge:
    """Create a bridge with default tools registered."""
    bridge = MCPBridge(bus_name="CORE")
    bridge.register_tool(MCPTool(
        name="ask_cynic",
        description="Ask CYNIC a question.",
        input_schema={"type": "object", "properties": {"prompt": {"type": "string"}}},
    ))
    bridge.register_tool(MCPTool(
        name="observe_cynic",
        description="Observe CYNIC state.",
        input_schema={"type": "object"},
    ))
    return bridge


def _make_app_with_bridge(bridge: MCPBridge) -> FastAPI:
    """Wire a FastAPI app with the observability + websocket routers."""
    organism = MagicMock()
    organism.senses.mcp_bridge = bridge

    container = MagicMock(spec=AppContainer)
    container.organism = organism

    app = FastAPI()
    app.include_router(obs_router)
    app.include_router(ws_router)
    app.dependency_overrides[get_app_container] = lambda: container
    return app


class TestFullWebSocketFlow:
    """tools/list via WebSocket returns registered tools."""

    def test_tools_list_via_websocket(self) -> None:
        bridge = _make_test_bridge()
        app = _make_app_with_bridge(bridge)
        client = TestClient(app)

        with client.websocket_connect("/ws/mcp") as ws:
            ws.send_json({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
            })
            response = ws.receive_json()

        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 1
        tools = response["result"]["tools"]
        tool_names = [t["name"] for t in tools]
        assert "ask_cynic" in tool_names
        assert "observe_cynic" in tool_names


class TestMCPMetricsUpdatedOnCall:
    """Metrics reflect tool calls made through the bridge."""

    @pytest.mark.asyncio
    async def test_metrics_updated_after_call(self) -> None:
        bridge = _make_test_bridge()
        await bridge.startup()

        assert bridge.metrics.total_calls == 0
        await bridge.handle_call("ask_cynic", {"prompt": "test"})
        assert bridge.metrics.total_calls == 1
        assert bridge.metrics.successful_calls == 1

        metrics = bridge.get_metrics()
        assert metrics["total_calls"] == 1
        assert metrics["avg_latency_ms"] > 0

        await bridge.shutdown()


class TestMCPHealthStatus:
    """Health endpoint returns proper values based on bridge state."""

    def test_health_degraded_when_not_running(self) -> None:
        bridge = _make_test_bridge()
        health = bridge.get_health()
        assert health["status"] == "degraded"
        assert health["tools_registered"] == 2

    @pytest.mark.asyncio
    async def test_health_healthy_after_startup(self) -> None:
        bridge = _make_test_bridge()
        await bridge.startup()
        health = bridge.get_health()
        assert health["status"] == "healthy"
        assert health["tools_registered"] == 2
        assert health["total_calls"] == 0
        assert health["error_rate"] == 0.0
        await bridge.shutdown()


class TestMCPBridgeInOrganism:
    """Bridge accessible from organism.senses.mcp_bridge."""

    def test_bridge_accessible_via_senses(self) -> None:
        bridge = _make_test_bridge()
        senses = SensoryCore(mcp_bridge=bridge)
        assert senses.mcp_bridge is bridge
        assert len(senses.mcp_bridge.tools) == 2

    def test_bridge_on_sensory_core_default(self) -> None:
        senses = SensoryCore()
        assert isinstance(senses.mcp_bridge, MCPBridge)
        assert senses.mcp_bridge.bus_name == "CORE"
