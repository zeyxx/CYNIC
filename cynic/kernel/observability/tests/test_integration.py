"""Integration tests for full symbiotic flow and state immutability.

Tests the complete workflow of:
1. Getting current symbiotic state from the unified state manager
2. Rendering all three views (observe, cynic, machine)
3. Verifying key content is present in renders
4. Verifying state immutability across all fields
"""

import pytest

from cynic.kernel.observability.cli.views import (
    render_cynic_view,
    render_machine_view,
    render_observe_view,
)
from cynic.kernel.observability.symbiotic_state_manager import get_current_state


@pytest.mark.asyncio
async def test_full_symbiotic_flow():
    """Test full symbiotic flow: get state and render all three views.

    This integration test verifies:
    - get_current_state() returns valid state
    - All three views can be rendered successfully
    - Each view contains key identifying strings
    - View lengths indicate substantial content (>100 chars)
    """
    # Get current unified state
    state = await get_current_state()

    # Verify state is valid
    assert state is not None
    assert state.timestamp > 0

    # Render all three views
    observe_view = render_observe_view(state)
    cynic_view = render_cynic_view(state)
    machine_view = render_machine_view(state)

    # Verify renders are substantial (>100 chars each)
    assert len(observe_view) > 100
    assert len(cynic_view) > 100
    assert len(machine_view) > 100

    # OBSERVE view should contain key strings
    assert "OBSERVE" in observe_view
    assert "CONSCIOUSNESS" in observe_view or "CYNIC" in observe_view
    assert "YOUR STATE" in observe_view

    # CYNIC view should contain key strings
    assert "MIND" in cynic_view or "CYNIC" in cynic_view
    assert "THINKING" in cynic_view

    # MACHINE view should contain key strings
    assert "RESOURCE UTILIZATION" in machine_view or "RESOURCES" in machine_view
    assert "MACHINE" in machine_view


@pytest.mark.asyncio
async def test_state_immutability():
    """Test that SymbioticState is truly immutable.

    Attempts to modify frozen dataclass fields should raise AttributeError.
    """
    # Get current state
    state = await get_current_state()

    # Attempt to modify alignment_score - should raise AttributeError
    with pytest.raises(AttributeError):
        state.alignment_score = 0.5  # type: ignore

    # Attempt to modify human_energy - should raise AttributeError
    with pytest.raises(AttributeError):
        state.human_energy = 7.5  # type: ignore

    # Also verify a few other fields can't be modified
    with pytest.raises(AttributeError):
        state.cynic_confidence = 0.9  # type: ignore

    with pytest.raises(AttributeError):
        state.human_focus = 5.0  # type: ignore

    with pytest.raises(AttributeError):
        state.timestamp = 999999.0  # type: ignore
