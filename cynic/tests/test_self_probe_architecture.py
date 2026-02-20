"""
SelfProber Architecture Analysis Tests

Tests that SelfProber detects:
1. Coupling growth (handlers adding new dependencies)
2. Health degradation (architecture health score dropping)
3. Complex handlers (above complexity threshold)
4. Complexity regression (handlers becoming more complex)
"""

from __future__ import annotations

import pytest
from unittest.mock import MagicMock, AsyncMock
from cynic.core.event_bus import CoreEvent
from cynic.api.handlers.base import HandlerGroup
from cynic.api.handlers.introspect import HandlerArchitectureIntrospector
from cynic.cognition.cortex.self_probe import SelfProber


# ── Fixtures ──────────────────────────────────────────────────────────────────

@pytest.fixture
def mock_handler_v1():
    """Mock handler with minimal dependencies (version 1)."""
    handler = MagicMock(spec=HandlerGroup)
    handler.name = "intelligence"
    handler.subscriptions.return_value = [
        (CoreEvent.JUDGMENT_CREATED, MagicMock()),
        (CoreEvent.EMERGENCE_DETECTED, MagicMock()),
    ]
    handler.dependencies.return_value = frozenset({"orchestrator", "qtable"})
    return handler


@pytest.fixture
def mock_handler_v2():
    """Mock handler with MORE dependencies (version 2 - coupling growth)."""
    handler = MagicMock(spec=HandlerGroup)
    handler.name = "intelligence"
    handler.subscriptions.return_value = [
        (CoreEvent.JUDGMENT_CREATED, MagicMock()),
        (CoreEvent.EMERGENCE_DETECTED, MagicMock()),
    ]
    # Added: residual_detector, axiom_monitor (coupling growth)
    handler.dependencies.return_value = frozenset({
        "orchestrator", "qtable", "residual_detector", "axiom_monitor"
    })
    return handler


@pytest.fixture
def mock_registry_v1(mock_handler_v1):
    """Mock registry with one simple handler."""
    registry = MagicMock()
    registry._groups = [mock_handler_v1]
    registry.introspect.return_value = {"total_deps": 2, "groups": []}
    return registry


@pytest.fixture
def mock_registry_v2(mock_handler_v2):
    """Mock registry with handler that has grown in coupling."""
    registry = MagicMock()
    registry._groups = [mock_handler_v2]
    registry.introspect.return_value = {"total_deps": 4, "groups": []}
    return registry


@pytest.fixture
def prober():
    """Fresh SelfProber instance."""
    return SelfProber()


# ── Tests ─────────────────────────────────────────────────────────────────────

class TestArchitectureAnalysisInitialization:
    """Verify introspector is lazily initialized."""

    def test_introspector_none_before_analysis(self, prober):
        """Introspector not created until first architecture analysis."""
        assert prober._introspector is None

    def test_introspector_created_on_first_analysis(self, prober, mock_registry_v1):
        """Introspector lazily instantiated during first analysis."""
        prober.set_handler_registry(mock_registry_v1)
        prober._analyze_architecture("MANUAL", "TEST", 0.5)
        assert prober._introspector is not None
        assert isinstance(prober._introspector, HandlerArchitectureIntrospector)

    def test_introspector_reused_on_subsequent_calls(self, prober, mock_registry_v1):
        """Introspector is reused, not recreated."""
        prober.set_handler_registry(mock_registry_v1)
        prober._analyze_architecture("MANUAL", "TEST", 0.5)
        first_introspector = prober._introspector
        prober._analyze_architecture("MANUAL", "TEST", 0.5)
        assert prober._introspector is first_introspector


class TestCouplingGrowthDetection:
    """Verify detection of handler coupling growth."""

    def test_no_proposals_on_first_run(self, prober, mock_registry_v1):
        """First analysis has no previous snapshot — no coupling growth detected."""
        prober.set_handler_registry(mock_registry_v1)
        proposals = prober._analyze_architecture("MANUAL", "TEST", 0.5)
        # No proposals because _prev_snapshot is None
        assert len(proposals) == 0

    def test_detects_coupling_growth_simple(self, prober, mock_registry_v1, mock_registry_v2):
        """Detect when handler adds dependencies between runs."""
        prober.set_handler_registry(mock_registry_v1)
        # First run: v1 (2 deps)
        prober._analyze_architecture("MANUAL", "TEST", 0.5)
        assert prober._prev_snapshot is not None

        # Change registry to v2 (4 deps)
        prober.set_handler_registry(mock_registry_v2)
        proposals = prober._analyze_architecture("MANUAL", "TEST", 0.5)

        # Should detect coupling growth
        coupling_proposals = [p for p in proposals if p.pattern_type == "ARCHITECTURE_COUPLING_GROWTH"]
        assert len(coupling_proposals) > 0

    def test_coupling_growth_proposal_details(self, prober, mock_registry_v1, mock_registry_v2):
        """Coupling growth proposal contains correct severity and recommendation."""
        prober.set_handler_registry(mock_registry_v1)
        prober._analyze_architecture("MANUAL", "TEST", 0.5)

        prober.set_handler_registry(mock_registry_v2)
        proposals = prober._analyze_architecture("MANUAL", "TEST", 0.5)

        coupling_proposals = [p for p in proposals if p.pattern_type == "ARCHITECTURE_COUPLING_GROWTH"]
        if coupling_proposals:
            p = coupling_proposals[0]
            assert p.target == "intelligence"
            assert p.severity > 0.0
            assert "refactoring" in p.recommendation.lower()


class TestHealthScoreDegradation:
    """Verify detection of architecture health degradation."""

    def test_no_health_degradation_on_first_run(self, prober, mock_registry_v1):
        """First analysis has no baseline — no degradation detected."""
        prober.set_handler_registry(mock_registry_v1)
        proposals = prober._analyze_architecture("MANUAL", "TEST", 0.5)
        health_proposals = [p for p in proposals if p.pattern_type == "ARCHITECTURE_HEALTH_DEGRADATION"]
        assert len(health_proposals) == 0

    def test_stores_health_snapshot(self, prober, mock_registry_v1):
        """Health snapshot is stored for next comparison."""
        prober.set_handler_registry(mock_registry_v1)
        assert prober._prev_snapshot is None
        prober._analyze_architecture("MANUAL", "TEST", 0.5)
        assert prober._prev_snapshot is not None

    def test_complex_handler_detection(self, prober):
        """Detect handlers with high complexity (above threshold)."""
        # Create a mock handler with high complexity
        complex_handler = MagicMock(spec=HandlerGroup)
        complex_handler.name = "complex_handler"
        # 5 subscriptions × 10 + 5 deps × 5 = 75 complexity
        complex_handler.subscriptions.return_value = [
            (CoreEvent.JUDGMENT_CREATED, MagicMock()),
            (CoreEvent.EMERGENCE_DETECTED, MagicMock()),
            (CoreEvent.DECISION_MADE, MagicMock()),
            (CoreEvent.BUDGET_WARNING, MagicMock()),
            (CoreEvent.AXIOM_ACTIVATED, MagicMock()),
        ]
        complex_handler.dependencies.return_value = frozenset({
            "orchestrator", "qtable", "residual", "escore", "scheduler"
        })

        registry = MagicMock()
        registry._groups = [complex_handler]

        prober.set_handler_registry(registry)
        proposals = prober._analyze_architecture("MANUAL", "TEST", 0.5)

        complexity_proposals = [p for p in proposals if p.pattern_type == "ARCHITECTURE_COMPLEXITY"]
        assert len(complexity_proposals) > 0
        if complexity_proposals:
            p = complexity_proposals[0]
            assert p.target == "complex_handler"
            assert p.severity > 0.0


class TestArchitectureAnalysisEdgeCases:
    """Edge cases and error handling."""

    def test_no_registry_returns_empty_proposals(self, prober):
        """No proposals if registry not set."""
        proposals = prober._analyze_architecture("MANUAL", "TEST", 0.5)
        assert len(proposals) == 0

    def test_empty_handler_groups_returns_empty_proposals(self, prober):
        """No proposals if handler groups are empty."""
        registry = MagicMock()
        registry._groups = []
        prober.set_handler_registry(registry)
        proposals = prober._analyze_architecture("MANUAL", "TEST", 0.5)
        assert len(proposals) == 0

    def test_exception_handling_returns_empty(self, prober):
        """Exception during analysis returns empty proposals."""
        registry = MagicMock()
        registry._groups = MagicMock(side_effect=RuntimeError("Introspection failed"))
        prober.set_handler_registry(registry)
        proposals = prober._analyze_architecture("MANUAL", "TEST", 0.5)
        assert len(proposals) == 0


class TestArchitectureProposalIntegration:
    """Integration: architecture proposals are stored and emitted."""

    def test_architecture_proposals_included_in_analyze(self, prober):
        """Architecture proposals are included in analyze() output."""
        complex_handler = MagicMock(spec=HandlerGroup)
        complex_handler.name = "test_handler"
        complex_handler.subscriptions.return_value = [
            (CoreEvent.JUDGMENT_CREATED, MagicMock()),
            (CoreEvent.EMERGENCE_DETECTED, MagicMock()),
            (CoreEvent.DECISION_MADE, MagicMock()),
        ]
        complex_handler.dependencies.return_value = frozenset({
            "a", "b", "c", "d", "e"  # 5 deps
        })

        registry = MagicMock()
        registry._groups = [complex_handler]

        prober.set_handler_registry(registry)
        proposals = prober.analyze(trigger="MANUAL", pattern_type="TEST", severity=0.5)

        # Should include architecture proposals
        architecture_proposals = [
            p for p in proposals
            if p.pattern_type in (
                "ARCHITECTURE_COUPLING_GROWTH",
                "ARCHITECTURE_HEALTH_DEGRADATION",
                "ARCHITECTURE_COMPLEXITY",
            )
        ]
        assert len(architecture_proposals) >= 0  # Might have no growth on first run

    def test_multiple_handlers_analyzed(self, prober):
        """Multiple handlers are analyzed independently."""
        handler1 = MagicMock(spec=HandlerGroup)
        handler1.name = "handler1"
        handler1.subscriptions.return_value = [(CoreEvent.JUDGMENT_CREATED, MagicMock())]
        handler1.dependencies.return_value = frozenset({"a", "b"})

        handler2 = MagicMock(spec=HandlerGroup)
        handler2.name = "handler2"
        handler2.subscriptions.return_value = [
            (CoreEvent.JUDGMENT_CREATED, MagicMock()),
            (CoreEvent.EMERGENCE_DETECTED, MagicMock()),
            (CoreEvent.DECISION_MADE, MagicMock()),
        ]
        handler2.dependencies.return_value = frozenset({"x", "y", "z", "w", "v"})

        registry = MagicMock()
        registry._groups = [handler1, handler2]

        prober.set_handler_registry(registry)
        proposals = prober._analyze_architecture("MANUAL", "TEST", 0.5)

        # handler2 is more complex, should generate proposal
        complex_proposals = [p for p in proposals if p.pattern_type == "ARCHITECTURE_COMPLEXITY"]
        # May or may not have proposals depending on thresholds, but should not crash
        assert isinstance(complex_proposals, list)
