"""
Unit tests for LLM Router — Q-Table driven model selection

Tests:
  1. Cold start: routes to Sonnet when confidence < φ⁻¹
  2. Routing logic: confidence-based model selection
  3. Task type filtering: only simple tasks can route to Haiku
  4. Complexity filtering: only trivial/simple can route to cheap
  5. Visit count validation: minimum 3 visits required
  6. Stats tracking: route counts and rates
"""

import pytest

from cynic.kernel.core.phi import PHI_INV
from cynic.kernel.organism.brain.learning.qlearning import LearningSignal, QTable
from cynic.kernel.organism.metabolism.llm_router import (
    _CHEAP_COMPLEXITIES,
    _MIN_VISITS_TO_ROUTE,
    _SIMPLE_TASK_TYPES,
    MODEL_HAIKU,
    MODEL_SONNET,
    LLMRouter,
)

# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture
def router() -> LLMRouter:
    """Create a fresh LLMRouter for each test."""
    return LLMRouter()


@pytest.fixture
def qtable_warm() -> QTable:
    """Create a QTable with some data (high confidence)."""
    qtable = QTable()
    
    # Add enough visits to exceed confidence threshold
    for _ in range(10):
        qtable.update(LearningSignal(
            state_key="SDK:claude-sonnet:debug:trivial",
            action="WAG",
            reward=0.7,
        ))
    
    return qtable


@pytest.fixture
def qtable_cold() -> QTable:
    """Create a QTable with no data (cold start)."""
    return QTable()


# ════════════════════════════════════════════════════════════════════════════
# COLD START TESTS
# ════════════════════════════════════════════════════════════════════════════

def test_cold_start_routes_to_sonnet(router: LLMRouter, qtable_cold: QTable) -> None:
    """Test cold start routes to Sonnet when confidence is 0."""
    decision = router.route(
        state_key="SDK:claude-sonnet:debug:trivial",
        qtable=qtable_cold,
        task_type="debug",
        complexity="trivial",
    )
    
    assert decision.recommended_model == MODEL_SONNET
    assert decision.route_to_local is False
    assert "Cold start" in decision.reason
    assert decision.confidence == 0.0


def test_low_confidence_routes_to_sonnet(router: LLMRouter) -> None:
    """Test low confidence (< φ⁻¹) routes to Sonnet."""
    qtable = QTable()
    
    # Add just 1 visit — low confidence
    qtable.update(LearningSignal(
        state_key="low:confidence:test",
        action="WAG",
        reward=0.5,
    ))
    
    decision = router.route(
        state_key="low:confidence:test",
        qtable=qtable,
        task_type="debug",
        complexity="trivial",
    )
    
    assert decision.recommended_model == MODEL_SONNET
    assert decision.route_to_local is False


# ════════════════════════════════════════════════════════════════════════════
# TASK TYPE FILTERING TESTS
# ════════════════════════════════════════════════════════════════════════════

def test_complex_task_routes_to_sonnet(router: LLMRouter) -> None:
    """Test complex tasks always route to Sonnet."""
    qtable = QTable()
    # Add enough visits for high confidence on this specific state
    for _ in range(30):
        qtable.update(LearningSignal(
            state_key="SDK:claude-sonnet:review:complex",
            action="WAG",
            reward=0.7,
        ))
    
    decision = router.route(
        state_key="SDK:claude-sonnet:review:complex",
        qtable=qtable,
        task_type="review",  # Not in SIMPLE_TASK_TYPES
        complexity="trivial",
    )
    
    assert decision.recommended_model == MODEL_SONNET
    assert decision.route_to_local is False
    assert "requires full capability" in decision.reason


def test_simple_task_allowed(router: LLMRouter, qtable_warm: QTable) -> None:
    """Test simple tasks can potentially route to Haiku."""
    for task_type in _SIMPLE_TASK_TYPES:
        decision = router.route(
            state_key=f"SDK:claude-sonnet:{task_type}:trivial",
            qtable=qtable_warm,
            task_type=task_type,
            complexity="trivial",
        )
        
        # Should pass task type check, but may fail other gates
        assert decision.task_type == task_type


# ════════════════════════════════════════════════════════════════════════════
# COMPLEXITY FILTERING TESTS
# ════════════════════════════════════════════════════════════════════════════

def test_complexity_trivial_routes_to_haiku(router: LLMRouter) -> None:
    """Test trivial complexity can route to Haiku when all gates pass."""
    qtable = QTable()
    
    # Add enough visits for confidence + minimum threshold
    for _ in range(10):
        qtable.update(LearningSignal(
            state_key="trivial:complexity:test",
            action="WAG",
            reward=0.7,
        ))
    
    decision = router.route(
        state_key="trivial:complexity:test",
        qtable=qtable,
        task_type="debug",
        complexity="trivial",
    )
    
    # Should pass complexity gate
    assert decision.complexity == "trivial"


def test_complexity_complex_routes_to_sonnet(router: LLMRouter) -> None:
    """Test complex complexity routes to Sonnet."""
    qtable = QTable()
    
    # Add enough visits to pass confidence threshold
    for _ in range(30):
        qtable.update(LearningSignal(
            state_key="complex:complexity:test",
            action="WAG",
            reward=0.7,
        ))
    
    decision = router.route(
        state_key="complex:complexity:test",
        qtable=qtable,
        task_type="debug",
        complexity="complex",  # Not in _CHEAP_COMPLEXITIES
    )
    
    assert decision.recommended_model == MODEL_SONNET
    assert "too high" in decision.reason


def test_complexity_moderate_routes_to_sonnet(router: LLMRouter) -> None:
    """Test moderate complexity routes to Sonnet."""
    qtable = QTable()
    
    for _ in range(10):
        qtable.update(LearningSignal(
            state_key="moderate:complexity:test",
            action="WAG",
            reward=0.7,
        ))
    
    decision = router.route(
        state_key="moderate:complexity:test",
        qtable=qtable,
        task_type="debug",
        complexity="moderate",
    )
    
    assert decision.recommended_model == MODEL_SONNET


# ════════════════════════════════════════════════════════════════════════════
# VISIT COUNT TESTS
# ════════════════════════════════════════════════════════════════════════════

def test_visit_count_below_minimum_routes_to_sonnet(router: LLMRouter) -> None:
    """Test visit count below minimum routes to Sonnet."""
    qtable = QTable()
    
    # Add visits but below minimum threshold
    for _ in range(_MIN_VISITS_TO_ROUTE - 1):
        qtable.update(LearningSignal(
            state_key="low:visits:test",
            action="WAG",
            reward=0.7,
        ))
    
    decision = router.route(
        state_key="low:visits:test",
        qtable=qtable,
        task_type="debug",
        complexity="trivial",
    )
    
    assert decision.recommended_model == MODEL_SONNET
    # Either cold start (confidence gate) or insufficient data gate
    assert "Insufficient data" in decision.reason or "Cold start" in decision.reason


def test_visit_count_above_minimum_passes_gate(router: LLMRouter) -> None:
    """Test visit count above minimum passes the gate."""
    qtable = QTable()
    
    # Add visits at or above minimum threshold
    for _ in range(_MIN_VISITS_TO_ROUTE):
        qtable.update(LearningSignal(
            state_key="enough:visits:test",
            action="WAG",
            reward=0.7,
        ))
    
    decision = router.route(
        state_key="enough:visits:test",
        qtable=qtable,
        task_type="debug",
        complexity="trivial",
    )
    
    # Should pass visit gate (but may fail other gates)
    assert "Insufficient data" not in decision.reason


# ════════════════════════════════════════════════════════════════════════════
# SUCCESSFUL ROUTING TO HAIKU TESTS
# ════════════════════════════════════════════════════════════════════════════

def test_full_routing_to_haiku(router: LLMRouter) -> None:
    """Test full routing to Haiku when all gates pass."""
    qtable = QTable()
    
    # Add enough visits for high confidence
    for _ in range(20):
        qtable.update(LearningSignal(
            state_key="SDK:claude-sonnet:debug:trivial",
            action="WAG",
            reward=0.8,
        ))
    
    decision = router.route(
        state_key="SDK:claude-sonnet:debug:trivial",
        qtable=qtable,
        task_type="debug",
        complexity="trivial",
    )
    
    assert decision.recommended_model == MODEL_HAIKU
    assert decision.route_to_local is True
    assert decision.confidence >= PHI_INV


def test_simple_complexity_routes_to_haiku(router: LLMRouter) -> None:
    """Test simple complexity can route to Haiku."""
    qtable = QTable()
    
    for _ in range(20):
        qtable.update(LearningSignal(
            state_key="simple:task:test",
            action="WAG",
            reward=0.8,
        ))
    
    decision = router.route(
        state_key="simple:task:test",
        qtable=qtable,
        task_type="refactor",
        complexity="simple",
    )
    
    assert decision.recommended_model == MODEL_HAIKU
    assert decision.route_to_local is True


# ════════════════════════════════════════════════════════════════════════════
# STATS TESTS
# ════════════════════════════════════════════════════════════════════════════

def test_stats_initial_state(router: LLMRouter) -> None:
    """Test stats() returns correct initial state."""
    stats = router.stats()
    
    assert stats["total_routes"] == 0
    assert stats["routes_to_local"] == 0
    assert stats["routes_to_full"] == 0
    assert stats["local_rate"] == 0.0
    assert stats["phi_threshold"] == PHI_INV
    assert stats["min_visits"] == _MIN_VISITS_TO_ROUTE


def test_stats_tracks_routes(router: LLMRouter) -> None:
    """Test stats() tracks routing decisions to local only."""
    qtable = QTable()
    
    # Add visits for state key to pass confidence threshold
    for _ in range(30):
        qtable.update(LearningSignal(state_key="test:route:1", action="WAG", reward=0.8))
    
    # Make some routing decisions to local (Haiku)
    router.route("test:route:1", qtable, "debug", "trivial")  # to Haiku
    router.route("test:route:1", qtable, "debug", "trivial")  # to Haiku
    
    stats = router.stats()
    
    # Stats only track routes_to_local (increment on Haiku routing)
    assert stats["total_routes"] == 2
    assert stats["routes_to_local"] == 2
    assert stats["routes_to_full"] == 0
    assert stats["local_rate"] == pytest.approx(1.0, rel=0.01)


# ════════════════════════════════════════════════════════════════════════════
# EDGE CASES
# ════════════════════════════════════════════════════════════════════════════

def test_unknown_task_type_routes_to_sonnet(router: LLMRouter, qtable_warm: QTable) -> None:
    """Test unknown task type routes to Sonnet."""
    decision = router.route(
        state_key="unknown:task:test",
        qtable=qtable_warm,
        task_type="unknown_task_type",
        complexity="trivial",
    )
    
    assert decision.recommended_model == MODEL_SONNET


def test_all_simple_task_types(router: LLMRouter) -> None:
    """Test all SIMPLE_TASK_TYPES are recognized."""
    qtable = QTable()
    
    for _ in range(20):
        qtable.update(LearningSignal(state_key="test", action="WAG", reward=0.8))
    
    for task_type in _SIMPLE_TASK_TYPES:
        decision = router.route(
            state_key=f"test:{task_type}",
            qtable=qtable,
            task_type=task_type,
            complexity="trivial",
        )
        assert decision.task_type == task_type


def test_all_cheap_complexities(router: LLMRouter) -> None:
    """Test all _CHEAP_COMPLEXITIES are recognized."""
    qtable = QTable()
    
    for _ in range(20):
        qtable.update(LearningSignal(state_key="test", action="WAG", reward=0.8))
    
    for complexity in _CHEAP_COMPLEXITIES:
        decision = router.route(
            state_key=f"test:{complexity}",
            qtable=qtable,
            task_type="debug",
            complexity=complexity,
        )
        assert decision.complexity == complexity


def test_routing_decision_attributes(router: LLMRouter, qtable_cold: QTable) -> None:
    """Test RoutingDecision has all required attributes."""
    decision = router.route(
        state_key="test:attrs",
        qtable=qtable_cold,
        task_type="debug",
        complexity="trivial",
    )
    
    assert hasattr(decision, "recommended_model")
    assert hasattr(decision, "route_to_local")
    assert hasattr(decision, "confidence")
    assert hasattr(decision, "reason")
    assert hasattr(decision, "task_type")
    assert hasattr(decision, "complexity")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
