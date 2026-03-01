"""Tests for symbiotic observability data models."""

from dataclasses import fields, is_dataclass

import pytest

from cynic.kernel.observability.models import SymbioticState


def test_symbiotic_state_is_frozen_dataclass():
    """SymbioticState must be immutable."""
    assert is_dataclass(SymbioticState)

    # Check that it's frozen
    state = SymbioticState(
        cynic_observations={},
        cynic_thinking="",
        cynic_planning=[],
        cynic_confidence=0.5,
        cynic_e_score=0.618,
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
        timestamp=0.0,
    )

    # Should not be able to mutate
    with pytest.raises(AttributeError):
        state.cynic_thinking = "new thinking"


def test_symbiotic_state_has_all_required_fields():
    """SymbioticState has all documented fields."""
    state_fields = {f.name for f in fields(SymbioticState)}
    required = {
        # CYNIC
        "cynic_observations",
        "cynic_thinking",
        "cynic_planning",
        "cynic_confidence",
        "cynic_e_score",
        # Human
        "human_energy",
        "human_focus",
        "human_intentions",
        "human_values",
        "human_feedback",
        "human_growth_areas",
        # Machine
        "machine_resources",
        "machine_constraints",
        "machine_capability_delta",
        "machine_health",
        # Symbiotic
        "alignment_score",
        "conflicts",
        "mutual_influences",
        "shared_objectives",
        "timestamp",
    }
    assert required.issubset(state_fields)
