"""Tests for JudgeOrchestrator — the heart of the judgment cycle."""
import time
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell, ConsensusResult
from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgeOrchestrator
from cynic.kernel.organism.brain.cognition.neurons.base import DogId, DogJudgment


@pytest.fixture
def mock_dogs():
    """Create a set of mock Dogs."""
    dogs = {}
    for dog_id in [DogId.SAGE, DogId.ANALYST, DogId.GUARDIAN]:
        dog = AsyncMock()
        dog.dog_id = dog_id
        
        # Mock analyze to return a valid DogJudgment
        judgment = DogJudgment(
            dog_id=dog_id,
            cell_id="test-cell-001",
            q_score=50.0,
            confidence=0.5,
            reasoning="Test judgment",
            timestamp=time.time()
        )
        dog.analyze = AsyncMock(return_value=judgment)
        dogs[dog_id] = dog
    return dogs


@pytest.fixture
def mock_axiom_arch():
    """Create a mock AxiomArchitecture."""
    arch = MagicMock()
    arch.score_and_compute = MagicMock(return_value=MagicMock(
        q_score=50.0,
        axiom_scores={"PHI": 0.5},
        active_axioms=["PHI"],
    ))
    return arch


@pytest.fixture
def mock_cynic_dog():
    """Create a mock CynicDog (PBFT coordinator)."""
    dog = AsyncMock()
    dog.pbft_run = AsyncMock(return_value=ConsensusResult(
        final_q_score=50.0,
        final_confidence=0.6,
        votes=3,
        quorum=2,
        consensus=True,
        verdict="WAG",
        reasoning="Test consensus",
    ))
    return dog


@pytest.fixture
def orchestrator(mock_dogs, mock_axiom_arch, mock_cynic_dog):
    """Create a JudgeOrchestrator instance."""
    return JudgeOrchestrator(
        dogs=mock_dogs,
        axiom_arch=mock_axiom_arch,
        cynic_dog=mock_cynic_dog,
    )


@pytest.fixture
def test_cell():
    """Create a test cell."""
    return Cell(
        cell_id="test-cell-001",
        reality="CODE",
        analysis="JUDGE",
        content="test content",
        budget_usd=0.01,
    )


@pytest.mark.asyncio
async def test_orchestrator_initialization(orchestrator):
    """Orchestrator should initialize with components."""
    assert orchestrator.dogs is not None
    assert orchestrator.axiom_arch is not None
    assert orchestrator.cynic_dog is not None
    assert hasattr(orchestrator, "_composer")


@pytest.mark.asyncio
async def test_orchestrator_run_macro_cycle(orchestrator, test_cell):
    """Orchestrator should run a full MACRO cycle."""
    # Ensure composer is initialized
    orchestrator._ensure_composer()
    
    # Mock final_judgment more completely to avoid to_dict() issues
    mock_judgment = MagicMock()
    mock_judgment.q_score = 50.0
    mock_judgment.verdict = "WAG"
    mock_judgment.consensus_reached = True
    mock_judgment.judgment_id = "j1"
    mock_judgment.confidence = 0.6
    mock_judgment.consensus_votes = 3
    mock_judgment.consensus_quorum = 2
    mock_judgment.cost_usd = 0.0
    mock_judgment.duration_ms = 0.0
    mock_judgment.dog_votes = {}
    mock_judgment.axiom_scores = {}
    mock_judgment.active_axioms = []
    mock_judgment.residual_variance = 0.0
    mock_judgment.reasoning = "Test reasoning"
    
    # Mock model_copy to return itself
    mock_judgment.model_copy.return_value = mock_judgment
    
    mock_judgment.to_dict.return_value = {
        "q_score": 50.0,
        "verdict": "WAG",
        "judgment_id": "j1",
        "consensus_reached": True,
        "confidence": 0.6,
        "consensus_votes": 3,
        "consensus_quorum": 2,
        "cost_usd": 0.0,
        "latency_ms": 0.0,
        "dog_votes": {},
        "axiom_scores": {},
        "active_axioms": [],
        "residual_variance": 0.0,
        "cell_id": "test-cell-001",
        "timestamp": time.time(),
    }

    # Mock the composer.compose to return our judgment
    from cynic.kernel.organism.brain.cognition.cortex.handlers.base import HandlerResult
    compose_result = HandlerResult(
        success=True,
        handler_id="macro",
        output=mock_judgment,
        duration_ms=100.0
    )
    
    with patch.object(orchestrator._composer, "compose", new_callable=AsyncMock) as mock_compose:
        mock_compose.return_value = compose_result
        
        with patch.object(orchestrator._dialogue_agent, "explain_judgment", new_callable=AsyncMock) as mock_explain:
            mock_explain.return_value = "Mocked explanation"
            
            judgment = await orchestrator.run(test_cell, level=ConsciousnessLevel.MACRO)
            
            assert judgment is not None
            assert judgment.q_score == 50.0
            mock_compose.assert_called_once()


@pytest.mark.asyncio
async def test_orchestrator_run_reflex_cycle(orchestrator, test_cell):
    """Orchestrator should run a REFLEX cycle."""
    orchestrator._ensure_composer()
    
    mock_judgment = MagicMock()
    mock_judgment.q_score = 40.0
    mock_judgment.verdict = "BARK"
    mock_judgment.consensus_reached = False
    mock_judgment.judgment_id = "j2"
    mock_judgment.confidence = 0.3
    mock_judgment.consensus_votes = 1
    mock_judgment.consensus_quorum = 3
    mock_judgment.cost_usd = 0.0
    mock_judgment.duration_ms = 0.0
    mock_judgment.dog_votes = {}
    mock_judgment.axiom_scores = {}
    mock_judgment.active_axioms = []
    mock_judgment.residual_variance = 0.1
    mock_judgment.reasoning = ""
    
    # Mock model_copy to return itself
    mock_judgment.model_copy.return_value = mock_judgment
    
    mock_judgment.to_dict.return_value = {
        "q_score": 40.0,
        "verdict": "BARK",
        "judgment_id": "j2",
        "consensus_reached": False,
        "confidence": 0.3,
        "consensus_votes": 1,
        "consensus_quorum": 3,
        "cost_usd": 0.0,
        "latency_ms": 0.0,
        "dog_votes": {},
        "axiom_scores": {},
        "active_axioms": [],
        "residual_variance": 0.1,
        "cell_id": "test-cell-001",
        "timestamp": time.time(),
    }

    from cynic.kernel.organism.brain.cognition.cortex.handlers.base import HandlerResult
    compose_result = HandlerResult(
        success=True,
        handler_id="reflex",
        output=mock_judgment,
        duration_ms=10.0
    )
    
    with patch.object(orchestrator._composer, "compose", new_callable=AsyncMock) as mock_compose:
        mock_compose.return_value = compose_result
        
        with patch.object(orchestrator._dialogue_agent, "explain_judgment", new_callable=AsyncMock) as mock_explain:
            mock_explain.return_value = "Mocked reflex explanation"
            
            judgment = await orchestrator.run(test_cell, level=ConsciousnessLevel.REFLEX)
            
            assert judgment.q_score == 40.0
            mock_compose.assert_called_once()
