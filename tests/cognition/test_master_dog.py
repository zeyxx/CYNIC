"""Tests for MasterDog - the data-driven unified engine."""
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.kernel.core.event_bus import EventBus
from cynic.kernel.core.judgment import Cell
from cynic.kernel.organism.brain.cognition.neurons.base import DogId, DogJudgment
from cynic.kernel.organism.brain.cognition.neurons.master import MasterDog
from cynic.kernel.organism.brain.cognition.neurons.registry import get_soul


@pytest.fixture
def mock_bus():
    """Create a mock EventBus for tests (DI instead of global)."""
    bus = MagicMock(spec=EventBus)
    bus.emit = AsyncMock()
    return bus


@pytest.mark.asyncio
async def test_master_dog_as_sage(mock_bus):
    """Verify MasterDog can act as SAGE."""
    sage_soul = get_soul(DogId.SAGE)
    dog = MasterDog(sage_soul, bus=mock_bus)

    assert dog.dog_id == DogId.SAGE
    # Just check the dog ID, not exact sefirot string (encoding issues)
    assert "Wisdom" in dog.soul.sefirot

    cell = Cell(cell_id="c1", reality="CODE", content="print('hello')", analysis="JUDGE")

    # Test heuristic path
    with patch.object(MasterDog, "get_llm", new_callable=AsyncMock) as mock_llm:
        mock_llm.return_value = None
        judgment = await dog.analyze(cell)

        assert isinstance(judgment, DogJudgment)
        assert judgment.dog_id == DogId.SAGE
        assert "instinct" in judgment.reasoning.lower()

@pytest.mark.asyncio
async def test_master_dog_as_scholar(mock_bus):
    """Verify MasterDog can act as SCHOLAR."""
    scholar_soul = get_soul(DogId.SCHOLAR)
    dog = MasterDog(scholar_soul, bus=mock_bus)

    assert dog.dog_id == DogId.SCHOLAR
    assert dog.soul.expertise_fn == "tfidf_lookup"

    cell = Cell(cell_id="c2", reality="CODE", content="def x(): pass", analysis="JUDGE")

    with patch.object(MasterDog, "get_llm", new_callable=AsyncMock) as mock_llm:
        mock_llm.return_value = None
        judgment = await dog.analyze(cell)

        assert judgment.dog_id == DogId.SCHOLAR
        assert "instinct" in judgment.reasoning.lower()
