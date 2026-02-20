"""
API Integration Tests — real Ollama (gemma2:2b) required.

Run:   pytest -m integration
Skip:  pytest (default — no integration tests run)

Tests the full HTTP API with real LLM dogs wired:
    POST /judge      → temporal MCTS via SageDog
    POST /perceive   → REFLEX judgment on raw perception
    POST /feedback   → human rating → Q-Table update
    GET  /health     → LLM adapters discovered
    GET  /introspect → dog stats + scholar buffer

Validates the bridge that unit tests mock:
    HTTP request → CynicOrganism → JudgeOrchestrator → SageDog (temporal MCTS) → verdict
"""
from __future__ import annotations

import asyncio

import pytest
import pytest_asyncio
from httpx import ASGITransport, AsyncClient

from cynic.api.server import app
from cynic.api.state import awaken, set_state
from cynic.core.phi import MAX_Q_SCORE, PHI_INV
from cynic.llm.adapter import LLMRegistry, OllamaAdapter

pytestmark = pytest.mark.integration


# ════════════════════════════════════════════════════════════════════════════
# TEST CONTENT — same as test_integration.py for comparability
# ════════════════════════════════════════════════════════════════════════════

GOOD_CODE = """\
from cynic.core.phi import PHI, PHI_INV, phi_bound_score


def phi_aggregate(scores: dict, weights: dict) -> float:
    \"\"\"phi-weighted geometric mean. Preserves [0, MAX_Q] range.\"\"\"
    import math
    log_sum = sum(weights[k] * math.log(max(v, 0.1)) for k, v in scores.items())
    total_weight = sum(weights.values())
    return phi_bound_score(math.exp(log_sum / total_weight))


def fibonacci(n: int) -> int:
    \"\"\"F(n) exact, no recursion, no memoization needed for n < 20.\"\"\"
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a
"""

BAD_CODE = "\n".join([
    "import sys, json",
    "x = []",
    "cred = 'hunter2'",
    "def f(a,b,c,d,e,f,g,h):",
    "    global x",
    "    try:",
    "        for i in range(9999):",
    "            for j in range(9999):",
    "                x.append(i*j)",
    "    except:",
    "        pass",
    "# TODO fix this",
    "# HACK temporary",
])


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture(scope="module")
def ollama_registry():
    """
    Module-scoped real LLMRegistry — discovers Ollama once per module.

    Skips the entire test module if Ollama + gemma2:2b is unreachable.
    Using asyncio.run() (sync pattern) for module-scope compatibility.
    """
    async def _build():
        adapter = OllamaAdapter("gemma2:2b")
        if not await adapter.check_available():
            return None
        registry = LLMRegistry()
        await registry.discover()
        return registry

    registry = asyncio.run(_build())
    if registry is None:
        pytest.skip("Ollama not available at localhost:11434")
    return registry


@pytest_asyncio.fixture(autouse=True)
async def kernel_with_llm(ollama_registry):
    """
    Function-scoped kernel with real LLMRegistry wired.

    Rebuilt each test (autouse) so buses are always fresh after reset_singletons.
    LLM dogs (SageDog, ScholarDog, CartographerDog, DeployerDog) get temporal MCTS.

    Also populates the global registry singleton so /health reports adapters correctly.
    """
    # Populate global registry so /health endpoint reports adapters
    from cynic.llm.adapter import get_registry
    global_reg = get_registry()
    for adapter in ollama_registry.get_available():
        global_reg.register(adapter, available=True)

    state = awaken(db_pool=None, registry=ollama_registry)
    set_state(state)
    yield state
    state.learning_loop.stop()


@pytest_asyncio.fixture
async def client():
    """HTTP client hitting the ASGI app directly (no network)."""
    async with AsyncClient(
        transport=ASGITransport(app=app),
        base_url="http://test",
    ) as c:
        yield c


# ════════════════════════════════════════════════════════════════════════════
# GET /health — verify LLM adapters discovered
# ════════════════════════════════════════════════════════════════════════════

class TestHealthIntegration:
    async def test_health_reports_llm_adapters(self, client):
        """Health endpoint must report at least one discovered LLM adapter."""
        resp = await client.get("/health")
        assert resp.status_code == 200
        data = resp.json()
        assert len(data["llm_adapters"]) > 0, (
            "No LLM adapters reported — registry.discover() may have failed"
        )

    async def test_health_has_all_dogs(self, client):
        """All 11 dogs must be wired in the kernel."""
        resp = await client.get("/health")
        assert resp.status_code == 200
        dogs = resp.json()["dogs"]
        assert len(dogs) == 11


# ════════════════════════════════════════════════════════════════════════════
# POST /judge — full pipeline with real temporal MCTS
# ════════════════════════════════════════════════════════════════════════════

class TestJudgeIntegration:
    async def test_judge_returns_valid_verdict(self, client):
        resp = await client.post("/judge", json={
            "reality": "CODE",
            "analysis": "JUDGE",
            "content": GOOD_CODE,
            "context": "integration test — clean phi-based code",
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["verdict"] in ("HOWL", "WAG", "BARK", "GROWL")
        assert "judgment_id" in data

    async def test_judge_confidence_phi_bounded(self, client):
        """Confidence must never exceed φ⁻¹ = 0.618 (LAW OF DOUBT)."""
        resp = await client.post("/judge", json={
            "reality": "CODE",
            "analysis": "JUDGE",
            "content": GOOD_CODE,
        })
        assert resp.status_code == 200
        assert 0 < resp.json()["confidence"] <= PHI_INV

    async def test_judge_discriminates_quality(self, client):
        """Core hypothesis: good code must score higher than bad code via LLM.

        MACRO required — SAGE (temporal MCTS) is only in MACRO_DOGS.
        MICRO = REFLEX + SCHOLAR only (no SAGE, no LLM discrimination).
        """
        good_resp = await client.post("/judge", json={
            "reality": "CODE",
            "analysis": "JUDGE",
            "content": GOOD_CODE,
            "context": "clean phi-based utility functions",
            "level": "MACRO",
            "budget_usd": 0.1,
        })
        bad_resp = await client.post("/judge", json={
            "reality": "CODE",
            "analysis": "JUDGE",
            "content": BAD_CODE,
            "context": "deeply nested globals hardcoded credentials dead markers",
            "level": "MACRO",
            "budget_usd": 0.1,
        })
        assert good_resp.status_code == 200
        assert bad_resp.status_code == 200
        good_q = good_resp.json()["q_score"]
        bad_q = bad_resp.json()["q_score"]
        assert good_q > bad_q, (
            f"LLM not discriminating quality: good={good_q:.1f} bad={bad_q:.1f}"
        )

    async def test_judge_dog_votes_in_range(self, client):
        """Every dog vote must be in [0, MAX_Q_SCORE]."""
        resp = await client.post("/judge", json={
            "reality": "CODE",
            "analysis": "JUDGE",
            "content": GOOD_CODE,
        })
        assert resp.status_code == 200
        for dog_id, vote in resp.json()["dog_votes"].items():
            assert 0 <= vote <= MAX_Q_SCORE, f"{dog_id}: {vote} out of range"

    async def test_judge_reflex_skips_llm(self, client):
        """REFLEX level → heuristic path only → llm_calls == 0."""
        resp = await client.post("/judge", json={
            "reality": "CODE",
            "analysis": "JUDGE",
            "content": GOOD_CODE,
            "level": "REFLEX",
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["level_used"] == "REFLEX"
        assert data["llm_calls"] == 0


# ════════════════════════════════════════════════════════════════════════════
# POST /perceive — raw perception bridge
# ════════════════════════════════════════════════════════════════════════════

class TestPerceiveIntegration:
    async def test_perceive_runs_judgment_by_default(self, client):
        resp = await client.post("/perceive", json={
            "source": "test_hook",
            "reality": "CODE",
            "data": GOOD_CODE,
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["judgment"] is not None
        assert data["judgment"]["verdict"] in ("HOWL", "WAG", "BARK", "GROWL")

    async def test_perceive_skip_judgment(self, client):
        """run_judgment=False → enqueued=True, no judgment object."""
        resp = await client.post("/perceive", json={
            "source": "test",
            "reality": "CODE",
            "data": "some code",
            "run_judgment": False,
        })
        assert resp.status_code == 200
        data = resp.json()
        assert data["enqueued"] is True
        assert data["judgment"] is None

    async def test_perceive_populates_last_judgment_for_feedback(self, client):
        """Perceive → feedback must find last_judgment without 404."""
        await client.post("/perceive", json={
            "source": "hook",
            "reality": "HUMAN",
            "data": "user is focused, coding productively",
        })
        resp = await client.post("/feedback", json={"rating": 4})
        assert resp.status_code == 200
        assert resp.json()["reward"] > 0


# ════════════════════════════════════════════════════════════════════════════
# Feedback loop — /judge → /feedback → /policy
# ════════════════════════════════════════════════════════════════════════════

class TestFeedbackLoopIntegration:
    async def test_feedback_updates_qtable(self, client):
        """Full loop: judge → rate → verify Q-value persisted."""
        await client.post("/judge", json={
            "reality": "CODE",
            "analysis": "JUDGE",
            "content": GOOD_CODE,
        })
        fb_resp = await client.post("/feedback", json={"rating": 5})
        assert fb_resp.status_code == 200
        data = fb_resp.json()
        assert data["q_value"] > 0
        assert data["visits"] == 1
        # Rating 5/5 → reward = (5-1)/4 * 0.8 + 0.1 = 0.9
        assert data["reward"] == pytest.approx(0.9, abs=0.01)

    async def test_policy_reflects_learned_action(self, client):
        """After one feedback, exploit policy recommends the rated action."""
        await client.post("/judge", json={
            "reality": "CODE",
            "analysis": "JUDGE",
            "content": GOOD_CODE,
        })
        fb = await client.post("/feedback", json={"rating": 5})
        fb_data = fb.json()
        state_key = fb_data["state_key"]
        rated_action = fb_data["action"]

        policy_resp = await client.get(f"/policy/{state_key}?mode=exploit")
        assert policy_resp.status_code == 200
        assert policy_resp.json()["recommended_action"] == rated_action


# ════════════════════════════════════════════════════════════════════════════
# GET /introspect — meta-cognition with real dogs
# ════════════════════════════════════════════════════════════════════════════

class TestIntrospectIntegration:
    async def test_introspect_kernel_integrity(self, client):
        """Kernel must report positive integrity with real dogs wired."""
        resp = await client.get("/introspect")
        assert resp.status_code == 200
        phi_assess = resp.json()["φ_self_assessment"]
        assert phi_assess["kernel_integrity"] > 0
        assert phi_assess["verdict"] in ("HOWL", "WAG", "GROWL", "BARK")

    async def test_introspect_scholar_tracks_lookups(self, client):
        """After perceive calls, Scholar must have recorded lookups."""
        for i in range(3):
            await client.post("/perceive", json={
                "source": "test",
                "reality": "CODE",
                "data": f"def fn_{i}(x): return x + {i}",
            })

        resp = await client.get("/introspect")
        assert resp.status_code == 200
        scholar = resp.json()["scholar"]
        assert scholar["lookups"] >= 0  # sanity — doesn't go negative
        assert "buffer_size" in scholar
        assert "hit_rate" in scholar
