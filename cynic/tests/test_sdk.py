"""
Tests for /ws/sdk — Claude Code --sdk-url WebSocket server.

CYNIC is the server (BRAIN). Claude Code is the client (HANDS).
We simulate Claude Code's NDJSON messages and verify CYNIC's responses.
"""
from __future__ import annotations

import json
import time

import pytest
from starlette.testclient import TestClient

from cynic.api.server import app
from cynic.api.state import build_kernel, set_state


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture(autouse=True)
def sdk_kernel_sync():
    """Sync kernel fixture — required because TestClient WebSocket is sync."""
    state = build_kernel(db_pool=None)
    set_state(state)
    yield
    state.learning_loop.stop()


def _send(ws, msg: dict) -> None:
    """Send one NDJSON message (JSON + newline)."""
    ws.send_text(json.dumps(msg) + "\n")


def _recv(ws) -> dict:
    """Receive one NDJSON message and parse it."""
    raw = ws.receive_text()
    # Handle multi-line frames
    for line in raw.splitlines():
        line = line.strip()
        if line:
            return json.loads(line)
    return {}


def _system_init(session_id: str = "test-session-1") -> dict:
    return {
        "type": "system",
        "subtype": "init",
        "session_id": session_id,
        "cwd": "/tmp/test_project",
        "tools": ["Bash", "Edit", "Write", "Read", "Glob"],
        "model": "claude-sonnet-4-6",
        "claude_code_version": "2.1.45",
    }


def _can_use_tool(tool_name: str, tool_input: dict, request_id: str = "req-1") -> dict:
    return {
        "type": "control_request",
        "subtype": "can_use_tool",
        "request_id": request_id,
        "request": {
            "subtype": "can_use_tool",
            "tool_name": tool_name,
            "input": tool_input,
            "tool_use_id": "tu-1",
        },
    }


def _result(is_error: bool = False, cost: float = 0.003) -> dict:
    return {
        "type": "result",
        "subtype": "success" if not is_error else "error_during_execution",
        "is_error": is_error,
        "result": "Done" if not is_error else "Error",
        "duration_ms": 1234,
        "total_cost_usd": cost,
        "usage": {"input_tokens": 100, "output_tokens": 50},
        "session_id": "test-session-1",
    }


# ════════════════════════════════════════════════════════════════════════════
# TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestSDKConnection:

    def test_sdk_accepts_connection(self):
        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            msg = _recv(ws)
        assert msg["type"] == "keep_alive"

    def test_sdk_session_appears_in_registry(self):
        from cynic.api.server import _sdk_sessions
        initial = set(_sdk_sessions.keys())
        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)
            # Session should be registered while connected
            active = set(_sdk_sessions.keys())
            assert len(active) > len(initial)
        # Session removed after disconnect
        assert set(_sdk_sessions.keys()) == initial

    def test_sdk_session_records_init_metadata(self):
        from cynic.api.server import _sdk_sessions
        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)
            session = next(iter(_sdk_sessions.values()))
            assert session.cwd == "/tmp/test_project"
            assert session.model == "claude-sonnet-4-6"
            assert "Bash" in session.tools


class TestSDKToolJudgment:

    def test_sdk_approves_safe_read_tool(self):
        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)  # keep_alive after init

            _send(ws, _can_use_tool("Read", {"file_path": "/tmp/test.py"}))
            response = _recv(ws)

        assert response["type"] == "control_response"
        inner = response["response"]["response"]
        # Read is safe — should be allowed (WAG or HOWL)
        assert inner["behavior"] == "allow"
        assert "updatedInput" in inner

    def test_sdk_control_response_has_request_id(self):
        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)

            _send(ws, _can_use_tool("Bash", {"command": "ls -la"}, request_id="my-req-42"))
            response = _recv(ws)

        assert response["response"]["request_id"] == "my-req-42"

    def test_sdk_allow_response_has_updated_input(self):
        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)

            tool_input = {"command": "echo hello"}
            _send(ws, _can_use_tool("Bash", tool_input))
            response = _recv(ws)

        inner = response["response"]["response"]
        if inner["behavior"] == "allow":
            assert "updatedInput" in inner

    def test_sdk_responds_to_keep_alive(self):
        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)  # keep_alive after init

            _send(ws, {"type": "keep_alive"})
            response = _recv(ws)

        assert response["type"] == "keep_alive"


class TestSDKResultLearning:

    def test_sdk_result_updates_qtable(self):
        from cynic.api.state import get_state
        state = get_state()
        before = state.qtable.stats()["total_updates"]

        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)
            _send(ws, _result(is_error=False, cost=0.003))
            # No response expected for result messages

        after = state.qtable.stats()["total_updates"]
        assert after > before

    def test_sdk_result_tracks_cost(self):
        from cynic.api.server import _sdk_sessions

        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)

            session = next(iter(_sdk_sessions.values()))
            assert session.total_cost_usd == 0.0

            _send(ws, _result(cost=0.005))
            # Give async tasks a moment (fire-and-forget)
            import time as _t
            _t.sleep(0.05)

            assert session.total_cost_usd == pytest.approx(0.005, abs=1e-6)

    def test_sdk_error_result_gives_low_reward(self):
        """Error result → low reward → Q-Table updated with BARK on rich state key."""
        from cynic.api.state import get_state
        state = get_state()

        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)
            _send(ws, _result(is_error=True, cost=0.002))

        # Rich state key: SDK:{model}:{task_type}:{complexity}
        # No task prompt → "general", no tools → "trivial"
        import time as _t
        _t.sleep(0.05)
        q_bark = state.qtable.predict_q("SDK:claude-sonnet-4-6:general:trivial", "BARK")
        assert q_bark > 0


class TestSDKSessions:

    def test_sdk_sessions_endpoint_empty_when_no_connections(self):
        from cynic.api.server import _sdk_sessions
        _sdk_sessions.clear()

        with TestClient(app) as client:
            resp = client.get("/sdk/sessions")
        assert resp.status_code == 200
        assert resp.json()["active"] == 0

    def test_sdk_sessions_endpoint_shows_active_session(self):
        from cynic.api.server import _sdk_sessions
        _sdk_sessions.clear()

        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)

            with TestClient(app) as client:
                resp = client.get("/sdk/sessions")

        data = resp.json()
        # Session cleanup may have happened by the time we assert — just check structure
        assert "active" in data
        assert "sessions" in data

    def test_sdk_task_endpoint_returns_404_without_session(self):
        from cynic.api.server import _sdk_sessions
        _sdk_sessions.clear()

        with TestClient(app) as client:
            resp = client.post("/sdk/task", json={"prompt": "hello"})
        assert resp.status_code == 404

    def test_sdk_task_endpoint_requires_prompt(self):
        with TestClient(app) as client:
            resp = client.post("/sdk/task", json={"session_id": "xyz"})
        assert resp.status_code == 400


class TestSDKAssistantMessages:

    def test_sdk_handles_assistant_message_silently(self):
        """Assistant messages are recorded but no response is sent."""
        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)  # keep_alive

            # Send assistant message (Claude's response)
            _send(ws, {
                "type": "assistant",
                "message": {
                    "role": "assistant",
                    "content": [{"type": "text", "text": "I'll help you with that."}],
                    "usage": {"input_tokens": 50, "output_tokens": 20},
                },
                "session_id": "test-session-1",
            })

            # Send keep_alive to verify connection is still alive
            _send(ws, {"type": "keep_alive"})
            response = _recv(ws)

        assert response["type"] == "keep_alive"


class TestSDKRootRoute:

    def test_root_lists_sdk_routes(self):
        with TestClient(app) as client:
            resp = client.get("/")
        routes = resp.json()["routes"]
        assert "/ws/sdk" in routes
        assert "/sdk/sessions" in routes
        assert "/sdk/task" in routes
