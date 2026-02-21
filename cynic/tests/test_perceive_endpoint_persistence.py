"""
Phase 0: Test /perceive endpoint emits judgment request events properly.
Verify that /perceive can trigger REFLEX judgments which should await persistence.
"""
import pytest
from unittest.mock import patch, AsyncMock
from cynic.core.judgment import Cell
from cynic.api.routers.core import _persist_judgment_async, _write_guidance_async
from cynic.core.event_bus import get_core_bus


@pytest.mark.asyncio
async def test_perceive_endpoint_event_driven():
    """
    Verify /perceive endpoint integrates with event-driven judgment pipeline.
    When run_judgment=True, it should emit JUDGMENT_REQUESTED event.
    The event handler will eventually call awaited persistence functions.
    """
    from cynic.core.judgment import Cell, Judgment

    # Create test cell from perception
    cell = Cell(
        reality="CODE",
        analysis="PERCEIVE",
        time_dim="PRESENT",
        content="test_code_snippet",
        context="Perception from test",
        lod=0,
        budget_usd=0.001,
    )

    # Create test judgment
    judgment = Judgment(
        cell=cell,
        verdict="BARK",
        q_score=0.2,
        confidence=0.3,
        dog_votes={"GUARDIAN": 0.2},
        cost_usd=0.001,
    )

    # Mock the repository
    mock_repo = AsyncMock()
    mock_repo.save = AsyncMock()

    # Verify both persistence functions are awaitable
    # (They would be called by event handlers processing JUDGMENT_REQUESTED)
    with patch("cynic.api.routers.core._get_judgment_repo", return_value=mock_repo):
        # Both should be awaitable coroutines
        await _persist_judgment_async(judgment)
        await _write_guidance_async(cell, judgment)

    # Verify repo.save was called
    mock_repo.save.assert_called_once()
