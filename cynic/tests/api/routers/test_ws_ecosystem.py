"""Tests for WebSocket /ws/consciousness/ecosystem endpoint."""
from __future__ import annotations

import pytest
from fastapi.testclient import TestClient
from cynic.api.server import app


def test_ws_ecosystem_connect():
    """WebSocket /ws/consciousness/ecosystem accepts connection."""
    with TestClient(app) as client:
        with client.websocket_connect("/ws/consciousness/ecosystem") as websocket:
            # Receive connected message
            data = websocket.receive_json()
            assert data["type"] == "connected"
            assert "phi" in data
            assert "initial_snapshot" in data
            assert isinstance(data["initial_snapshot"], dict)


def test_ws_ecosystem_receive_periodic_updates():
    """WebSocket receives periodic ecosystem updates."""
    with TestClient(app) as client:
        with client.websocket_connect("/ws/consciousness/ecosystem") as websocket:
            # Receive connected message
            connected = websocket.receive_json()
            assert connected["type"] == "connected"

            # Should receive at least one update (periodic 5s interval)
            msg = websocket.receive_json()
            assert msg["type"] in ["ecosystem_update", "ping"]
            assert "ts" in msg
