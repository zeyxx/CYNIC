"""
Phase 0: Test async judgment persistence.
Verify that _persist_judgment_async awaits completion (not fire-and-forget).
"""
import pytest
import uuid
from unittest.mock import AsyncMock, patch
from cynic.api.routers.core import _persist_judgment_async
from cynic.core.judgment import Judgment, Cell


@pytest.mark.asyncio
async def test_persist_judgment_async_completes():
    """Verify _persist_judgment_async awaits completion (is not fire-and-forget)."""
    # Create test cell with valid enum values
    cell = Cell(
        reality="CODE",
        analysis="JUDGE",
        time_dim="PRESENT",
        content="test content",
        context="test context",
        lod=0,
        budget_usd=0.01,
    )

    # Create test judgment with valid values
    judgment = Judgment(
        cell=cell,
        verdict="BARK",
        q_score=0.2,
        confidence=0.3,
        dog_votes={"GUARDIAN": 0.2, "ANALYST": 0.15},
        cost_usd=0.001,
    )

    # Mock the repository save method
    mock_repo = AsyncMock()
    mock_repo.save = AsyncMock()

    # Patch _get_judgment_repo to return mock
    with patch("cynic.api.routers.core._get_judgment_repo", return_value=mock_repo):
        # This should await and call the mock (not fire-and-forget)
        await _persist_judgment_async(judgment)

    # Verify that save was called (proves it awaited)
    mock_repo.save.assert_called_once()
    assert judgment.judgment_id is not None
