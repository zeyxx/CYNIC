"""Tests for HumanStateTracker."""
import pytest
from cynic.kernel.observability.human_state_tracker import HumanStateTracker


@pytest.mark.asyncio
async def test_human_state_tracker_initializes():
    """HumanStateTracker can be created."""
    tracker = HumanStateTracker()
    assert tracker is not None


@pytest.mark.asyncio
async def test_get_human_state():
    """Can get current human state."""
    tracker = HumanStateTracker()
    state = await tracker.get_state()

    assert hasattr(state, 'energy')
    assert hasattr(state, 'focus')
    assert hasattr(state, 'intentions')
    assert 0 <= state.energy <= 10
    assert 0 <= state.focus <= 10


@pytest.mark.asyncio
async def test_report_feedback():
    """Can report human feedback."""
    tracker = HumanStateTracker()

    await tracker.report_feedback(
        feedback_type="correction",
        message="CYNIC was too conservative",
        confidence=0.8
    )

    state = await tracker.get_state()
    assert len(state.feedback) > 0
    assert "too conservative" in state.feedback[-1]
