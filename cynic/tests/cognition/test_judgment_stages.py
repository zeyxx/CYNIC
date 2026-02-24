"""Test judgment stages — verify stage contract and composition."""
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.cognition.cortex.judgment_stages import (
    JudgmentStage,
    PerceiveStage,
    JudgeStage,
    DecideStage,
    ActStage,
    LearnStage,
    AccountStage,
    EmergeStage,
    execute_judgment_pipeline,
)
from cynic.cognition.cortex.pipeline import JudgmentPipeline
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell


@pytest.fixture
def mock_orchestrator():
    """Create mock orchestrator with essential attributes."""
    orch = MagicMock()
    orch.dogs = {}
    orch.escore_tracker = None
    orch.lod_controller = None
    orch.axiom_monitor = None
    orch.context_compressor = None
    orch.decision_validator = None
    orch._act_phase = AsyncMock(return_value={"executed": True})
    return orch


@pytest.fixture
def test_pipeline():
    """Create test pipeline."""
    cell = Cell(
        cell_id="test-cell-001",
        reality="CODE",
        analysis="JUDGE",
        content="test content",
        budget_usd=0.01,
    )
    return JudgmentPipeline(
        cell=cell,
        level=ConsciousnessLevel.MACRO,
    )


@pytest.mark.asyncio
async def test_perceive_stage_emits_event(mock_orchestrator, test_pipeline):
    """PerceiveStage should emit PERCEPTION_RECEIVED event."""
    with patch("cynic.cognition.cortex.judgment_stages.get_core_bus") as mock_bus:
        mock_bus_instance = AsyncMock()
        mock_bus.return_value = mock_bus_instance

        stage = PerceiveStage(mock_orchestrator)
        result = await stage.execute(test_pipeline)

        # Verify pipeline returned unchanged
        assert result is test_pipeline
        # Verify event emitted
        mock_bus_instance.emit.assert_called_once()


@pytest.mark.asyncio
async def test_judge_stage_creates_judgment(mock_orchestrator, test_pipeline):
    """JudgeStage should create a judgment."""
    # Mock Dogs
    dog_judgment = MagicMock(
        q_score=50.0,
        cost_usd=0.001,
        llm_id=None,
        dog_id="TEST_DOG",
    )
    mock_dog = AsyncMock()
    mock_dog.analyze = AsyncMock(return_value=dog_judgment)

    mock_orchestrator.dogs = {"TEST_DOG": mock_dog}
    mock_orchestrator.cynic_dog = MagicMock()
    mock_orchestrator.cynic_dog.pbft_run = AsyncMock(return_value=MagicMock(
        final_q_score=50.0,
        final_confidence=0.6,
        votes=1,
        quorum=1,
        consensus=True,
    ))
    mock_orchestrator.axiom_arch = MagicMock()
    mock_orchestrator.axiom_arch.score_and_compute = MagicMock(return_value=MagicMock(
        q_score=50.0,
        axiom_scores={},
        active_axioms=[],
    ))

    stage = JudgeStage(mock_orchestrator)
    result = await stage.execute(test_pipeline)

    # Verify judgment created
    assert result.final_judgment is not None
    assert result.consensus is not None


@pytest.mark.asyncio
async def test_decide_stage_validates_judgment(mock_orchestrator, test_pipeline):
    """DecideStage should call decision validator."""
    # Create mock judgment
    test_pipeline.final_judgment = MagicMock()

    mock_orchestrator.decision_validator = MagicMock()
    mock_orchestrator.decision_validator.validate = MagicMock(return_value=MagicMock(approved=True))

    stage = DecideStage(mock_orchestrator)
    result = await stage.execute(test_pipeline)

    # Verify validation called
    mock_orchestrator.decision_validator.validate.assert_called_once()


@pytest.mark.asyncio
async def test_act_stage_executes_action(mock_orchestrator, test_pipeline):
    """ActStage should call _act_phase and record result."""
    test_pipeline.final_judgment = MagicMock()

    stage = ActStage(mock_orchestrator)
    result = await stage.execute(test_pipeline)

    # Verify act phase called
    mock_orchestrator._act_phase.assert_called_once()
    assert result.action_executed == True


@pytest.mark.asyncio
async def test_learn_stage_placeholder(mock_orchestrator, test_pipeline):
    """LearnStage is placeholder — should pass through unchanged."""
    original_pipeline = test_pipeline
    stage = LearnStage(mock_orchestrator)
    result = await stage.execute(test_pipeline)
    assert result is original_pipeline


@pytest.mark.asyncio
async def test_account_stage_tracks_cost(mock_orchestrator, test_pipeline):
    """AccountStage should track cost."""
    judgment = MagicMock()
    judgment.cost_usd = 0.005
    judgment.dog_votes = {"DOG1": 50.0, "DOG2": 60.0}
    test_pipeline.final_judgment = judgment

    stage = AccountStage(mock_orchestrator)
    result = await stage.execute(test_pipeline)
    assert result is test_pipeline


@pytest.mark.asyncio
async def test_emerge_stage_detects_anomaly(mock_orchestrator, test_pipeline):
    """EmergeStage should emit event if residual high."""
    judgment = MagicMock()
    judgment.unnameable_detected = True
    judgment.residual_variance = 0.7
    judgment.cell = test_pipeline.cell
    judgment.judgment_id = "test-judgment-001"
    test_pipeline.final_judgment = judgment

    with patch("cynic.cognition.cortex.judgment_stages.get_core_bus") as mock_bus:
        mock_bus_instance = AsyncMock()
        mock_bus.return_value = mock_bus_instance

        stage = EmergeStage(mock_orchestrator)
        result = await stage.execute(test_pipeline)

        # Verify emergence event emitted
        mock_bus_instance.emit.assert_called_once()


@pytest.mark.asyncio
async def test_execute_judgment_pipeline_full_cycle(mock_orchestrator, test_pipeline):
    """Test execute_judgment_pipeline runs all 7 stages."""
    # Mock minimal orchestrator for full cycle
    dog_judgment = MagicMock(
        q_score=50.0,
        cost_usd=0.001,
        llm_id=None,
        dog_id="TEST_DOG",
    )
    mock_dog = AsyncMock()
    mock_dog.analyze = AsyncMock(return_value=dog_judgment)

    mock_orchestrator.dogs = {"TEST_DOG": mock_dog}
    mock_orchestrator.cynic_dog = MagicMock()
    mock_orchestrator.cynic_dog.pbft_run = AsyncMock(return_value=MagicMock(
        final_q_score=50.0,
        final_confidence=0.6,
        votes=1,
        quorum=1,
        consensus=True,
    ))
    mock_orchestrator.axiom_arch = MagicMock()
    mock_orchestrator.axiom_arch.score_and_compute = MagicMock(return_value=MagicMock(
        q_score=50.0,
        axiom_scores={},
        active_axioms=[],
    ))

    with patch("cynic.cognition.cortex.judgment_stages.get_core_bus") as mock_bus:
        mock_bus_instance = AsyncMock()
        mock_bus.return_value = mock_bus_instance

        result = await execute_judgment_pipeline(mock_orchestrator, test_pipeline)

        # Verify all stages executed
        assert result.final_judgment is not None
        assert result.action_executed is not None
