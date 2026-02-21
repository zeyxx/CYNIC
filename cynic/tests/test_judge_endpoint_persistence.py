"""
Phase 0: Test that persistence functions are awaitable (not fire-and-forget).
Verify both _persist_judgment_async and _write_guidance_async can be awaited,
so they can be integrated into orchestrator.run() synchronization boundary.
"""
import pytest
from unittest.mock import patch, AsyncMock
from cynic.core.judgment import Cell, Judgment
from cynic.api.routers.core import _persist_judgment_async, _write_guidance_async


@pytest.mark.asyncio
async def test_persistence_functions_are_awaitable():
    """Verify both persistence functions are awaitable (not fire-and-forget)."""

    # Create a test cell
    cell = Cell(
        reality="CODE",
        analysis="JUDGE",
        time_dim="PRESENT",
        content="test code",
        context="test context",
        lod=0,
        budget_usd=0.01,
    )

    # Create a test judgment
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

    # Patch and verify both functions are awaited (not fire-and-forget)
    with patch("cynic.api.routers.core._get_judgment_repo", return_value=mock_repo):
        # Both should be awaitable coroutines
        await _persist_judgment_async(judgment)
        await _write_guidance_async(cell, judgment)

    # Verify repo.save was called
    mock_repo.save.assert_called_once()
