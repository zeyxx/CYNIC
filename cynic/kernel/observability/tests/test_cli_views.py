"""Tests for CLI Views module.

Tests rendering functions for OBSERVE, CYNIC, and MACHINE views.
Verifies formatting, progress bars, and content inclusion.
"""

import time

import pytest

from cynic.kernel.observability.cli.views import (
    _format_percentage,
    _render_progress_bar,
    render_cynic_view,
    render_machine_view,
    render_observe_view,
)
from cynic.kernel.observability.models import SymbioticState


@pytest.fixture
def sample_state() -> SymbioticState:
    """Create a sample SymbioticState for testing."""
    return SymbioticState(
        # CYNIC state
        cynic_observations={
            "system_health": "nominal",
            "attention_level": "focused",
            "reasoning_depth": "deep",
        },
        cynic_thinking="Analyzing symbiotic equilibrium and alignment metrics",
        cynic_planning=[
            "Monitor resource allocation",
            "Update learning models",
            "Detect potential conflicts",
        ],
        cynic_confidence=0.618,
        cynic_e_score=0.75,
        # Human state
        human_energy=7.5,
        human_focus=8.0,
        human_intentions=["Complete CYNIC unification", "Optimize observability"],
        human_values=["Fairness", "Learning", "Growth"],
        human_feedback=[],
        human_growth_areas={},
        # Machine state
        machine_resources={
            "cpu_percent": 45.0,
            "memory_percent": 62.0,
            "disk_percent": 55.0,
            "network_bandwidth": 1024.0,
            "temperature": 55.0,
        },
        machine_constraints={"warnings": []},
        machine_capability_delta=[],
        machine_health={
            "is_healthy": True,
            "cpu_ok": True,
            "memory_ok": True,
            "disk_ok": True,
        },
        # Symbiotic relationship
        alignment_score=0.82,
        conflicts=[],
        mutual_influences=[],
        shared_objectives=["Complete CYNIC unification", "Optimize observability"],
        timestamp=time.time(),
    )


class TestProgressBar:
    """Tests for _render_progress_bar helper function."""

    def test_progress_bar_empty(self):
        """Progress bar with 0 value should be all empty."""
        result = _render_progress_bar(0.0, 100.0, width=10)
        assert result == "â-'" * 10

    def test_progress_bar_full(self):
        """Progress bar with max value should be all filled."""
        result = _render_progress_bar(100.0, 100.0, width=10)
        assert result == "â-ˆ" * 10

    def test_progress_bar_half(self):
        """Progress bar with half value should be half filled."""
        result = _render_progress_bar(50.0, 100.0, width=10)
        assert result == "â-ˆ" * 5 + "â-'" * 5

    def test_progress_bar_custom_width(self):
        """Progress bar respects custom width parameter."""
        result = _render_progress_bar(50.0, 100.0, width=20)
        assert len(result) == 20

    def test_progress_bar_clamps_value(self):
        """Progress bar clamps values outside [0, max]."""
        result_over = _render_progress_bar(150.0, 100.0, width=10)
        result_under = _render_progress_bar(-50.0, 100.0, width=10)
        assert result_over == "â-ˆ" * 10
        assert result_under == "â-'" * 10


class TestFormatPercentage:
    """Tests for _format_percentage helper function."""

    def test_format_percentage_default_precision(self):
        """Format percentage with default precision."""
        result = _format_percentage(45.678)
        assert result == "45.7%"

    def test_format_percentage_custom_precision(self):
        """Format percentage with custom precision."""
        result = _format_percentage(45.678, precision=2)
        assert result == "45.68%"

    def test_format_percentage_zero_precision(self):
        """Format percentage with zero decimal places."""
        result = _format_percentage(45.678, precision=0)
        assert result == "46%"

    def test_format_percentage_zero_value(self):
        """Format zero percentage."""
        result = _format_percentage(0.0)
        assert result == "0.0%"

    def test_format_percentage_hundred(self):
        """Format 100 percent."""
        result = _format_percentage(100.0)
        assert result == "100.0%"


class TestObserveView:
    """Tests for render_observe_view function."""

    def test_render_observe_view_returns_string(self, sample_state):
        """render_observe_view returns a string."""
        result = render_observe_view(sample_state)
        assert isinstance(result, str)

    def test_render_observe_view_has_header(self, sample_state):
        """OBSERVE view includes header section."""
        result = render_observe_view(sample_state)
        assert "OBSERVE" in result
        assert "Current State Snapshot" in result

    def test_render_observe_view_has_cynic_section(self, sample_state):
        """OBSERVE view includes CYNIC section."""
        result = render_observe_view(sample_state)
        assert "CYNIC" in result
        assert "Confidence" in result
        assert "E-Score" in result

    def test_render_observe_view_has_your_state_section(self, sample_state):
        """OBSERVE view includes YOUR STATE section."""
        result = render_observe_view(sample_state)
        assert "YOUR STATE" in result
        assert "Energy" in result
        assert "Focus" in result

    def test_render_observe_view_has_machine_section(self, sample_state):
        """OBSERVE view includes MACHINE CAPACITY section."""
        result = render_observe_view(sample_state)
        assert "MACHINE CAPACITY" in result
        assert "CPU" in result
        assert "Memory" in result
        assert "Disk" in result

    def test_render_observe_view_has_alignment_section(self, sample_state):
        """OBSERVE view includes SYMBIOTIC ALIGNMENT section."""
        result = render_observe_view(sample_state)
        assert "SYMBIOTIC ALIGNMENT" in result
        assert "Alignment" in result

    def test_render_observe_view_includes_thinking(self, sample_state):
        """OBSERVE view includes CYNIC thinking."""
        result = render_observe_view(sample_state)
        assert "Analyzing symbiotic equilibrium" in result

    def test_render_observe_view_includes_planning(self, sample_state):
        """OBSERVE view includes CYNIC planning items."""
        result = render_observe_view(sample_state)
        assert "Monitor resource allocation" in result
        assert "Update learning models" in result

    def test_render_observe_view_includes_intentions(self, sample_state):
        """OBSERVE view includes human intentions."""
        result = render_observe_view(sample_state)
        assert "Complete CYNIC unification" in result
        assert "Optimize observability" in result

    def test_render_observe_view_includes_values(self, sample_state):
        """OBSERVE view includes human values."""
        result = render_observe_view(sample_state)
        assert "Fairness" in result
        assert "Learning" in result
        assert "Growth" in result

    def test_render_observe_view_includes_progress_bars(self, sample_state):
        """OBSERVE view includes progress bars with â-ˆ and â-'."""
        result = render_observe_view(sample_state)
        assert "â-ˆ" in result
        assert "â-'" in result

    def test_render_observe_view_includes_shared_objectives(self, sample_state):
        """OBSERVE view includes shared objectives."""
        result = render_observe_view(sample_state)
        assert "Shared Objectives" in result

    def test_render_observe_view_no_conflicts(self, sample_state):
        """OBSERVE view shows no conflicts when none exist."""
        result = render_observe_view(sample_state)
        assert "Conflicts" in result

    def test_render_observe_view_with_conflicts(self, sample_state):
        """OBSERVE view shows conflicts when they exist."""
        state = SymbioticState(
            cynic_observations=sample_state.cynic_observations,
            cynic_thinking=sample_state.cynic_thinking,
            cynic_planning=sample_state.cynic_planning,
            cynic_confidence=sample_state.cynic_confidence,
            cynic_e_score=sample_state.cynic_e_score,
            human_energy=sample_state.human_energy,
            human_focus=sample_state.human_focus,
            human_intentions=sample_state.human_intentions,
            human_values=sample_state.human_values,
            human_feedback=sample_state.human_feedback,
            human_growth_areas=sample_state.human_growth_areas,
            machine_resources=sample_state.machine_resources,
            machine_constraints=sample_state.machine_constraints,
            machine_capability_delta=sample_state.machine_capability_delta,
            machine_health=sample_state.machine_health,
            alignment_score=sample_state.alignment_score,
            conflicts=["High memory usage detected"],
            mutual_influences=sample_state.mutual_influences,
            shared_objectives=sample_state.shared_objectives,
            timestamp=sample_state.timestamp,
        )
        result = render_observe_view(state)
        assert "High memory usage detected" in result


class TestCynicView:
    """Tests for render_cynic_view function."""

    def test_render_cynic_view_returns_string(self, sample_state):
        """render_cynic_view returns a string."""
        result = render_cynic_view(sample_state)
        assert isinstance(result, str)

    def test_render_cynic_view_has_header(self, sample_state):
        """CYNIC view includes header."""
        result = render_cynic_view(sample_state)
        assert "CYNIC MIND" in result
        assert "Deep Dive" in result

    def test_render_cynic_view_has_observations(self, sample_state):
        """CYNIC view includes observations section."""
        result = render_cynic_view(sample_state)
        assert "OBSERVATIONS" in result
        assert "system_health" in result
        assert "nominal" in result

    def test_render_cynic_view_has_thinking(self, sample_state):
        """CYNIC view includes current thinking."""
        result = render_cynic_view(sample_state)
        assert "CURRENT THINKING" in result
        assert "Analyzing symbiotic equilibrium" in result

    def test_render_cynic_view_has_confidence(self, sample_state):
        """CYNIC view includes confidence level."""
        result = render_cynic_view(sample_state)
        assert "Confidence Level" in result

    def test_render_cynic_view_has_planning(self, sample_state):
        """CYNIC view includes planning items."""
        result = render_cynic_view(sample_state)
        assert "PLANNING ITEMS" in result
        assert "Monitor resource allocation" in result
        assert "Update learning models" in result
        assert "Detect potential conflicts" in result

    def test_render_cynic_view_planning_numbered(self, sample_state):
        """CYNIC view planning items are numbered."""
        result = render_cynic_view(sample_state)
        assert "1." in result
        assert "2." in result
        assert "3." in result

    def test_render_cynic_view_has_e_score(self, sample_state):
        """CYNIC view includes E-Score section."""
        result = render_cynic_view(sample_state)
        assert "E-SCORE" in result
        assert "Energy/Consciousness" in result
        assert "0.75" in result

    def test_render_cynic_view_empty_observations(self):
        """CYNIC view handles empty observations gracefully."""
        state = SymbioticState(
            cynic_observations={},
            cynic_thinking="Thinking",
            cynic_planning=[],
            cynic_confidence=0.5,
            cynic_e_score=0.5,
            human_energy=5.0,
            human_focus=5.0,
            human_intentions=[],
            human_values=[],
            human_feedback=[],
            human_growth_areas={},
            machine_resources={},
            machine_constraints={},
            machine_capability_delta=[],
            machine_health={},
            alignment_score=0.5,
            conflicts=[],
            mutual_influences=[],
            shared_objectives=[],
            timestamp=time.time(),
        )
        result = render_cynic_view(state)
        assert "no observations" in result

    def test_render_cynic_view_empty_planning(self):
        """CYNIC view handles empty planning gracefully."""
        state = SymbioticState(
            cynic_observations={"test": "value"},
            cynic_thinking="Thinking",
            cynic_planning=[],
            cynic_confidence=0.5,
            cynic_e_score=0.5,
            human_energy=5.0,
            human_focus=5.0,
            human_intentions=[],
            human_values=[],
            human_feedback=[],
            human_growth_areas={},
            machine_resources={},
            machine_constraints={},
            machine_capability_delta=[],
            machine_health={},
            alignment_score=0.5,
            conflicts=[],
            mutual_influences=[],
            shared_objectives=[],
            timestamp=time.time(),
        )
        result = render_cynic_view(state)
        assert "no planning items" in result


class TestMachineView:
    """Tests for render_machine_view function."""

    def test_render_machine_view_returns_string(self, sample_state):
        """render_machine_view returns a string."""
        result = render_machine_view(sample_state)
        assert isinstance(result, str)

    def test_render_machine_view_has_header(self, sample_state):
        """MACHINE view includes header."""
        result = render_machine_view(sample_state)
        assert "MACHINE" in result
        assert "Resources & Health" in result

    def test_render_machine_view_has_resource_section(self, sample_state):
        """MACHINE view includes resource utilization section."""
        result = render_machine_view(sample_state)
        assert "RESOURCE UTILIZATION" in result

    def test_render_machine_view_has_cpu(self, sample_state):
        """MACHINE view includes CPU section."""
        result = render_machine_view(sample_state)
        assert "CPU" in result
        assert "45" in result  # 45.0%

    def test_render_machine_view_has_memory(self, sample_state):
        """MACHINE view includes memory section."""
        result = render_machine_view(sample_state)
        assert "Memory" in result
        assert "62" in result  # 62.0%

    def test_render_machine_view_has_disk(self, sample_state):
        """MACHINE view includes disk section."""
        result = render_machine_view(sample_state)
        assert "Disk" in result
        assert "55" in result  # 55.0%

    def test_render_machine_view_has_temperature(self, sample_state):
        """MACHINE view includes temperature if available."""
        result = render_machine_view(sample_state)
        assert "Temperature" in result
        assert "55" in result

    def test_render_machine_view_has_network(self, sample_state):
        """MACHINE view includes network bandwidth if available."""
        result = render_machine_view(sample_state)
        assert "Network" in result

    def test_render_machine_view_has_health_section(self, sample_state):
        """MACHINE view includes health indicators section."""
        result = render_machine_view(sample_state)
        assert "HEALTH INDICATORS" in result

    def test_render_machine_view_overall_health_ok(self, sample_state):
        """MACHINE view shows overall health check."""
        result = render_machine_view(sample_state)
        assert "Overall Health" in result
        assert "âœ"" in result

    def test_render_machine_view_cpu_health_ok(self, sample_state):
        """MACHINE view shows CPU health status."""
        result = render_machine_view(sample_state)
        assert "CPU OK" in result
        assert "âœ"" in result

    def test_render_machine_view_memory_health_ok(self, sample_state):
        """MACHINE view shows memory health status."""
        result = render_machine_view(sample_state)
        assert "Memory OK" in result
        assert "âœ"" in result

    def test_render_machine_view_disk_health_ok(self, sample_state):
        """MACHINE view shows disk health status."""
        result = render_machine_view(sample_state)
        assert "Disk OK" in result
        assert "âœ"" in result

    def test_render_machine_view_health_failed(self):
        """MACHINE view shows failed health checks with âœ-."""
        state = SymbioticState(
            cynic_observations={},
            cynic_thinking="Thinking",
            cynic_planning=[],
            cynic_confidence=0.5,
            cynic_e_score=0.5,
            human_energy=5.0,
            human_focus=5.0,
            human_intentions=[],
            human_values=[],
            human_feedback=[],
            human_growth_areas={},
            machine_resources={
                "cpu_percent": 90.0,
                "memory_percent": 95.0,
                "disk_percent": 85.0,
            },
            machine_constraints={},
            machine_capability_delta=[],
            machine_health={
                "is_healthy": False,
                "cpu_ok": False,
                "memory_ok": False,
                "disk_ok": False,
            },
            alignment_score=0.5,
            conflicts=[],
            mutual_influences=[],
            shared_objectives=[],
            timestamp=time.time(),
        )
        result = render_machine_view(state)
        # Should have multiple âœ- marks
        assert result.count("âœ-") >= 3

    def test_render_machine_view_constraints_shown(self):
        """MACHINE view shows constraint warnings."""
        state = SymbioticState(
            cynic_observations={},
            cynic_thinking="Thinking",
            cynic_planning=[],
            cynic_confidence=0.5,
            cynic_e_score=0.5,
            human_energy=5.0,
            human_focus=5.0,
            human_intentions=[],
            human_values=[],
            human_feedback=[],
            human_growth_areas={},
            machine_resources={},
            machine_constraints={
                "warnings": [
                    "Memory usage critical: 95.0%",
                    "CPU usage high: 85.0%",
                ]
            },
            machine_capability_delta=[],
            machine_health={"is_healthy": False},
            alignment_score=0.5,
            conflicts=[],
            mutual_influences=[],
            shared_objectives=[],
            timestamp=time.time(),
        )
        result = render_machine_view(state)
        assert "CONSTRAINTS & WARNINGS" in result
        assert "Memory usage critical" in result
        assert "CPU usage high" in result

    def test_render_machine_view_no_constraints(self, sample_state):
        """MACHINE view shows no warnings when none exist."""
        result = render_machine_view(sample_state)
        assert "no warnings" in result

    def test_render_machine_view_capability_delta(self):
        """MACHINE view shows capability changes."""
        state = SymbioticState(
            cynic_observations={},
            cynic_thinking="Thinking",
            cynic_planning=[],
            cynic_confidence=0.5,
            cynic_e_score=0.5,
            human_energy=5.0,
            human_focus=5.0,
            human_intentions=[],
            human_values=[],
            human_feedback=[],
            human_growth_areas={},
            machine_resources={},
            machine_constraints={},
            machine_capability_delta=["GPU detected"],
            machine_health={"is_healthy": True},
            alignment_score=0.5,
            conflicts=[],
            mutual_influences=[],
            shared_objectives=[],
            timestamp=time.time(),
        )
        result = render_machine_view(state)
        assert "CAPABILITY CHANGES" in result
        assert "GPU detected" in result

    def test_render_machine_view_progress_bars(self, sample_state):
        """MACHINE view includes progress bars."""
        result = render_machine_view(sample_state)
        assert "â-ˆ" in result
        assert "â-'" in result


class TestViewsIntegration:
    """Integration tests for all views."""

    def test_all_views_with_high_load_state(self):
        """All views handle high-load machine state correctly."""
        state = SymbioticState(
            cynic_observations={},
            cynic_thinking="System under strain",
            cynic_planning=["Reduce load", "Optimize memory"],
            cynic_confidence=0.3,
            cynic_e_score=0.4,
            human_energy=3.0,
            human_focus=2.0,
            human_intentions=["Debug performance"],
            human_values=[],
            human_feedback=[],
            human_growth_areas={},
            machine_resources={
                "cpu_percent": 95.0,
                "memory_percent": 98.0,
                "disk_percent": 92.0,
            },
            machine_constraints={
                "warnings": [
                    "Memory usage critical",
                    "Disk usage warning",
                ]
            },
            machine_capability_delta=[],
            machine_health={
                "is_healthy": False,
                "cpu_ok": False,
                "memory_ok": False,
                "disk_ok": False,
            },
            alignment_score=0.15,
            conflicts=[
                "High memory usage prevents task execution",
                "Human focus low with critical issues",
            ],
            mutual_influences=[],
            shared_objectives=["Debug performance"],
            timestamp=time.time(),
        )

        observe = render_observe_view(state)
        cynic = render_cynic_view(state)
        machine = render_machine_view(state)

        # All should be non-empty strings
        assert len(observe) > 50
        assert len(cynic) > 50
        assert len(machine) > 50

        # High load should be visible
        assert "95" in observe or "95" in machine
        assert "98" in observe or "98" in machine
