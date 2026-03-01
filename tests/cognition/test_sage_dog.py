"""Tests for SageDog â€” wisdom evaluator and world-maker."""
from unittest.mock import AsyncMock, patch

import pytest

from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.phi import MAX_CONFIDENCE
from cynic.kernel.organism.brain.cognition.neurons.base import DogId, DogJudgment
from cynic.kernel.organism.brain.cognition.neurons.sage import SageDog


@pytest.fixture
def sage_dog():
    """Create a SageDog instance for testing."""
    return SageDog()


@pytest.fixture
def test_cell():
    """Create a test cell."""
    return Cell(
        cell_id="test-cell-sage-001",
        reality="CODE",
        analysis="JUDGE",
        content="""
def calculate_phi():
    '''Calculate the golden ratio.'''
    phi = (1 + 5 ** 0.5) / 2
    return phi
""",
        budget_usd=0.01,
    )


@pytest.mark.asyncio
async def test_sage_dog_initialization(sage_dog):
    """SageDog should initialize with correct ID."""
    assert sage_dog.dog_id == DogId.SAGE
    assert sage_dog.task_type == "wisdom"


@pytest.mark.asyncio
async def test_sage_dog_analyze_heuristic_only(sage_dog, test_cell):
    """SageDog should perform heuristic analysis when no LLM available."""
    # Mock get_llm to return None (forcing heuristic mode)
    with patch.object(SageDog, "get_llm", new_callable=AsyncMock) as mock_get_llm:
        mock_get_llm.return_value = None
        
        judgment = await sage_dog.analyze(test_cell)
        
        assert isinstance(judgment, DogJudgment)
        assert judgment.dog_id == DogId.SAGE
        assert 0 <= judgment.q_score <= 100
        # Should have low confidence in heuristic mode
        assert judgment.confidence < MAX_CONFIDENCE
        # Axiom scores are in evidence for heuristic path
        assert "axioms" in judgment.evidence
        assert "PHI" in judgment.evidence["axioms"]
        assert judgment.reasoning


@pytest.mark.asyncio
async def test_sage_dog_wisdom_patterns(sage_dog):
    """SageDog should recognize wisdom patterns in code."""
    content = 'def my_function():\n    """Docstring."""\n    return True'
    cell = Cell(cell_id="c1", content=content, reality="CODE", analysis="JUDGE")
    
    with patch.object(SageDog, "get_llm", new_callable=AsyncMock) as mock_get_llm:
        mock_get_llm.return_value = None
        judgment = await sage_dog.analyze(cell)
        
        # Good code should have a decent Q-score even in heuristic mode
        assert judgment.q_score > 50.0


@pytest.mark.asyncio
async def test_sage_dog_smell_patterns(sage_dog):
    """SageDog should recognize code smells."""
    content = 'class MyManager:\n    def do_nothing(self):\n        pass'
    cell = Cell(cell_id="c1", content=content, reality="CODE", analysis="JUDGE")
    
    with patch.object(SageDog, "get_llm", new_callable=AsyncMock) as mock_get_llm:
        mock_get_llm.return_value = None
        judgment = await sage_dog.analyze(cell)
        
        # Code smells should lower the Q-score
        assert judgment.q_score < 60.0


@pytest.mark.asyncio
async def test_sage_dog_dream_facets(sage_dog):
    """SageDog should generate dynamic facets using LLM."""
    mock_adapter = AsyncMock()
    mock_adapter.generate = AsyncMock(return_value="""
WISDOM: Practical application of knowledge
BALANCE: Harmony between competing interests
PHILO: Love of wisdom
REASON: Logical consistency
""")
    # Add llm_id to adapter
    mock_adapter.llm_id = "test-llm"
    
    with patch.object(SageDog, "get_llm", new_callable=AsyncMock) as mock_get_llm:
        mock_get_llm.return_value = mock_adapter
        
        # Mock registry for record_judgment if needed, but dream_facets doesn't call it.
        # Wait, let's see why it failed before: 
        # "object MagicMock can't be used in 'await' expression"
        # In sage.py:166:
        # await registry.record_world_building(reality, axiom, facets, self.dog_id, adapter.llm_id)
        
        mock_registry = AsyncMock()
        
        facets = await sage_dog.dream_facets("PHI", "PHILOSOPHY", mock_registry)
        
        assert isinstance(facets, dict)
        assert len(facets) > 0
        # Check for any of the expected facets (parsing might be loose)
        assert any(k in facets for k in ["WISDOM", "BALANCE", "PHILO", "REASON"])
        
        # Verify registry was called
        mock_registry.register_facet.assert_called()
