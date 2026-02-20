"""Tests for MCP Resources — Claude Code bridge."""

import pytest
from unittest.mock import AsyncMock, MagicMock
from cynic.mcp.resources import MCPResourceManager


@pytest.fixture
def mock_state():
    """Create mock app state."""
    state = MagicMock()
    state.decision_tracer = AsyncMock()
    state.loop_closure_validator = AsyncMock()
    state.event_journal = AsyncMock()
    return state


@pytest.fixture
def resource_manager(mock_state):
    """Create MCPResourceManager with mock state."""
    return MCPResourceManager(mock_state)


@pytest.mark.asyncio
async def test_get_similar_judgments(resource_manager, mock_state):
    """Test querying similar judgments."""
    # Mock trace data
    mock_trace = MagicMock()
    mock_trace.final_verdict = "WAG"
    mock_trace.final_q_score = 76.0
    mock_trace.to_dict.return_value = {
        "verdict": "WAG",
        "q_score": 76.0,
    }

    mock_state.decision_tracer.recent_traces.return_value = [mock_trace]

    result = await resource_manager.get_similar_judgments(
        query_q_score=75.0,
        query_verdict="WAG",
        limit=10,
    )

    assert "results" in result
    assert result["query"]["q_score"] == 75.0
    assert result["query"]["verdict"] == "WAG"


@pytest.mark.asyncio
async def test_get_judgment_reasoning(resource_manager, mock_state):
    """Test getting judgment reasoning."""
    # Mock trace with JUDGE phase
    mock_node = MagicMock()
    mock_node.phase = "JUDGE"
    mock_vote = MagicMock()
    mock_vote.dog_id = "SAGE"
    mock_vote.role = "primary"
    mock_vote.q_score = 75.0
    mock_vote.confidence = 0.61
    mock_vote.reasoning = "High confidence"
    mock_node.dog_votes = [mock_vote]

    mock_trace = MagicMock()
    mock_trace.trace_id = "t001"
    mock_trace.nodes = [mock_node]
    mock_trace.final_verdict = "WAG"
    mock_trace.final_q_score = 75.0
    mock_trace.final_confidence = 0.61
    mock_trace.max_depth = 3
    mock_trace.branching_factor = 1.2
    mock_trace.total_duration_ms = 100.0
    mock_trace.to_dict.return_value = {"trace": "data"}

    mock_state.decision_tracer.get_trace_by_judgment.return_value = mock_trace

    result = await resource_manager.get_judgment_reasoning("j001")

    assert result["judgment_id"] == "j001"
    assert result["final_verdict"] == "WAG"
    assert "dog_votes" in result
    assert result["dog_votes"][0]["dog_id"] == "SAGE"


@pytest.mark.asyncio
async def test_get_loop_status(resource_manager, mock_state):
    """Test getting loop closure status."""
    mock_stats = {
        "total_cycles": 100,
        "complete_cycles": 95,
        "stalled_cycles": 2,
        "orphan_cycles": 1,
        "completion_rate_percent": 95.0,
    }

    mock_state.loop_closure_validator.stats.return_value = mock_stats
    mock_state.loop_closure_validator.get_open_cycles.return_value = []
    mock_state.loop_closure_validator.get_stalled_phases.return_value = []
    mock_state.loop_closure_validator.get_orphan_judgments.return_value = []

    result = await resource_manager.get_loop_status()

    assert "health" in result
    assert result["health"]["completion_rate"] == 95.0
    assert result["health"]["is_healthy"] is True


@pytest.mark.asyncio
async def test_get_loop_status_unhealthy(resource_manager, mock_state):
    """Test loop status detection of health issues."""
    mock_stats = {
        "total_cycles": 100,
        "complete_cycles": 50,
        "stalled_cycles": 20,
        "completion_rate_percent": 50.0,
    }

    mock_stalled = MagicMock()
    mock_stalled.judgment_id = "j_stalled"

    mock_state.loop_closure_validator.stats.return_value = mock_stats
    mock_state.loop_closure_validator.get_open_cycles.return_value = []
    mock_state.loop_closure_validator.get_stalled_phases.return_value = [mock_stalled]
    mock_state.loop_closure_validator.get_orphan_judgments.return_value = []

    result = await resource_manager.get_loop_status()

    assert result["health"]["is_healthy"] is False
    assert "j_stalled" in result["alerts"]["stalled"]


@pytest.mark.asyncio
async def test_get_learned_patterns(resource_manager, mock_state):
    """Test getting learned patterns."""
    # Mock traces
    mock_trace = MagicMock()
    mock_trace.final_verdict = "WAG"
    mock_trace.final_q_score = 75.0

    mock_node = MagicMock()
    mock_vote = MagicMock()
    mock_vote.dog_id = "SAGE"
    mock_vote.q_score = 75.0
    mock_node.dog_votes = [mock_vote]

    mock_trace.nodes = [mock_node]

    mock_state.decision_tracer.recent_traces.return_value = [mock_trace]

    result = await resource_manager.get_learned_patterns(limit=20)

    assert "verdict_distribution" in result
    assert "dog_performance" in result
    assert "patterns" in result


@pytest.mark.asyncio
async def test_get_event_stream(resource_manager, mock_state):
    """Test getting recent event stream."""
    import time

    mock_event = MagicMock()
    mock_event.event_id = "e001"
    mock_event.event_type = "JUDGMENT_CREATED"
    mock_event.source = "SAGE"
    mock_event.timestamp_ms = time.time() * 1000.0
    mock_event.category = "judgment"

    mock_state.event_journal.time_range.return_value = [mock_event]

    result = await resource_manager.get_event_stream(limit=50)

    assert "events" in result
    assert len(result["events"]) == 1
    assert result["events"][0]["event_id"] == "e001"


@pytest.mark.asyncio
async def test_get_resource_similar_judgments(resource_manager, mock_state):
    """Test routing /mcp/judgments/similar URI."""
    mock_trace = MagicMock()
    mock_trace.final_verdict = "WAG"
    mock_trace.final_q_score = 75.0
    mock_trace.to_dict.return_value = {"trace": "data"}

    mock_state.decision_tracer.recent_traces.return_value = [mock_trace]

    result = await resource_manager.get_resource("/mcp/judgments/similar")

    assert "results" in result
    assert "query" in result


@pytest.mark.asyncio
async def test_get_resource_judgment_reasoning(resource_manager, mock_state):
    """Test routing /mcp/judgments/{judgment_id}/reasoning URI."""
    mock_trace = MagicMock()
    mock_trace.trace_id = "t001"
    mock_trace.nodes = []
    mock_trace.final_verdict = "WAG"
    mock_trace.final_q_score = 75.0
    mock_trace.final_confidence = 0.61
    mock_trace.max_depth = 3
    mock_trace.branching_factor = 1.2
    mock_trace.total_duration_ms = 100.0
    mock_trace.to_dict.return_value = {"trace": "data"}

    mock_state.decision_tracer.get_trace_by_judgment.return_value = mock_trace

    result = await resource_manager.get_resource("/mcp/judgments/j001/reasoning")

    assert result["judgment_id"] == "j001"
    assert "final_verdict" in result


@pytest.mark.asyncio
async def test_get_resource_loop_status(resource_manager, mock_state):
    """Test routing /mcp/loops/status URI."""
    mock_stats = {
        "total_cycles": 100,
        "complete_cycles": 95,
        "completion_rate_percent": 95.0,
    }

    mock_state.loop_closure_validator.stats.return_value = mock_stats
    mock_state.loop_closure_validator.get_open_cycles.return_value = []
    mock_state.loop_closure_validator.get_stalled_phases.return_value = []
    mock_state.loop_closure_validator.get_orphan_judgments.return_value = []

    result = await resource_manager.get_resource("/mcp/loops/status")

    assert "health" in result
    assert "timestamp_ms" in result


@pytest.mark.asyncio
async def test_get_resource_learning_patterns(resource_manager, mock_state):
    """Test routing /mcp/learning/patterns URI."""
    mock_state.decision_tracer.recent_traces.return_value = []

    result = await resource_manager.get_resource("/mcp/learning/patterns")

    assert "analyzed_traces" in result
    assert "verdict_distribution" in result


@pytest.mark.asyncio
async def test_get_resource_events(resource_manager, mock_state):
    """Test routing /mcp/events/recent URI."""
    mock_state.event_journal.time_range.return_value = []

    result = await resource_manager.get_resource("/mcp/events/recent")

    assert "events" in result


@pytest.mark.asyncio
async def test_get_resource_unknown_uri(resource_manager, mock_state):
    """Test handling unknown resource URI."""
    result = await resource_manager.get_resource("/mcp/unknown/endpoint")

    assert "error" in result


@pytest.mark.asyncio
async def test_error_handling_in_similar_judgments(resource_manager, mock_state):
    """Test error handling in resource methods."""
    mock_state.decision_tracer.recent_traces.side_effect = Exception("DB error")

    result = await resource_manager.get_similar_judgments(
        query_q_score=75.0,
        query_verdict="WAG",
    )

    assert "error" in result


@pytest.mark.asyncio
async def test_error_handling_in_loop_status(resource_manager, mock_state):
    """Test error handling in loop status."""
    mock_state.loop_closure_validator.stats.side_effect = Exception("Query error")

    result = await resource_manager.get_loop_status()

    assert "error" in result
