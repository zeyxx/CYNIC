"""Test judgment stages — verify stage contract and composition."""
from unittest.mock import AsyncMock, MagicMock

import pytest

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell
from cynic.kernel.organism.brain.cognition.cortex.judgment_stages import (
    AccountStage,
    ActStage,
    DecideStage,
    EmergeStage,
    JudgeStage,
    LearnStage,
    PerceiveStage,
    execute_judgment_pipeline,
)
from cynic.kernel.organism.brain.cognition.cortex.pipeline import JudgmentPipeline


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

    # Level 2: Mock the instance bus (used by judgment_stages to emit events)
    orch.bus = AsyncMock()
    orch.bus.emit = AsyncMock()

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
    """PerceiveStage should emit PERCEPTION_RECEIVED event (immutable evolution)."""
    stage = PerceiveStage(mock_orchestrator)
    result = await stage.execute(test_pipeline)

    # Verify immutable evolution: new instance with preserved lineage
    assert result is not test_pipeline, "Should return new evolved instance"
    assert result.trace_id == test_pipeline.trace_id, "Trace ID preserved"
    assert result.pipeline_id == test_pipeline.pipeline_id, "Pipeline ID unchanged"
    # Verify event emitted on orchestrator's bus
    mock_orchestrator.bus.emit.assert_called_once()


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
    mock_orchestrator.cynic_dog.phi_bft_run = AsyncMock(return_value=MagicMock(
        final_q_score=50.0,
        final_confidence=0.6,
        votes=1,
        quorum=1,
        consensus=True,
    ))
    mock_orchestrator.axiom_arch = MagicMock()
    mock_orchestrator.axiom_arch.score_and_compute = AsyncMock(return_value=MagicMock(
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
    # Create mock judgment (evolve creates new instance since pipeline is frozen)
    mock_judgment = MagicMock()
    pipeline_with_judgment = test_pipeline.evolve(final_judgment=mock_judgment)

    mock_orchestrator.decision_validator = MagicMock()
    mock_orchestrator.decision_validator.validate = MagicMock(return_value=MagicMock(approved=True))

    stage = DecideStage(mock_orchestrator)
    result = await stage.execute(pipeline_with_judgment)

    # Verify validation called
    mock_orchestrator.decision_validator.validate.assert_called_once()
    # Verify lineage preserved
    assert result.trace_id == pipeline_with_judgment.trace_id


@pytest.mark.asyncio
async def test_act_stage_executes_action(mock_orchestrator, test_pipeline):
    """ActStage should call _act_phase and record result."""
    # Evolve to add judgment (frozen pipeline)
    pipeline_with_judgment = test_pipeline.evolve(final_judgment=MagicMock())

    stage = ActStage(mock_orchestrator)
    result = await stage.execute(pipeline_with_judgment)

    # Verify act phase called
    mock_orchestrator._act_phase.assert_called_once()
    assert result.action_executed is True
    assert result.trace_id == pipeline_with_judgment.trace_id


@pytest.mark.asyncio
async def test_learn_stage_placeholder(mock_orchestrator, test_pipeline):
    """LearnStage is placeholder - should evolve immutably with same data."""
    original_pipeline = test_pipeline
    stage = LearnStage(mock_orchestrator)
    result = await stage.execute(test_pipeline)
    # With functional paradigm, returns evolved instance (new object, same data)
    assert result is not original_pipeline, "Should return new evolved instance"
    assert result.trace_id == original_pipeline.trace_id, "Lineage preserved"
    assert result.pipeline_id == original_pipeline.pipeline_id, "Pipeline ID same"
    assert result.learning_applied is False, "No learning applied in placeholder stage"


@pytest.mark.asyncio
async def test_account_stage_tracks_cost(mock_orchestrator, test_pipeline):
    """AccountStage should track cost (functional evolution)."""
    judgment = MagicMock()
    judgment.cost_usd = 0.005
    judgment.dog_votes = {"DOG1": 50.0, "DOG2": 60.0}
    # Evolve to add judgment (frozen pipeline)
    pipeline_with_judgment = test_pipeline.evolve(final_judgment=judgment)

    stage = AccountStage(mock_orchestrator)
    result = await stage.execute(pipeline_with_judgment)
    # Verify immutable evolution
    assert result is not pipeline_with_judgment, "Returns new evolved instance"
    assert result.trace_id == pipeline_with_judgment.trace_id, "Lineage preserved"
    assert result.total_cost_usd >= 0, "Cost tracked"


@pytest.mark.asyncio
async def test_emerge_stage_detects_anomaly(mock_orchestrator, test_pipeline):
    """EmergeStage should emit event if residual high."""
    judgment = MagicMock()
    judgment.unnameable_detected = True
    judgment.residual_variance = 0.7
    judgment.cell = test_pipeline.cell
    judgment.judgment_id = "test-judgment-001"
    # Evolve to add judgment (frozen pipeline)
    pipeline_with_judgment = test_pipeline.evolve(final_judgment=judgment)

    stage = EmergeStage(mock_orchestrator)
    result = await stage.execute(pipeline_with_judgment)

    # Verify immutable evolution
    assert result is not pipeline_with_judgment
    assert result.trace_id == pipeline_with_judgment.trace_id
    # Verify emergence event emitted on orchestrator's bus
    mock_orchestrator.bus.emit.assert_called_once()


@pytest.mark.asyncio
async def test_execute_judgment_pipeline_full_cycle(mock_orchestrator, test_pipeline):
    """Test execute_judgment_pipeline runs all 7 stages (functional DAG)."""
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
    mock_orchestrator.cynic_dog.phi_bft_run = AsyncMock(return_value=MagicMock(
        final_q_score=50.0,
        final_confidence=0.6,
        votes=1,
        quorum=1,
        consensus=True,
    ))
    mock_orchestrator.axiom_arch = MagicMock()
    mock_orchestrator.axiom_arch.score_and_compute = AsyncMock(return_value=MagicMock(
        q_score=50.0,
        axiom_scores={},
        active_axioms=[],
    ))

    # No patch needed: refactor uses instance bus (orchestrator.bus)
    result = await execute_judgment_pipeline(mock_orchestrator, test_pipeline)

    # Verify all stages executed
    assert result.final_judgment is not None, "Judgment should be created"
    assert result.action_executed is not None, "Action should be recorded"
    assert result.trace_id == test_pipeline.trace_id, "Lineage preserved through DAG"
