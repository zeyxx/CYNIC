"""
CYNIC API Tests — FastAPI server endpoint coverage

Tests the HTTP bridge without a real DB or LLM.
All Dogs are non-LLM (GUARDIAN/ANALYST/JANITOR/CYNIC) → pure Python, no external deps.

Pattern: httpx.AsyncClient with app directly (no real server needed).
"""
from __future__ import annotations

import pytest
import pytest_asyncio
from httpx import AsyncClient, ASGITransport

from cynic.api.server import app
from cynic.api.state import build_kernel, set_state
from cynic.core.phi import PHI


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest_asyncio.fixture(autouse=True)
async def kernel_state():
    """Build and wire kernel before each test. No DB, no LLM."""
    state = build_kernel(db_pool=None)
    set_state(state)
    yield state
    state.learning_loop.stop()


@pytest_asyncio.fixture
async def client():
    """HTTP client that calls the ASGI app directly (no network)."""
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
    ) as c:
        yield c


# ════════════════════════════════════════════════════════════════════════════
# ROOT
# ════════════════════════════════════════════════════════════════════════════

class TestRoot:
    async def test_root_returns_alive(self, client):
        resp = await client.get("/")
        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "alive"
        assert data["name"] == "CYNIC Kernel"
        assert "φ" in data

    async def test_root_lists_routes(self, client):
        resp = await client.get("/")
        routes = resp.json()["routes"]
        assert "/judge" in routes
        assert "/perceive" in routes
        assert "/learn" in routes
        assert "/health" in routes
        assert "/stats" in routes
        assert "/ws/stream" in routes


# ════════════════════════════════════════════════════════════════════════════
# GET /health
# ════════════════════════════════════════════════════════════════════════════

class TestHealth:
    async def test_health_alive(self, client):
        resp = await client.get("/health")
        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "alive"
        assert data["uptime_s"] >= 0
        assert data["phi"] == pytest.approx(1.618, abs=0.001)

    async def test_health_has_dogs(self, client):
        resp = await client.get("/health")
        dogs = resp.json()["dogs"]
        assert len(dogs) >= 4  # cynic, guardian, analyst, janitor

    async def test_health_learning_active(self, client):
        resp = await client.get("/health")
        learning = resp.json()["learning"]
        assert learning["active"] is True
        assert learning["states"] == 0  # fresh start
        assert learning["total_updates"] == 0

    async def test_health_has_scheduler_field(self, client):
        """Scheduler stats appear in /health — blind spot fixed."""
        resp = await client.get("/health")
        data = resp.json()
        assert "scheduler" in data
        sched = data["scheduler"]
        assert "running" in sched
        assert "workers_per_level" in sched
        assert "queues" in sched

    async def test_health_scheduler_worker_counts(self, client):
        """Scheduler worker counts are φ-derived."""
        resp = await client.get("/health")
        workers = resp.json()["scheduler"]["workers_per_level"]
        assert workers["REFLEX"] == 5   # F(5)
        assert workers["MICRO"]  == 3   # F(4)
        assert workers["MACRO"]  == 2   # F(3)
        assert workers["META"]   == 1


# ════════════════════════════════════════════════════════════════════════════
# POST /judge
# ════════════════════════════════════════════════════════════════════════════

class TestJudge:
    async def test_judge_basic_code(self, client):
        resp = await client.post("/judge", json={
            "content": "def add(a, b): return a + b",
            "reality": "CODE",
            "level": "REFLEX",
        })
        assert resp.status_code == 200
        data = resp.json()
        assert "judgment_id" in data
        assert data["verdict"] in {"HOWL", "WAG", "GROWL", "BARK"}
        assert 0.0 <= data["q_score"] <= 100.0
        assert 0.0 <= data["confidence"] <= 0.618  # φ-bound

    async def test_judge_confidence_phi_bounded(self, client):
        """Confidence must never exceed φ⁻¹ = 0.618."""
        for _ in range(5):
            resp = await client.post("/judge", json={
                "content": "x = 1",
                "reality": "CODE",
                "level": "REFLEX",
            })
            assert resp.status_code == 200
            assert resp.json()["confidence"] <= 0.618 + 1e-6

    async def test_judge_returns_dog_votes(self, client):
        resp = await client.post("/judge", json={
            "content": {"price": 0.042, "volume": 1000},
            "reality": "MARKET",
            "level": "REFLEX",
        })
        assert resp.status_code == 200
        votes = resp.json()["dog_votes"]
        assert len(votes) > 0  # at least one dog voted

    async def test_judge_all_realities(self, client):
        realities = ["CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"]
        for r in realities:
            resp = await client.post("/judge", json={
                "content": f"test content for {r}",
                "reality": r,
                "level": "REFLEX",
            })
            assert resp.status_code == 200, f"Failed for reality={r}"
            assert resp.json()["verdict"] in {"HOWL", "WAG", "GROWL", "BARK"}

    async def test_judge_invalid_reality_rejected(self, client):
        resp = await client.post("/judge", json={
            "content": "test",
            "reality": "INVALID",
        })
        assert resp.status_code == 422  # Pydantic validation error

    async def test_judge_micro_level(self, client):
        resp = await client.post("/judge", json={
            "content": "suspicious_content = True",
            "reality": "CODE",
            "level": "MICRO",
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["level_used"] == "MICRO"
        assert data["confidence"] <= 0.618 + 1e-6

    async def test_judge_duration_recorded(self, client):
        resp = await client.post("/judge", json={
            "content": "hello world",
            "reality": "HUMAN",
            "level": "REFLEX",
        })
        assert resp.status_code == 200
        assert resp.json()["duration_ms"] >= 0


# ════════════════════════════════════════════════════════════════════════════
# POST /perceive
# ════════════════════════════════════════════════════════════════════════════

class TestPerceive:
    async def test_perceive_from_js_hook(self, client):
        """Simulate a JS thin hook posting to /perceive."""
        resp = await client.post("/perceive", json={
            "source": "observe.js",
            "reality": "CODE",
            "data": {"file": "test.py", "event": "file_changed"},
            "context": "File changed in CODE reality",
            "run_judgment": True,
            "level": "REFLEX",
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["source"] == "observe.js"
        assert data["reality"] == "CODE"
        assert data["judgment"] is not None
        assert data["judgment"]["verdict"] in {"HOWL", "WAG", "GROWL", "BARK"}

    async def test_perceive_no_judgment(self, client):
        """run_judgment=False just acknowledges receipt."""
        resp = await client.post("/perceive", json={
            "source": "heartbeat",
            "reality": "CYNIC",
            "data": {"tick": 1},
            "run_judgment": False,
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["enqueued"] is True
        assert data["judgment"] is None

    async def test_perceive_has_cell_id(self, client):
        resp = await client.post("/perceive", json={
            "source": "test",
            "reality": "CYNIC",
            "data": "ping",
        })
        assert resp.status_code == 200
        assert len(resp.json()["cell_id"]) == 36  # UUID4

    async def test_perceive_all_realities(self, client):
        realities = ["CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"]
        for r in realities:
            resp = await client.post("/perceive", json={
                "source": f"hook:{r}",
                "reality": r,
                "data": {"event": "test"},
                "run_judgment": True,
                "level": "REFLEX",
            })
            assert resp.status_code == 200, f"Failed for reality={r}"
            assert resp.json()["judgment"]["verdict"] in {"HOWL", "WAG", "GROWL", "BARK"}

    async def test_perceive_q_score_phi_bounded(self, client):
        resp = await client.post("/perceive", json={
            "source": "hook:UserPromptSubmit",
            "reality": "HUMAN",
            "data": {"prompt": "Write a function that adds two numbers"},
            "run_judgment": True,
        })
        assert resp.status_code == 200
        q = resp.json()["judgment"]["q_score"]
        assert 0.0 <= q <= 100.0

    async def test_perceive_confidence_phi_bounded(self, client):
        resp = await client.post("/perceive", json={
            "source": "hook:PostToolUse",
            "reality": "CODE",
            "data": {"tool": "Bash", "command": "ls"},
            "run_judgment": True,
            "level": "REFLEX",
        })
        assert resp.status_code == 200
        assert resp.json()["judgment"]["confidence"] <= 0.618 + 1e-6

    async def test_perceive_sets_last_judgment(self, client, kernel_state):
        assert kernel_state.last_judgment is None
        await client.post("/perceive", json={
            "source": "hook:UserPromptSubmit",
            "reality": "HUMAN",
            "data": {"prompt": "hello"},
            "run_judgment": True,
        })
        assert kernel_state.last_judgment is not None
        assert kernel_state.last_judgment["action"] in {"HOWL", "WAG", "GROWL", "BARK"}

    async def test_perceive_no_judgment_skips_last_judgment(self, client, kernel_state):
        initial = kernel_state.last_judgment
        await client.post("/perceive", json={
            "source": "heartbeat",
            "reality": "CYNIC",
            "data": {"tick": 42},
            "run_judgment": False,
        })
        assert kernel_state.last_judgment == initial

    async def test_perceive_closes_feedback_loop(self, client, kernel_state):
        """perceive → last_judgment → feedback → Q update."""
        await client.post("/perceive", json={
            "source": "hook:UserPromptSubmit",
            "reality": "HUMAN",
            "data": {"prompt": "build something"},
            "run_judgment": True,
        })
        state_key = kernel_state.last_judgment["state_key"]
        resp = await client.post("/feedback", json={"rating": 5})
        assert resp.status_code == 200
        assert resp.json()["state_key"] == state_key
        assert resp.json()["q_value"] > 0.5

    async def test_perceive_micro_level(self, client):
        resp = await client.post("/perceive", json={
            "source": "hook:PostToolUse",
            "reality": "CODE",
            "data": {"file": "test.py"},
            "run_judgment": True,
            "level": "MICRO",
        })
        assert resp.status_code == 200
        assert resp.json()["judgment"]["level_used"] == "MICRO"

    async def test_perceive_data_as_string(self, client):
        resp = await client.post("/perceive", json={
            "source": "hook:Notification",
            "reality": "CYNIC",
            "data": "System notification: test complete",
            "run_judgment": True,
        })
        assert resp.status_code == 200
        assert resp.json()["judgment"] is not None

    async def test_perceive_data_as_number(self, client):
        resp = await client.post("/perceive", json={
            "source": "market-watcher",
            "reality": "MARKET",
            "data": 0.04237,
            "context": "price tick",
            "run_judgment": True,
        })
        assert resp.status_code == 200
        assert resp.json()["judgment"]["verdict"] in {"HOWL", "WAG", "GROWL", "BARK"}

    async def test_perceive_cell_ids_unique(self, client):
        ids = set()
        for _ in range(5):
            resp = await client.post("/perceive", json={
                "source": "test",
                "reality": "CYNIC",
                "data": "ping",
                "run_judgment": False,
            })
            assert resp.status_code == 200
            ids.add(resp.json()["cell_id"])
        assert len(ids) == 5

    async def test_perceive_message_contains_verdict(self, client):
        resp = await client.post("/perceive", json={
            "source": "test",
            "reality": "HUMAN",
            "data": {"prompt": "test"},
            "run_judgment": True,
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["judgment"]["verdict"] in data["message"]

    async def test_perceive_has_dog_votes(self, client):
        resp = await client.post("/perceive", json={
            "source": "test",
            "reality": "CODE",
            "data": "def foo(): pass",
            "run_judgment": True,
            "level": "REFLEX",
        })
        assert resp.status_code == 200
        votes = resp.json()["judgment"]["dog_votes"]
        assert isinstance(votes, dict)
        assert len(votes) > 0


# ════════════════════════════════════════════════════════════════════════════
# POST /learn
# ════════════════════════════════════════════════════════════════════════════

class TestLearn:
    async def test_learn_updates_qtable(self, client, kernel_state):
        resp = await client.post("/learn", json={
            "state_key": "CODE:JUDGE:PRESENT:1",
            "action": "WAG",
            "reward": 0.8,
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["state_key"] == "CODE:JUDGE:PRESENT:1"
        assert data["action"] == "WAG"
        assert data["visits"] == 1
        assert 0.5 < data["q_value"] < 1.0  # Positive reward → Q > neutral 0.5

    async def test_learn_invalid_action_rejected(self, client):
        resp = await client.post("/learn", json={
            "state_key": "CODE:JUDGE:PRESENT:1",
            "action": "WOOF",  # not a valid verdict
            "reward": 0.5,
        })
        assert resp.status_code == 422

    async def test_learn_reward_out_of_range_rejected(self, client):
        resp = await client.post("/learn", json={
            "state_key": "CODE:JUDGE:PRESENT:1",
            "action": "WAG",
            "reward": 1.5,  # > 1.0
        })
        assert resp.status_code == 422

    async def test_learn_confidence_grows_with_visits(self, client, kernel_state):
        """More signals → higher confidence (capped at φ⁻¹)."""
        state_key = "CYNIC:LEARN:PRESENT:2"
        confidences = []
        for i in range(10):
            resp = await client.post("/learn", json={
                "state_key": state_key,
                "action": "WAG",
                "reward": 0.7,
            })
            assert resp.status_code == 200
            confidences.append(resp.json()["confidence"])

        # Confidence should grow
        assert confidences[-1] >= confidences[0]
        # But never exceed φ⁻¹
        assert all(c <= 0.618 + 1e-6 for c in confidences)


# ════════════════════════════════════════════════════════════════════════════
# GET /policy/{state_key}
# ════════════════════════════════════════════════════════════════════════════

class TestPolicy:
    async def test_policy_exploit_unseen_state(self, client):
        """Unseen state → cautious default (GROWL)."""
        resp = await client.get("/policy/UNKNOWN:STATE:KEY", params={"mode": "exploit"})
        assert resp.status_code == 200
        data = resp.json()
        assert data["recommended_action"] == "GROWL"  # cautious default
        assert data["mode"] == "exploit"

    async def test_policy_explore_returns_action(self, client):
        resp = await client.get("/policy/CODE:JUDGE:PRESENT:1", params={"mode": "explore"})
        assert resp.status_code == 200
        data = resp.json()
        assert data["recommended_action"] in {"BARK", "GROWL", "WAG", "HOWL"}

    async def test_policy_shows_all_actions(self, client):
        resp = await client.get("/policy/CODE:JUDGE:PRESENT:1")
        assert resp.status_code == 200
        top = resp.json()["top_actions"]
        assert len(top) == 4  # all 4 verdicts

    async def test_policy_invalid_mode_rejected(self, client):
        resp = await client.get("/policy/some:key", params={"mode": "random"})
        assert resp.status_code == 422

    async def test_policy_improves_after_learning(self, client, kernel_state):
        """Policy should prefer WAG after repeated positive WAG signals."""
        state_key = "CODE:JUDGE:FUTURE:1"

        # Train: WAG is good
        for _ in range(20):
            await client.post("/learn", json={
                "state_key": state_key,
                "action": "WAG",
                "reward": 0.9,
            })
        # Train: BARK is bad
        for _ in range(20):
            await client.post("/learn", json={
                "state_key": state_key,
                "action": "BARK",
                "reward": 0.1,
            })

        resp = await client.get(f"/policy/{state_key}", params={"mode": "exploit"})
        assert resp.status_code == 200
        assert resp.json()["recommended_action"] == "WAG"


# ════════════════════════════════════════════════════════════════════════════
# GET /stats
# ════════════════════════════════════════════════════════════════════════════

class TestStats:
    async def test_stats_structure(self, client):
        resp = await client.get("/stats")
        assert resp.status_code == 200
        data = resp.json()
        assert "judgments" in data
        assert "learning" in data
        assert "top_states" in data
        assert "consciousness" in data

    async def test_stats_updates_after_judgment(self, client, kernel_state):
        # Before
        r1 = await client.get("/stats")
        count_before = r1.json()["judgments"]["judgments_total"]

        # Judge something
        await client.post("/judge", json={
            "content": "x = 1",
            "reality": "CODE",
            "level": "REFLEX",
        })

        # After
        r2 = await client.get("/stats")
        count_after = r2.json()["judgments"]["judgments_total"]
        assert count_after == count_before + 1


# ════════════════════════════════════════════════════════════════════════════
# POST /feedback
# ════════════════════════════════════════════════════════════════════════════

class TestFeedback:
    async def test_feedback_without_judgment_returns_404(self, client, kernel_state):
        """No prior judgment → 404."""
        kernel_state.last_judgment = None
        resp = await client.post("/feedback", json={"rating": 3})
        assert resp.status_code == 404

    async def test_feedback_rating_5_gives_high_reward(self, client, kernel_state):
        """rating=5 → reward=0.9 → Q-value rises above neutral 0.5."""
        kernel_state.last_judgment = {
            "state_key": "HUMAN:PERCEIVE:PRESENT:0",
            "action": "WAG",
            "judgment_id": "test-123",
        }
        resp = await client.post("/feedback", json={"rating": 5})
        assert resp.status_code == 200
        data = resp.json()
        assert data["reward"] == pytest.approx(0.9, abs=0.01)
        assert data["q_value"] > 0.5
        assert data["visits"] == 1
        assert data["state_key"] == "HUMAN:PERCEIVE:PRESENT:0"

    async def test_feedback_rating_1_gives_low_reward(self, client, kernel_state):
        """rating=1 → reward=0.1 → Q-value falls below neutral 0.5."""
        kernel_state.last_judgment = {
            "state_key": "HUMAN:PERCEIVE:PRESENT:0",
            "action": "HOWL",
            "judgment_id": "test-456",
        }
        resp = await client.post("/feedback", json={"rating": 1})
        assert resp.status_code == 200
        data = resp.json()
        assert data["reward"] == pytest.approx(0.1, abs=0.01)
        assert data["q_value"] < 0.5

    async def test_feedback_rating_3_is_neutral(self, client, kernel_state):
        """rating=3 → reward=0.5 → Q-value stays near neutral."""
        kernel_state.last_judgment = {
            "state_key": "CODE:JUDGE:PRESENT:1",
            "action": "GROWL",
            "judgment_id": "test-789",
        }
        resp = await client.post("/feedback", json={"rating": 3})
        assert resp.status_code == 200
        data = resp.json()
        assert data["reward"] == pytest.approx(0.5, abs=0.01)
        # TD(0): Q stays near 0.5 with neutral reward
        assert 0.45 < data["q_value"] < 0.55

    async def test_feedback_reward_never_reaches_extremes(self, client, kernel_state):
        """φ-aligned: reward is always in [0.1, 0.9] — LAW OF DOUBT."""
        kernel_state.last_judgment = {
            "state_key": "CYNIC:LEARN:PRESENT:0",
            "action": "BARK",
            "judgment_id": "test-000",
        }
        for rating in [1, 2, 3, 4, 5]:
            resp = await client.post("/feedback", json={"rating": rating})
            assert resp.status_code == 200
            data = resp.json()
            assert 0.09 < data["reward"] < 0.91, f"rating={rating} produced extreme reward={data['reward']}"

    async def test_feedback_invalid_rating_rejected(self, client):
        """rating=0 or rating=6 should be rejected (Pydantic validation)."""
        resp = await client.post("/feedback", json={"rating": 0})
        assert resp.status_code == 422
        resp = await client.post("/feedback", json={"rating": 6})
        assert resp.status_code == 422


# ════════════════════════════════════════════════════════════════════════════
# GET /introspect  (MetaCognition — composant 9/9)
# ════════════════════════════════════════════════════════════════════════════

class TestIntrospect:
    async def test_introspect_returns_200(self, client):
        resp = await client.get("/introspect")
        assert resp.status_code == 200

    async def test_introspect_has_phi_self_assessment(self, client):
        """φ self-assessment is the meta-cognitive core."""
        resp = await client.get("/introspect")
        data = resp.json()
        phi = data["φ_self_assessment"]
        assert "kernel_integrity" in phi
        assert "verdict" in phi
        assert phi["verdict"] in {"HOWL", "WAG", "GROWL", "BARK"}
        assert 0.0 <= phi["self_confidence"] <= 0.618  # φ-bounded

    async def test_introspect_has_nine_components(self, client):
        """All 9 kernel components tracked."""
        resp = await client.get("/introspect")
        components = resp.json()["components"]
        assert len(components) == 9
        for key in components:
            assert "status" in components[key]
            assert "description" in components[key]

    async def test_introspect_9_of_9_active_fresh_start(self, client):
        """On fresh kernel, MetaCognition itself is ACTIVE."""
        resp = await client.get("/introspect")
        components = resp.json()["components"]
        assert components["9_META_COGNITION"]["status"] == "ACTIVE"

    async def test_introspect_has_residual_stats(self, client):
        resp = await client.get("/introspect")
        residual = resp.json()["residual"]
        assert "observations" in residual
        assert "patterns_detected" in residual
        assert "anomaly_rate" in residual

    async def test_introspect_has_scholar_status(self, client):
        resp = await client.get("/introspect")
        scholar = resp.json()["scholar"]
        assert "buffer_size" in scholar
        assert "hit_rate" in scholar
        assert 0.0 <= scholar.get("buffer_richness", 0) <= 1.0

    async def test_introspect_residual_grows_after_judgments(self, client):
        """After judgments flow through, residual detector observes them."""
        # Run a few judgments
        for _ in range(3):
            await client.post("/judge", json={
                "content": "x = 1",
                "reality": "CODE",
                "level": "REFLEX",
            })

        resp = await client.get("/introspect")
        data = resp.json()
        assert data["residual"]["observations"] >= 3

    async def test_introspect_scholar_grows_after_judgments(self, client):
        """Scholar buffer fills as judgments flow."""
        for _ in range(5):
            await client.post("/judge", json={
                "content": f"def func_{_}(): pass",
                "reality": "CODE",
                "level": "REFLEX",
            })

        resp = await client.get("/introspect")
        scholar = resp.json()["scholar"]
        assert scholar["buffer_size"] >= 5  # At least 5 entries learned

    async def test_introspect_dogs_listed(self, client):
        """All registered Dogs appear in dogs status."""
        resp = await client.get("/introspect")
        dogs = resp.json()["dogs"]
        assert "CYNIC" in dogs
        assert "GUARDIAN" in dogs
        assert "ANALYST" in dogs
        assert "JANITOR" in dogs

    async def test_introspect_uptime_nonnegative(self, client):
        resp = await client.get("/introspect")
        assert resp.json()["uptime_s"] >= 0.0


# ════════════════════════════════════════════════════════════════════════════
# WS /ws/stream  (real-time event stream)
# ════════════════════════════════════════════════════════════════════════════

class TestWebSocketStream:
    """
    WebSocket tests use starlette.testclient.TestClient (sync) rather than
    httpx AsyncClient, because WebSocket testing requires a sync interface.

    The module-level kernel_state autouse fixture is async and only applies
    to async tests. These sync tests use ws_kernel_sync for state setup.
    """

    @pytest.fixture(autouse=True)
    def ws_kernel_sync(self):
        """Sync kernel setup for WebSocket tests — TestClient owns its event loop."""
        state = build_kernel(db_pool=None)
        set_state(state)
        yield
        state.learning_loop.stop()

    def test_ws_sends_connected_on_open(self):
        """WebSocket /ws/stream immediately sends 'connected' after handshake."""
        from starlette.testclient import TestClient
        with TestClient(app).websocket_connect("/ws/stream") as ws:
            msg = ws.receive_json()
        assert msg["type"] == "connected"
        assert "ts" in msg
        assert "phi" in msg

    def test_ws_phi_is_golden_ratio(self):
        """Connected message includes φ = 1.618..."""
        from starlette.testclient import TestClient
        with TestClient(app).websocket_connect("/ws/stream") as ws:
            msg = ws.receive_json()
        assert msg["type"] == "connected"
        assert abs(msg["phi"] - PHI) < 0.001

    def test_ws_closes_cleanly(self):
        """Disconnecting does not raise an exception."""
        from starlette.testclient import TestClient
        try:
            with TestClient(app).websocket_connect("/ws/stream") as ws:
                ws.receive_json()  # consume "connected"
        except Exception as exc:
            pytest.fail(f"WebSocket close raised unexpectedly: {exc}")

    def test_ws_receives_ping_and_sends_pong(self):
        """Client sends {type: ping}, server responds {type: pong} immediately."""
        from starlette.testclient import TestClient
        with TestClient(app).websocket_connect("/ws/stream") as ws:
            ws.receive_json()  # consume "connected"
            ws.send_json({"type": "ping"})
            response = ws.receive_json()
        assert response["type"] == "pong"
        assert "ts" in response


# ════════════════════════════════════════════════════════════════════════════
# GET /axioms  (δ1 ε-wiring: AxiomMonitor in kernel)
# ════════════════════════════════════════════════════════════════════════════

class TestAxiomsEndpoint:
    async def test_axioms_returns_200(self, client):
        resp = await client.get("/axioms")
        assert resp.status_code == 200

    async def test_axioms_has_required_keys(self, client):
        data = (await client.get("/axioms")).json()
        assert "active_count" in data
        assert "total_signals" in data
        assert "tier" in data
        assert "axioms" in data

    async def test_axioms_tier_dormant_initially(self, client):
        data = (await client.get("/axioms")).json()
        assert data["tier"] == "DORMANT"
        assert data["active_count"] == 0

    async def test_axioms_lists_all_four_emergent_axioms(self, client):
        data = (await client.get("/axioms")).json()
        for ax in ["AUTONOMY", "SYMBIOSIS", "EMERGENCE", "ANTIFRAGILITY"]:
            assert ax in data["axioms"]

    async def test_axioms_in_introspect(self, client):
        """Introspect now includes emergent_axioms stats."""
        data = (await client.get("/introspect")).json()
        assert "emergent_axioms" in data
        assert "active_count" in data["emergent_axioms"]
        assert "tier" in data["emergent_axioms"]


# ════════════════════════════════════════════════════════════════════════════
# GET /lod  (δ2 ε-wiring: LODController in kernel)
# ════════════════════════════════════════════════════════════════════════════

class TestLODEndpoint:
    async def test_lod_returns_200(self, client):
        resp = await client.get("/lod")
        assert resp.status_code == 200

    async def test_lod_has_required_keys(self, client):
        data = (await client.get("/lod")).json()
        assert "current_lod" in data
        assert "current_name" in data
        assert "allows_llm" in data
        assert "max_consciousness" in data
        assert "forced" in data

    async def test_lod_initially_full(self, client):
        data = (await client.get("/lod")).json()
        assert data["current_lod"] == 0
        assert data["current_name"] == "FULL"

    async def test_lod_allows_llm_initially(self, client):
        data = (await client.get("/lod")).json()
        assert data["allows_llm"] is True

    async def test_lod_in_introspect(self, client):
        """Introspect now includes LOD status."""
        data = (await client.get("/introspect")).json()
        assert "lod" in data
        assert "current_lod" in data["lod"]


# ════════════════════════════════════════════════════════════════════════════
# GET /mirror  (KernelMirror — Ring 3 unified self-reflection)
# ════════════════════════════════════════════════════════════════════════════

class TestMirrorEndpoint:
    async def test_mirror_returns_200(self, client):
        resp = await client.get("/mirror")
        assert resp.status_code == 200

    async def test_mirror_has_required_keys(self, client):
        data = (await client.get("/mirror")).json()
        for key in ("snapshot_id", "timestamp", "overall_health", "tier",
                    "qtable", "axioms", "lod", "sage", "dogs"):
            assert key in data, f"Missing mirror key: {key}"

    async def test_mirror_health_is_float_in_range(self, client):
        data = (await client.get("/mirror")).json()
        h = data["overall_health"]
        assert isinstance(h, float)
        assert 0.0 <= h <= 100.0

    async def test_mirror_tier_is_valid(self, client):
        data = (await client.get("/mirror")).json()
        assert data["tier"] in ("HOWL", "WAG", "GROWL", "BARK")

    async def test_mirror_includes_diff(self, client):
        """Two calls to /mirror → second includes diff."""
        await client.get("/mirror")  # first snapshot (no diff yet)
        data = (await client.get("/mirror")).json()
        assert "diff" in data
        assert isinstance(data["diff"], dict)

    async def test_mirror_snapshot_id_increments(self, client, kernel_state):
        """Persistent kernel_mirror → snapshot_id increments."""
        r1 = (await client.get("/mirror")).json()
        r2 = (await client.get("/mirror")).json()
        assert r2["snapshot_id"] == r1["snapshot_id"] + 1


# ════════════════════════════════════════════════════════════════════════════
# GET /consciousness  (unified metathinking output)
# ════════════════════════════════════════════════════════════════════════════

class TestConsciousnessEndpoint:
    async def test_consciousness_returns_200(self, client):
        resp = await client.get("/consciousness")
        assert resp.status_code == 200

    async def test_consciousness_has_mirror_key(self, client):
        data = (await client.get("/consciousness")).json()
        assert "mirror" in data

    async def test_consciousness_has_timestamp(self, client):
        data = (await client.get("/consciousness")).json()
        assert "timestamp" in data
        assert data["timestamp"] > 0

    async def test_consciousness_has_llm_routing(self, client):
        """llm_routing section present when router is wired."""
        data = (await client.get("/consciousness")).json()
        assert "llm_routing" in data

    async def test_consciousness_mirror_has_overall_health(self, client):
        data = (await client.get("/consciousness")).json()
        assert "overall_health" in data["mirror"]
        assert 0.0 <= data["mirror"]["overall_health"] <= 100.0


# ════════════════════════════════════════════════════════════════════════════
# GET /sdk/routing  (Ring 4 LLM router stats)
# ════════════════════════════════════════════════════════════════════════════

class TestSdkRoutingEndpoint:
    async def test_sdk_routing_returns_200(self, client):
        resp = await client.get("/sdk/routing")
        assert resp.status_code == 200

    async def test_sdk_routing_available_true(self, client):
        data = (await client.get("/sdk/routing")).json()
        assert data["available"] is True

    async def test_sdk_routing_has_phi_threshold(self, client):
        data = (await client.get("/sdk/routing")).json()
        assert "phi_threshold" in data
        assert abs(data["phi_threshold"] - 0.618) < 0.01

    async def test_sdk_routing_initial_rate_zero(self, client):
        data = (await client.get("/sdk/routing")).json()
        assert data["total_routes"] == 0
        assert data["routes_to_local"] == 0

    async def test_sdk_routing_has_task_types(self, client):
        data = (await client.get("/sdk/routing")).json()
        assert "simple_task_types" in data
        assert "debug" in data["simple_task_types"]
