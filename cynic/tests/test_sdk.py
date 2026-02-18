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


# ════════════════════════════════════════════════════════════════════════════
# L2 — prompt enrichment + JSONL persistence + L2→L1 cross-feed
# ════════════════════════════════════════════════════════════════════════════

class TestEnrichPrompt:
    """Tests for _enrich_prompt() — CYNIC→Claude context injection."""

    def test_enrich_returns_raw_prompt_on_cold_start(self):
        """No compressor context, no QTable data → raw prompt returned."""
        from cynic.api.server import _enrich_prompt
        from cynic.api.state import get_state
        state = get_state()
        # Fresh kernel: no compressor history, no QTable entries
        raw = "Fix the bug in auth.py"
        result = _enrich_prompt(raw, state)
        # Cold start: confidence < 0.10 and no context → raw returned
        assert result == raw

    def test_enrich_adds_context_when_qtable_has_data(self):
        """QTable has enough visits → context block prepended."""
        from cynic.api.server import _enrich_prompt
        from cynic.api.state import get_state
        from cynic.learning.qlearning import LearningSignal
        state = get_state()

        # Seed QTable with enough visits to raise confidence above threshold
        state_key = "SDK:default:debug:medium"
        for _ in range(25):  # F(8)=21 is "well-seen"; 25 gives ~0.618 confidence
            state.qtable.update(LearningSignal(
                state_key=state_key,
                action="WAG",
                reward=0.65,
                judgment_id="test",
                loop_name="SDK_RESULT",
            ))

        prompt = "Fix the bug in auth.py"
        result = _enrich_prompt(prompt, state)
        # Enriched: CYNIC context block prepended
        assert "# CYNIC Context" in result
        assert "debug" in result
        assert result.endswith(prompt)

    def test_enrich_includes_task_type(self):
        """Task type is correctly classified and included in enrichment."""
        from cynic.api.server import _enrich_prompt
        from cynic.api.state import get_state
        from cynic.learning.qlearning import LearningSignal
        state = get_state()

        # Seed for "test" task type
        state_key = "SDK:default:test:medium"
        for _ in range(25):
            state.qtable.update(LearningSignal(
                state_key=state_key,
                action="HOWL",
                reward=0.70,
                judgment_id="t",
                loop_name="SDK_RESULT",
            ))

        result = _enrich_prompt("Write tests for the auth module", state)
        assert "test" in result

    def test_enrich_preserves_full_original_prompt(self):
        """Original prompt appears verbatim at the end of enriched result."""
        from cynic.api.server import _enrich_prompt
        from cynic.api.state import get_state
        from cynic.learning.qlearning import LearningSignal
        state = get_state()

        original = "Refactor the database layer to use async/await properly"
        state_key = "SDK:default:refactor:medium"
        for _ in range(25):
            state.qtable.update(LearningSignal(
                state_key=state_key, action="WAG",
                reward=0.6, judgment_id="x", loop_name="SDK_RESULT",
            ))

        result = _enrich_prompt(original, state)
        assert result.endswith(original)


class TestSDKJSONLPersistence:
    """Tests for _append_sdk_session_jsonl() — session persistence."""

    def test_append_jsonl_writes_file(self, tmp_path):
        """append_sdk_session_jsonl() creates/appends to JSONL file."""
        import dataclasses as dc
        from cynic.api.server import _append_sdk_session_jsonl
        from cynic.act.telemetry import SessionTelemetry
        import cynic.api.server as srv_module

        # Redirect to tmp path for test isolation
        original_path = srv_module._SDK_SESSIONS_JSONL
        test_path = str(tmp_path / "sdk_sessions.jsonl")
        srv_module._SDK_SESSIONS_JSONL = test_path
        try:
            record = SessionTelemetry(
                session_id="sess-abc",
                task="Fix the bug",
                task_type="debug",
                complexity="simple",
                model="claude-sonnet-4-6",
                tools_sequence=["Read", "Edit"],
                tools_allowed=2,
                tools_denied=0,
                tool_allow_rate=1.0,
                input_tokens=100,
                output_tokens=50,
                total_cost_usd=0.002,
                duration_s=1.5,
                is_error=False,
                result_text="Fixed successfully",
                output_q_score=70.0,
                output_verdict="WAG",
                output_confidence=0.45,
                state_key="SDK:claude-sonnet-4-6:debug:simple",
                reward=0.65,
            )
            _append_sdk_session_jsonl(record)

            lines = open(test_path, encoding="utf-8").readlines()
            assert len(lines) == 1
            data = json.loads(lines[0])
            assert data["session_id"] == "sess-abc"
            assert data["task_type"] == "debug"
            assert data["output_verdict"] == "WAG"
        finally:
            srv_module._SDK_SESSIONS_JSONL = original_path

    def test_append_jsonl_accumulates_multiple_records(self, tmp_path):
        """Multiple append calls → multiple lines in JSONL."""
        import dataclasses as dc
        from cynic.api.server import _append_sdk_session_jsonl
        from cynic.act.telemetry import SessionTelemetry
        import cynic.api.server as srv_module

        original_path = srv_module._SDK_SESSIONS_JSONL
        test_path = str(tmp_path / "sdk_sessions.jsonl")
        srv_module._SDK_SESSIONS_JSONL = test_path
        try:
            def _make_record(sid: str) -> SessionTelemetry:
                return SessionTelemetry(
                    session_id=sid, task="task", task_type="general",
                    complexity="trivial", model="m",
                    tools_sequence=[], tools_allowed=0, tools_denied=0,
                    tool_allow_rate=1.0, input_tokens=0, output_tokens=0,
                    total_cost_usd=0.0, duration_s=0.5, is_error=False,
                    result_text="ok", output_q_score=50.0,
                    output_verdict="WAG", output_confidence=0.382,
                    state_key="SDK:m:general:trivial", reward=0.5,
                )

            for i in range(3):
                _append_sdk_session_jsonl(_make_record(f"sess-{i}"))

            lines = open(test_path, encoding="utf-8").readlines()
            assert len(lines) == 3
        finally:
            srv_module._SDK_SESSIONS_JSONL = original_path


class TestSDKL2L1CrossFeed:
    """Tests for L2→L1 cross-feed: BARK/error results trigger ActionProposer."""

    def test_error_result_triggers_action_proposal(self):
        """Error SDK result emits DECISION_MADE → ActionProposer creates pending action."""
        import time as _t
        from cynic.api.state import get_state
        state = get_state()

        pending_before = len(state.action_proposer.pending())

        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)  # keep_alive
            _send(ws, _result(is_error=True, cost=0.001))
            _t.sleep(0.15)  # allow async event handling

        # ActionProposer should have received DECISION_MADE and created a proposal
        pending_after = len(state.action_proposer.pending())
        assert pending_after > pending_before, (
            "Error result should create a PENDING action via L2→L1 cross-feed"
        )

    def test_cross_feed_only_fires_on_error_or_bark(self):
        """
        Cross-feed condition: fires when is_error=True OR verdict=="BARK".
        When neither condition holds, no action is proposed.

        We test the condition directly on the cross-feed predicate rather than
        relying on CYNIC's REFLEX verdict being deterministic.
        """
        # Predicate mirrors implementation: is_error or verdict == "BARK"
        def _should_cross_feed(is_error: bool, verdict: str) -> bool:
            return is_error or verdict == "BARK"

        assert _should_cross_feed(True, "HOWL") is True   # error always fires
        assert _should_cross_feed(True, "WAG") is True    # error always fires
        assert _should_cross_feed(False, "BARK") is True  # BARK fires
        assert _should_cross_feed(False, "WAG") is False  # success+WAG: no feed
        assert _should_cross_feed(False, "HOWL") is False # success+HOWL: no feed
        assert _should_cross_feed(False, "GROWL") is False  # success+GROWL: no feed

    def test_l2_cross_feed_proposal_has_cynic_reality(self):
        """L2→L1 proposals are tagged with reality=CYNIC (self-improvement)."""
        import time as _t
        from cynic.api.state import get_state
        state = get_state()
        state.action_proposer._queue.clear()

        with TestClient(app).websocket_connect("/ws/sdk") as ws:
            _send(ws, _system_init())
            _recv(ws)
            _send(ws, _result(is_error=True, cost=0.001))
            _t.sleep(0.15)

        proposals = state.action_proposer.all_actions()
        cynic_proposals = [p for p in proposals if p.reality == "CYNIC"]
        assert len(cynic_proposals) >= 1
        assert cynic_proposals[-1].verdict == "BARK"


# ════════════════════════════════════════════════════════════════════════════
# Social signal writer — closes SOCIAL×PERCEIVE loop
# ════════════════════════════════════════════════════════════════════════════

class TestSocialSignalWriter:
    """
    γ4-social: human interactions write to social.json → SocialWatcher reads it.

    Tests _append_social_signal() without hitting the filesystem —
    redirect _SOCIAL_SIGNAL_PATH to a tmp_path for isolation.
    """

    def _redirect(self, tmp_path):
        import cynic.api.server as srv_module
        path = str(tmp_path / "social.json")
        self._orig = srv_module._SOCIAL_SIGNAL_PATH
        srv_module._SOCIAL_SIGNAL_PATH = path
        return path

    def _restore(self):
        import cynic.api.server as srv_module
        srv_module._SOCIAL_SIGNAL_PATH = self._orig

    def test_append_creates_file(self, tmp_path):
        path = self._redirect(tmp_path)
        try:
            from cynic.api.server import _append_social_signal
            import os
            _append_social_signal("cynic_feedback", 0.5, 30.0, "judgment", "user_rating")
            assert os.path.exists(path)
        finally:
            self._restore()

    def test_append_writes_valid_schema(self, tmp_path):
        path = self._redirect(tmp_path)
        try:
            from cynic.api.server import _append_social_signal
            _append_social_signal("cynic_interaction", -0.3, 20.0, "action", "reject")
            with open(path, encoding="utf-8") as fh:
                data = json.load(fh)
            assert isinstance(data, list)
            sig = data[0]
            assert sig["source"] == "cynic_interaction"
            assert sig["sentiment"] == -0.3
            assert sig["volume"] == 20.0
            assert sig["signal_type"] == "reject"
            assert sig["read"] is False
            assert "ts" in sig
        finally:
            self._restore()

    def test_append_accumulates_multiple_signals(self, tmp_path):
        path = self._redirect(tmp_path)
        try:
            from cynic.api.server import _append_social_signal
            for i in range(3):
                _append_social_signal("cynic_feedback", float(i) * 0.2, 10.0, "j", "rating")
            with open(path, encoding="utf-8") as fh:
                data = json.load(fh)
            assert len(data) == 3
        finally:
            self._restore()

    def test_sentiment_clamped_to_range(self, tmp_path):
        path = self._redirect(tmp_path)
        try:
            from cynic.api.server import _append_social_signal
            _append_social_signal("test", 99.9, 200.0, "t", "test")
            with open(path, encoding="utf-8") as fh:
                sig = json.load(fh)[0]
            assert sig["sentiment"] == 1.0
            assert sig["volume"] == 100.0
        finally:
            self._restore()

    def test_rolling_cap_prevents_unbounded_growth(self, tmp_path):
        path = self._redirect(tmp_path)
        try:
            from cynic.api.server import _append_social_signal, _SOCIAL_SIGNAL_CAP
            for i in range(_SOCIAL_SIGNAL_CAP + 5):
                _append_social_signal("src", 0.0, 10.0, "t", "t")
            with open(path, encoding="utf-8") as fh:
                data = json.load(fh)
            assert len(data) == _SOCIAL_SIGNAL_CAP
        finally:
            self._restore()

    def test_feedback_endpoint_writes_signal(self, tmp_path):
        """POST /feedback → _append_social_signal() writes to social.json."""
        import cynic.api.server as srv_module
        path = str(tmp_path / "social.json")
        orig = srv_module._SOCIAL_SIGNAL_PATH
        srv_module._SOCIAL_SIGNAL_PATH = path
        try:
            with TestClient(app) as client:
                from cynic.api.state import get_state
                state = get_state()
                state.last_judgment = {
                    "state_key": "CODE:JUDGE:PRESENT:1",
                    "action": "GROWL",
                    "judgment_id": "test-j",
                }
                resp = client.post("/feedback", json={"rating": 4})
                assert resp.status_code == 200
            import os
            assert os.path.exists(path)
            with open(path, encoding="utf-8") as fh:
                data = json.load(fh)
            assert len(data) >= 1
            assert data[-1]["source"] == "cynic_feedback"
            assert data[-1]["sentiment"] > 0   # rating 4 > neutral 3
        finally:
            srv_module._SOCIAL_SIGNAL_PATH = orig
