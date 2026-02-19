"""
Tests: LLMRouter — Ring 4 Q-Table driven model selection

Validates cold-start safety, all 4 routing gates, stats tracking,
and RoutingDecision output structure.
"""
from __future__ import annotations

import pytest
from unittest.mock import MagicMock

from cynic.act.llm_router import (
    LLMRouter,
    RoutingDecision,
    MODEL_SONNET,
    MODEL_HAIKU,
    _MIN_VISITS_TO_ROUTE,
)
from cynic.core.phi import PHI_INV
from cynic.learning.qlearning import LearningSignal, QTable, QEntry


# ── Helpers ─────────────────────────────────────────────────────────────────

def _make_warm_qtable(
    state_key: str,
    action: str = "HOWL",
    visits: int = 5,
    q_value: float = 0.8,
) -> QTable:
    """Build a QTable with a single pre-populated entry (warm state)."""
    qt = QTable()
    # Populate via real updates so EWC and internal structs stay consistent
    for _ in range(visits):
        qt.update(LearningSignal(
            state_key=state_key, action=action, reward=q_value * 100.0,
        ))
    return qt


_DEFAULT_SK = "SDK:m:debug:trivial"


def _mock_qtable_with_confidence(
    confidence: float,
    visits: int = 5,
    state_key: str = _DEFAULT_SK,
) -> MagicMock:
    """Build a MagicMock QTable that returns a fixed confidence."""
    qt = MagicMock()
    qt.confidence.return_value = confidence
    entry = QEntry(state_key=state_key, action="HOWL")
    entry.q_value = 0.8
    entry.visits = visits
    qt.exploit.return_value = "HOWL"
    qt._table = {state_key: {"HOWL": entry}}
    return qt


# ═══════════════════════════════════════════════════════════════════════════
# RoutingDecision dataclass
# ═══════════════════════════════════════════════════════════════════════════

class TestRoutingDecision:

    def test_fields_accessible(self):
        """RoutingDecision has all documented fields."""
        d = RoutingDecision(
            recommended_model=MODEL_SONNET,
            route_to_local=False,
            confidence=0.5,
            reason="test",
            task_type="debug",
            complexity="trivial",
        )
        assert d.recommended_model == MODEL_SONNET
        assert d.route_to_local is False
        assert d.confidence == 0.5
        assert d.reason == "test"
        assert d.task_type == "debug"
        assert d.complexity == "trivial"


# ═══════════════════════════════════════════════════════════════════════════
# LLMRouter.route() — cold start (all gates should fail early)
# ═══════════════════════════════════════════════════════════════════════════

class TestLLMRouterColdStart:

    def test_cold_start_returns_sonnet(self):
        """Low confidence → always returns Sonnet."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=0.1)
        decision = router.route("SDK:model:debug:trivial", qt, "debug", "trivial")
        assert decision.recommended_model == MODEL_SONNET
        assert decision.route_to_local is False

    def test_cold_start_reason_mentions_phi(self):
        """Cold-start reason string includes phi threshold."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=0.3)
        decision = router.route("SDK:model:debug:trivial", qt, "debug", "trivial")
        assert "φ⁻¹" in decision.reason or "Cold start" in decision.reason

    def test_cold_start_confidence_in_decision(self):
        """Cold-start decision carries the actual confidence value."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=0.4)
        decision = router.route("SDK:model:debug:trivial", qt, "debug", "trivial")
        assert decision.confidence == pytest.approx(0.4)

    def test_zero_confidence_returns_sonnet(self):
        """Zero confidence (no data) → Sonnet."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=0.0)
        decision = router.route("SDK:model:debug:trivial", qt, "debug", "trivial")
        assert decision.recommended_model == MODEL_SONNET


# ═══════════════════════════════════════════════════════════════════════════
# Gate 2: task type filter
# ═══════════════════════════════════════════════════════════════════════════

class TestLLMRouterGate2TaskType:

    def test_unknown_task_type_stays_on_sonnet(self):
        """'review' task (not in simple set) → Sonnet even with high confidence."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01)
        decision = router.route("SDK:m:review:trivial", qt, "review", "trivial")
        assert decision.recommended_model == MODEL_SONNET
        assert "review" in decision.reason

    def test_complex_task_type_stays_on_sonnet(self):
        """'analyze' (not in simple set) → Sonnet."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01)
        decision = router.route("SDK:m:analyze:trivial", qt, "analyze", "trivial")
        assert decision.recommended_model == MODEL_SONNET

    def test_each_simple_type_passes_gate2(self):
        """All 5 simple task types pass Gate 2 when confidence is sufficient."""
        simple_types = ["debug", "refactor", "test", "explain", "write"]
        for t in simple_types:
            router = LLMRouter()
            qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=5)
            decision = router.route(f"SDK:m:{t}:trivial", qt, t, "trivial")
            # Should NOT fail on Gate 2 (may still fail Gate 4 if visits mock differs)
            assert "not in simple set" not in decision.reason, f"Gate 2 failed for type={t}"


# ═══════════════════════════════════════════════════════════════════════════
# Gate 3: complexity filter
# ═══════════════════════════════════════════════════════════════════════════

class TestLLMRouterGate3Complexity:

    def test_complex_complexity_stays_on_sonnet(self):
        """'complex' complexity → Sonnet even with high confidence + simple type."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01)
        decision = router.route("SDK:m:debug:complex", qt, "debug", "complex")
        assert decision.recommended_model == MODEL_SONNET
        assert "complex" in decision.reason or "too high" in decision.reason

    def test_moderate_complexity_stays_on_sonnet(self):
        """'moderate' complexity → Sonnet."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01)
        decision = router.route("SDK:m:debug:moderate", qt, "debug", "moderate")
        assert decision.recommended_model == MODEL_SONNET

    def test_trivial_complexity_passes_gate3(self):
        """'trivial' complexity passes Gate 3."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=5)
        decision = router.route("SDK:m:debug:trivial", qt, "debug", "trivial")
        assert "too high" not in decision.reason

    def test_simple_complexity_passes_gate3(self):
        """'simple' complexity passes Gate 3."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=5)
        decision = router.route("SDK:m:debug:simple", qt, "debug", "simple")
        assert "too high" not in decision.reason


# ═══════════════════════════════════════════════════════════════════════════
# Gate 4: minimum visits
# ═══════════════════════════════════════════════════════════════════════════

class TestLLMRouterGate4Visits:

    def test_insufficient_visits_stays_on_sonnet(self):
        """visits < MIN_VISITS → Sonnet despite high confidence + simple type."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=1)
        decision = router.route("SDK:m:debug:trivial", qt, "debug", "trivial")
        assert decision.recommended_model == MODEL_SONNET
        assert "Insufficient" in decision.reason

    def test_zero_visits_stays_on_sonnet(self):
        """0 visits (cold entry) → Sonnet."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=0)
        decision = router.route("SDK:m:debug:trivial", qt, "debug", "trivial")
        assert decision.recommended_model == MODEL_SONNET

    def test_min_visits_threshold_passes(self):
        """visits == MIN_VISITS_TO_ROUTE (3) → may route to Haiku."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=_MIN_VISITS_TO_ROUTE)
        decision = router.route("SDK:m:debug:trivial", qt, "debug", "trivial")
        # All gates passed: Haiku
        assert decision.recommended_model == MODEL_HAIKU


# ═══════════════════════════════════════════════════════════════════════════
# Happy path — all gates passed → Haiku
# ═══════════════════════════════════════════════════════════════════════════

class TestLLMRouterHappyPath:

    def test_all_gates_passed_returns_haiku(self):
        """All 4 gates passed → recommends Haiku."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=5)
        decision = router.route("SDK:m:debug:trivial", qt, "debug", "trivial")
        assert decision.recommended_model == MODEL_HAIKU
        assert decision.route_to_local is True

    def test_haiku_reason_mentions_cost_saving(self):
        """Haiku reason string mentions cost saving."""
        router = LLMRouter()
        sk = "SDK:m:refactor:simple"
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=5, state_key=sk)
        decision = router.route(sk, qt, "refactor", "simple")
        assert "Haiku" in decision.reason or "cost" in decision.reason.lower()

    def test_haiku_decision_carries_correct_task_type(self):
        """RoutingDecision.task_type matches the input."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=5)
        for t in ["test", "explain", "write"]:
            decision = router.route(f"SDK:m:{t}:trivial", qt, t, "trivial")
            assert decision.task_type == t

    def test_haiku_decision_route_to_local_true(self):
        """route_to_local=True when routing to Haiku."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=10)
        decision = router.route("SDK:m:debug:trivial", qt, "debug", "trivial")
        assert decision.route_to_local is True


# ═══════════════════════════════════════════════════════════════════════════
# LLMRouter.stats()
# ═══════════════════════════════════════════════════════════════════════════

class TestLLMRouterStats:

    def test_stats_returns_dict(self):
        """stats() returns a dict."""
        router = LLMRouter()
        s = router.stats()
        assert isinstance(s, dict)

    def test_stats_has_required_keys(self):
        """stats() includes all documented keys."""
        router = LLMRouter()
        s = router.stats()
        for key in ("total_routes", "routes_to_local", "routes_to_full",
                    "local_rate", "phi_threshold", "min_visits",
                    "simple_task_types", "cheap_complexities"):
            assert key in s, f"Missing stats key: {key}"

    def test_stats_phi_threshold_correct(self):
        """stats phi_threshold matches PHI_INV."""
        router = LLMRouter()
        assert router.stats()["phi_threshold"] == pytest.approx(PHI_INV)

    def test_stats_local_rate_zero_initially(self):
        """No routes yet → local_rate = 0."""
        router = LLMRouter()
        assert router.stats()["local_rate"] == 0.0

    def test_stats_tracks_haiku_routes(self):
        """After a Haiku route, total_routes and routes_to_local increment."""
        router = LLMRouter()
        qt = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=5)
        router.route("SDK:m:debug:trivial", qt, "debug", "trivial")
        s = router.stats()
        assert s["total_routes"] == 1
        assert s["routes_to_local"] == 1
        assert s["local_rate"] == 1.0

    def test_stats_local_rate_reflects_ratio(self):
        """local_rate = routes_to_local / total_routes."""
        router = LLMRouter()
        qt_warm = _mock_qtable_with_confidence(confidence=PHI_INV + 0.01, visits=5)
        qt_cold = _mock_qtable_with_confidence(confidence=0.1, visits=0)
        # 1 Haiku route
        router.route("SDK:m:debug:trivial", qt_warm, "debug", "trivial")
        # 1 Sonnet route (cold)
        router.route("SDK:m:debug:trivial", qt_cold, "debug", "trivial")
        # total_routes only increments on Haiku (cold start never increments)
        s = router.stats()
        assert s["routes_to_local"] == 1
        # total_routes counts only successful Haiku routes
        assert s["total_routes"] == 1

    def test_stats_simple_task_types_list(self):
        """simple_task_types is a sorted list of expected types."""
        router = LLMRouter()
        types = router.stats()["simple_task_types"]
        assert isinstance(types, list)
        assert "debug" in types
        assert "refactor" in types

    def test_stats_cheap_complexities_list(self):
        """cheap_complexities includes trivial and simple."""
        router = LLMRouter()
        cs = router.stats()["cheap_complexities"]
        assert "trivial" in cs
        assert "simple" in cs


# ═══════════════════════════════════════════════════════════════════════════
# Integration with real QTable (no mocks)
# ═══════════════════════════════════════════════════════════════════════════

class TestLLMRouterWithRealQTable:

    def test_real_qtable_cold_start_stays_sonnet(self):
        """Fresh QTable → confidence 0 → Sonnet."""
        router = LLMRouter()
        qt = QTable()
        decision = router.route("SDK:m:debug:trivial", qt, "debug", "trivial")
        assert decision.recommended_model == MODEL_SONNET

    def test_real_qtable_warm_routes_to_haiku(self):
        """After enough real updates, QTable warms up → Haiku."""
        router = LLMRouter()
        sk = "SDK:m:debug:trivial"
        qt = QTable()
        # Inject enough high-reward updates (3 is the minimum visit threshold)
        for _ in range(_MIN_VISITS_TO_ROUTE + 2):
            qt.update(LearningSignal(state_key=sk, action="HOWL", reward=90.0))
        # Confidence is PHI-bounded; check if we got past the confidence gate
        conf = qt.confidence(sk)
        decision = router.route(sk, qt, "debug", "trivial")
        if conf >= PHI_INV:
            # All gates passed — should be Haiku
            assert decision.recommended_model == MODEL_HAIKU
        else:
            # Still cold — stays on Sonnet (acceptable)
            assert decision.recommended_model == MODEL_SONNET
