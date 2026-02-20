"""
Handler Introspection Tests — Architectural self-awareness.

Tests that:
1. Snapshots capture handler topology correctly
2. Coupling growth detection works
3. Complexity scoring is consistent
4. Health score reflects architecture quality
"""

from __future__ import annotations

import pytest
from unittest.mock import MagicMock

from cynic.core.event_bus import CoreEvent
from cynic.api.handlers.base import HandlerGroup
from cynic.api.handlers.introspect import (
    HandlerAnalysis,
    ArchitectureSnapshot,
    CouplingGrowth,
    HandlerArchitectureIntrospector,
)


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture
def mock_handler_simple():
    """Mock handler with minimal dependencies."""
    handler = MagicMock(spec=HandlerGroup)
    handler.name = "simple"
    handler.subscriptions.return_value = [
        (CoreEvent.JUDGMENT_CREATED, MagicMock()),
    ]
    handler.dependencies.return_value = frozenset({"escore_tracker"})
    return handler


@pytest.fixture
def mock_handler_complex():
    """Mock handler with many dependencies."""
    handler = MagicMock(spec=HandlerGroup)
    handler.name = "complex"
    handler.subscriptions.return_value = [
        (CoreEvent.JUDGMENT_CREATED, MagicMock()),
        (CoreEvent.EMERGENCE_DETECTED, MagicMock()),
        (CoreEvent.BUDGET_WARNING, MagicMock()),
    ]
    handler.dependencies.return_value = frozenset({
        "orchestrator",
        "scheduler",
        "db_pool",
        "compressor",
        "action_proposer",
    })
    return handler


@pytest.fixture
def introspector():
    """Fresh introspector instance."""
    return HandlerArchitectureIntrospector()


# ── Analysis Tests ────────────────────────────────────────────────────────────

class TestHandlerAnalysis:

    def test_from_handler_simple(self, mock_handler_simple):
        """Analysis captures handler metadata correctly."""
        analysis = HandlerAnalysis.from_handler(mock_handler_simple)

        assert analysis.name == "simple"
        assert analysis.handler_count == 1
        assert analysis.dependencies == frozenset({"escore_tracker"})
        assert len(analysis.events) == 1

    def test_from_handler_complex(self, mock_handler_complex):
        """Complex handler gets higher complexity score."""
        analysis = HandlerAnalysis.from_handler(mock_handler_complex)

        assert analysis.name == "complex"
        assert analysis.handler_count == 3
        assert len(analysis.dependencies) == 5
        # Complexity = (3 * 10) + (5 * 5) = 30 + 25 = 55
        assert analysis.complexity_score == 55.0

    def test_complexity_score_formula(self, mock_handler_simple, mock_handler_complex):
        """Complexity formula: (handler_count * 10) + (dependency_count * 5)."""
        simple_analysis = HandlerAnalysis.from_handler(mock_handler_simple)
        complex_analysis = HandlerAnalysis.from_handler(mock_handler_complex)

        # Simple: (1 * 10) + (1 * 5) = 15
        assert simple_analysis.complexity_score == 15.0

        # Complex: (3 * 10) + (5 * 5) = 55
        assert complex_analysis.complexity_score == 55.0

    def test_is_complex_threshold(self, mock_handler_simple, mock_handler_complex):
        """is_complex() respects threshold."""
        simple_analysis = HandlerAnalysis.from_handler(mock_handler_simple)
        complex_analysis = HandlerAnalysis.from_handler(mock_handler_complex)

        assert simple_analysis.is_complex(threshold=50.0) is False
        assert complex_analysis.is_complex(threshold=50.0) is True

    def test_analysis_to_dict(self, mock_handler_simple):
        """Serialization preserves all data."""
        analysis = HandlerAnalysis.from_handler(mock_handler_simple)
        d = analysis.to_dict()

        assert d["name"] == "simple"
        assert d["handler_count"] == 1
        assert d["dependency_count"] == 1
        assert "dependencies" in d
        assert "event_count" in d


# ── Snapshot Tests ────────────────────────────────────────────────────────────

class TestArchitectureSnapshot:

    def test_snapshot_single_handler(self, introspector, mock_handler_simple):
        """Snapshot with one handler."""
        snapshot = introspector.snapshot([mock_handler_simple])

        assert len(snapshot.handler_analyses) == 1
        assert snapshot.total_handlers == 1
        assert snapshot.total_dependencies == frozenset({"escore_tracker"})

    def test_snapshot_multiple_handlers(self, introspector, mock_handler_simple, mock_handler_complex):
        """Snapshot aggregates multiple handlers."""
        snapshot = introspector.snapshot([mock_handler_simple, mock_handler_complex])

        assert len(snapshot.handler_analyses) == 2
        assert snapshot.total_handlers == 4  # 1 + 3
        # All deps: escore_tracker + 5 from complex
        assert len(snapshot.total_dependencies) == 6

    def test_snapshot_complexity_stats(self, introspector, mock_handler_simple, mock_handler_complex):
        """Complexity statistics calculated correctly."""
        snapshot = introspector.snapshot([mock_handler_simple, mock_handler_complex])

        # Scores: 15.0, 55.0 → avg = 35.0
        assert snapshot.average_complexity == 35.0

        # Most complex
        assert snapshot.most_complex_handler.name == "complex"
        assert snapshot.most_complex_handler.complexity_score == 55.0

    def test_snapshot_to_dict(self, introspector, mock_handler_simple):
        """Serialization works."""
        snapshot = introspector.snapshot([mock_handler_simple])
        d = snapshot.to_dict()

        assert "timestamp" in d
        assert "handlers" in d
        assert d["total_handlers"] == 1


# ── Coupling Growth Tests ──────────────────────────────────────────────────────

class TestCouplingGrowth:

    def test_growth_detection_added_dependency(self, introspector):
        """Detects when handler adds dependency."""
        handler1 = MagicMock(spec=HandlerGroup)
        handler1.name = "test"
        handler1.subscriptions.return_value = [(CoreEvent.JUDGMENT_CREATED, MagicMock())]
        handler1.dependencies.return_value = frozenset({"escore_tracker"})

        handler2 = MagicMock(spec=HandlerGroup)
        handler2.name = "test"
        handler2.subscriptions.return_value = [(CoreEvent.JUDGMENT_CREATED, MagicMock())]
        handler2.dependencies.return_value = frozenset({"escore_tracker", "orchestrator"})

        snap1 = introspector.snapshot([handler1])
        snap2 = introspector.snapshot([handler2])

        changes = introspector.detect_coupling_growth(snap1, snap2)

        assert len(changes) == 1
        assert changes[0].handler_name == "test"
        assert changes[0].added_dependencies == frozenset({"orchestrator"})
        assert changes[0].removed_dependencies == frozenset()
        assert changes[0].is_growth is True

    def test_growth_severity_scoring(self):
        """Severity formula: (added * 15) - (removed * 5)."""
        growth = CouplingGrowth(
            handler_name="test",
            prev_dependency_count=1,
            new_dependency_count=3,
            added_dependencies=frozenset({"dep1", "dep2"}),
            removed_dependencies=frozenset(),
            complexity_delta=10.0,
        )

        # (2 * 15) - (0 * 5) = 30
        assert growth.severity_score == 30.0

    def test_growth_shrinkage_detection(self):
        """Detects when handler reduces coupling."""
        growth = CouplingGrowth(
            handler_name="test",
            prev_dependency_count=3,
            new_dependency_count=1,
            added_dependencies=frozenset(),
            removed_dependencies=frozenset({"dep1", "dep2"}),
            complexity_delta=-10.0,
        )

        assert growth.is_shrinkage is True
        assert growth.is_growth is False


# ── Complex Handler Detection Tests ────────────────────────────────────────────

class TestComplexHandlerDetection:

    def test_find_complex_handlers(self, introspector, mock_handler_simple, mock_handler_complex):
        """Identifies handlers above complexity threshold."""
        snapshot = introspector.snapshot([mock_handler_simple, mock_handler_complex])

        complex_list = introspector.find_complex_handlers(snapshot, threshold=30.0)

        assert len(complex_list) == 1
        assert complex_list[0].name == "complex"

    def test_no_complex_handlers(self, introspector, mock_handler_simple):
        """Returns empty list when no handlers exceed threshold."""
        snapshot = introspector.snapshot([mock_handler_simple])

        complex_list = introspector.find_complex_handlers(snapshot, threshold=100.0)

        assert len(complex_list) == 0


# ── Health Score Tests ─────────────────────────────────────────────────────────

class TestHealthScore:

    def test_health_score_optimal(self, introspector, mock_handler_simple):
        """Optimal architecture (simple handler) has high health score."""
        snapshot = introspector.snapshot([mock_handler_simple])
        health = introspector.health_score(snapshot)

        # Simple handler: low complexity, no variance, no outliers
        # health ≈ 100 - (15 * 0.4) - (0 * 0.3) - 0 ≈ 94
        assert health > 85.0

    def test_health_score_complex(self, introspector, mock_handler_complex):
        """Complex architecture has lower health score."""
        snapshot = introspector.snapshot([mock_handler_complex])
        health = introspector.health_score(snapshot)

        # Complex handler: higher complexity
        # health ≈ 100 - (55 * 0.4) - 0 - 0 ≈ 78
        assert health < 85.0

    def test_health_score_bounded(self, introspector):
        """Health score stays in [0, 100]."""
        # Create a pathological handler
        handler = MagicMock(spec=HandlerGroup)
        handler.name = "pathological"
        # 100 handlers (complexity ≈ 100 due to cap)
        handler.subscriptions.return_value = [(CoreEvent.JUDGMENT_CREATED, MagicMock())] * 100
        handler.dependencies.return_value = frozenset({"x" * i for i in range(100)})

        snapshot = introspector.snapshot([handler])
        health = introspector.health_score(snapshot)

        assert 0.0 <= health <= 100.0
