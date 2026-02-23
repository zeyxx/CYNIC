"""Tests for /ws/mcp WebSocket endpoint.

Uses a lightweight FastAPI app (no lifespan) to test the WebSocket
endpoint in isolation. The auto_register system handles wiring
into the real app at runtime.
"""
from __future__ import annotations

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient

from cynic.api.routers.mcp_websocket import router


def _make_test_app() -> FastAPI:
    """Create a minimal FastAPI app with just the MCP WebSocket router."""
    test_app = FastAPI()
    test_app.include_router(router)
    return test_app


@pytest.fixture
def client() -> TestClient:
    """TestClient with lightweight MCP WebSocket app."""
    return TestClient(_make_test_app())


def test_mcp_websocket_endpoint_exists(client: TestClient) -> None:
    """/ws/mcp should accept WebSocket connections."""
    with client.websocket_connect("/ws/mcp") as websocket:
        assert websocket is not None


def test_mcp_websocket_tools_list(client: TestClient) -> None:
    """Should handle tools/list via WebSocket."""
    with client.websocket_connect("/ws/mcp") as websocket:
        websocket.send_json({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1,
        })
        response = websocket.receive_json()
        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 1
        assert "result" in response
        assert "tools" in response["result"]
        assert isinstance(response["result"]["tools"], list)
        assert len(response["result"]["tools"]) >= 2


def test_mcp_websocket_tools_list_contains_expected_tools(client: TestClient) -> None:
    """tools/list should include ask_cynic and observe_cynic."""
    with client.websocket_connect("/ws/mcp") as websocket:
        websocket.send_json({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 2,
        })
        response = websocket.receive_json()
        tool_names = [t["name"] for t in response["result"]["tools"]]
        assert "ask_cynic" in tool_names
        assert "observe_cynic" in tool_names


def test_mcp_websocket_unknown_method(client: TestClient) -> None:
    """Unknown methods should return JSON-RPC error."""
    with client.websocket_connect("/ws/mcp") as websocket:
        websocket.send_json({
            "jsonrpc": "2.0",
            "method": "unknown/method",
            "id": 3,
        })
        response = websocket.receive_json()
        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 3
        assert "error" in response
        assert response["error"]["code"] == -32601


def test_mcp_websocket_preserves_message_id(client: TestClient) -> None:
    """Response id must match request id for each message."""
    with client.websocket_connect("/ws/mcp") as websocket:
        for msg_id in [42, "abc-123", 0]:
            websocket.send_json({
                "jsonrpc": "2.0",
                "method": "tools/list",
                "id": msg_id,
            })
            response = websocket.receive_json()
            assert response["id"] == msg_id


def test_mcp_websocket_multiple_messages(client: TestClient) -> None:
    """Single connection should handle multiple sequential messages."""
    with client.websocket_connect("/ws/mcp") as websocket:
        # First: tools/list
        websocket.send_json({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1,
        })
        resp1 = websocket.receive_json()
        assert "result" in resp1

        # Second: unknown method
        websocket.send_json({
            "jsonrpc": "2.0",
            "method": "nonexistent",
            "id": 2,
        })
        resp2 = websocket.receive_json()
        assert "error" in resp2

        # Third: tools/list again
        websocket.send_json({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 3,
        })
        resp3 = websocket.receive_json()
        assert "result" in resp3
