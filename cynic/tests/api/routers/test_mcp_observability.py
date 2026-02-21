"""Tests for MCP observability endpoints.

Validates /api/mcp/bridge/health, /api/mcp/bridge/metrics, /api/mcp/bridge/tools
return correct shapes and status codes.

Uses a mock organism with a real MCPBridge instance to avoid
spinning up the full kernel.
"""
from __future__ import annotations

from unittest.mock import MagicMock

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient

from cynic.api.routers.mcp_observability import router
from cynic.api.state import AppContainer, get_app_container
from cynic.mcp.service import MCPBridge, MCPTool


def _make_app() -> tuple[FastAPI, MCPBridge]:
    """Create a test FastAPI app with a real MCPBridge on a mock organism."""
    bridge = MCPBridge(bus_name="CORE")
    bridge.register_tool(MCPTool(
        name="ask_cynic",
        description="Test tool",
        input_schema={"type": "object"},
    ))

    organism = MagicMock()
    organism.senses.mcp_bridge = bridge

    container = MagicMock(spec=AppContainer)
    container.organism = organism

    app = FastAPI()
    app.include_router(router)
    app.dependency_overrides[get_app_container] = lambda: container

    return app, bridge


class TestMCPHealth:
    """GET /api/mcp/bridge/health returns bridge health."""

    def test_health_returns_200(self) -> None:
        app, _ = _make_app()
        client = TestClient(app)
        resp = client.get("/api/mcp/bridge/health")
        assert resp.status_code == 200

    def test_health_has_expected_fields(self) -> None:
        app, _ = _make_app()
        client = TestClient(app)
        data = client.get("/api/mcp/bridge/health").json()
        assert "status" in data
        assert "tools_registered" in data
        assert "total_calls" in data
        assert "error_rate" in data


class TestMCPMetrics:
    """GET /api/mcp/bridge/metrics returns bridge metrics."""

    def test_metrics_returns_200(self) -> None:
        app, _ = _make_app()
        client = TestClient(app)
        resp = client.get("/api/mcp/bridge/metrics")
        assert resp.status_code == 200

    def test_metrics_has_expected_fields(self) -> None:
        app, _ = _make_app()
        client = TestClient(app)
        data = client.get("/api/mcp/bridge/metrics").json()
        assert "total_calls" in data
        assert "successful_calls" in data
        assert "avg_latency_ms" in data
        assert "uptime_s" in data


class TestMCPTools:
    """GET /api/mcp/bridge/tools returns registered tools."""

    def test_tools_returns_200(self) -> None:
        app, _ = _make_app()
        client = TestClient(app)
        resp = client.get("/api/mcp/bridge/tools")
        assert resp.status_code == 200

    def test_tools_has_expected_fields(self) -> None:
        app, _ = _make_app()
        client = TestClient(app)
        data = client.get("/api/mcp/bridge/tools").json()
        assert "tools" in data
        assert "count" in data
        assert data["count"] == 1
        assert data["tools"][0]["name"] == "ask_cynic"
