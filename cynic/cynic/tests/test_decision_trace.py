"""Tests for DecisionTracer — decision path tracing and DAG construction."""

import asyncio
import pytest
from cynic.nervous.decision_trace import DecisionTracer, DogVote, DogRole, TRACE_CAP


@pytest.fixture
def tracer():
    """Create fresh DecisionTracer for each test."""
    return DecisionTracer()


@pytest.mark.asyncio
async def test_start_trace(tracer):
    """Test starting a new trace."""
    trace_id = await tracer.start_trace(judgment_id="j001")
    assert isinstance(trace_id, str)
    assert len(trace_id) == 16

    # Verify stored
    trace = await tracer.get_trace(trace_id)
    assert trace is not None
    assert trace.judgment_id == "j001"
    assert len(trace.nodes) == 1  # Root PERCEIVE node


@pytest.mark.asyncio
async def test_add_node_to_trace(tracer):
    """Test adding nodes to a trace."""
    trace_id = await tracer.start_trace(judgment_id="j001")

    # Add JUDGE node
    node_id = await tracer.add_node(
        trace_id=trace_id,
        phase="JUDGE",
        component="SAGE",
        duration_ms=50.0,
        input_keys=["cell", "context"],
        input_sources=["perceiver", "compressor"],
        output_keys=["q_score", "verdict"],
        output_verdict="WAG",
        output_q_score=75.0,
    )

    assert isinstance(node_id, str)

    # Verify added
    trace = await tracer.get_trace(trace_id)
    assert len(trace.nodes) == 2  # Root + JUDGE
    assert trace.nodes[1].phase == "JUDGE"
    assert trace.nodes[1].output_verdict == "WAG"


@pytest.mark.asyncio
async def test_dog_votes_captured(tracer):
    """Test capturing dog votes in trace."""
    trace_id = await tracer.start_trace(judgment_id="j001")

    # Add JUDGE node
    await tracer.add_node(
        trace_id=trace_id,
        phase="JUDGE",
        component="SAGE",
        duration_ms=50.0,
        input_keys=["cell"],
        input_sources=["perceiver"],
        output_keys=["q_score"],
        output_verdict="WAG",
        output_q_score=75.0,
    )

    # Add dog votes
    votes = [
        DogVote(
            dog_id="SAGE",
            role=DogRole.PRIMARY,
            q_score=75.0,
            confidence=0.61,
            reasoning="Temporal MCTS: high confidence in scoring",
        ),
        DogVote(
            dog_id="ANALYST",
            role=DogRole.VOTER,
            q_score=70.0,
            confidence=0.55,
            reasoning="Pattern match: similar past judgments",
        ),
    ]

    await tracer.add_dog_votes(trace_id, phase_index=1, dog_votes=votes)

    # Verify votes stored
    trace = await tracer.get_trace(trace_id)
    judge_node = trace.nodes[1]
    assert len(judge_node.dog_votes) == 2
    assert judge_node.dog_votes[0].dog_id == "SAGE"
    assert judge_node.dog_votes[1].dog_id == "ANALYST"


@pytest.mark.asyncio
async def test_dag_construction(tracer):
    """Test DAG edges are built correctly."""
    trace_id = await tracer.start_trace(judgment_id="j001")

    # Add JUDGE node
    await tracer.add_node(
        trace_id=trace_id,
        phase="JUDGE",
        component="SAGE",
        duration_ms=50.0,
        input_keys=["cell"],
        input_sources=["perceiver"],
        output_keys=["q_score"],
    )

    # Add DECIDE node
    await tracer.add_node(
        trace_id=trace_id,
        phase="DECIDE",
        component="BRAIN",
        duration_ms=30.0,
        input_keys=["q_score"],
        input_sources=["SAGE"],
        output_keys=["action"],
    )

    # Verify edges
    trace = await tracer.get_trace(trace_id)
    assert len(trace.edges) == 2  # PERCEIVE->JUDGE, JUDGE->DECIDE
    assert trace.edges[0][1] == trace.nodes[1].node_id  # Edge to JUDGE
    assert trace.edges[1][0] == trace.nodes[1].node_id  # Edge from JUDGE


@pytest.mark.asyncio
async def test_close_trace_with_metrics(tracer):
    """Test closing trace computes metrics."""
    trace_id = await tracer.start_trace(judgment_id="j001")

    await tracer.add_node(
        trace_id=trace_id,
        phase="JUDGE",
        component="SAGE",
        duration_ms=50.0,
        input_keys=["cell"],
        input_sources=["perceiver"],
        output_keys=["q_score"],
    )

    await tracer.add_node(
        trace_id=trace_id,
        phase="DECIDE",
        component="BRAIN",
        duration_ms=30.0,
        input_keys=["q_score"],
        input_sources=["SAGE"],
        output_keys=["action"],
    )

    # Close trace
    await tracer.close_trace(
        trace_id=trace_id,
        final_verdict="WAG",
        final_q_score=75.0,
        final_confidence=0.61,
    )

    # Verify metrics computed
    trace = await tracer.get_trace(trace_id)
    assert trace.final_verdict == "WAG"
    assert trace.final_q_score == 75.0
    assert trace.total_duration_ms == 80.0  # 50 + 30
    assert trace.max_depth > 0


@pytest.mark.asyncio
async def test_recent_traces(tracer):
    """Test retrieving recent traces."""
    # Create 5 traces
    trace_ids = []
    for i in range(5):
        tid = await tracer.start_trace(judgment_id=f"j{i:03d}")
        trace_ids.append(tid)
        await tracer.close_trace(tid, final_verdict="WAG")
        await asyncio.sleep(0.01)

    # Get last 3
    recent = await tracer.recent_traces(limit=3)
    assert len(recent) == 3
    # Should be newest first
    assert recent[0].judgment_id == "j004"
    assert recent[1].judgment_id == "j003"
    assert recent[2].judgment_id == "j002"


@pytest.mark.asyncio
async def test_get_trace_by_judgment(tracer):
    """Test looking up trace by judgment_id."""
    trace_id = await tracer.start_trace(judgment_id="j_special")

    # Retrieve by judgment_id
    trace = await tracer.get_trace_by_judgment("j_special")
    assert trace is not None
    assert trace.trace_id == trace_id
    assert trace.judgment_id == "j_special"


@pytest.mark.asyncio
async def test_traces_by_verdict(tracer):
    """Test filtering traces by verdict."""
    # Create mixed verdicts
    for i, verdict in enumerate(["WAG", "WAG", "GROWL", "HOWL"]):
        tid = await tracer.start_trace(judgment_id=f"j{i}")
        await tracer.close_trace(tid, final_verdict=verdict)

    # Filter by WAG
    wag_traces = await tracer.traces_by_verdict("WAG")
    assert len(wag_traces) == 2
    assert all(t.final_verdict == "WAG" for t in wag_traces)

    # Filter by GROWL
    growl_traces = await tracer.traces_by_verdict("GROWL")
    assert len(growl_traces) == 1
    assert growl_traces[0].final_verdict == "GROWL"


@pytest.mark.asyncio
async def test_traces_by_component(tracer):
    """Test filtering traces by component involvement."""
    tid1 = await tracer.start_trace(judgment_id="j1")
    await tracer.add_node(
        tid1, phase="JUDGE", component="SAGE",
        duration_ms=10.0, input_keys=[], input_sources=[],
        output_keys=[],
    )
    await tracer.close_trace(tid1)

    tid2 = await tracer.start_trace(judgment_id="j2")
    await tracer.add_node(
        tid2, phase="JUDGE", component="ANALYST",
        duration_ms=10.0, input_keys=[], input_sources=[],
        output_keys=[],
    )
    await tracer.close_trace(tid2)

    # Filter by component
    sage_traces = await tracer.traces_by_component("SAGE")
    assert len(sage_traces) == 1
    assert sage_traces[0].judgment_id == "j1"

    analyst_traces = await tracer.traces_by_component("ANALYST")
    assert len(analyst_traces) == 1
    assert analyst_traces[0].judgment_id == "j2"


@pytest.mark.asyncio
async def test_rolling_cap_behavior(tracer):
    """Test that buffer respects F(10)=55 cap."""
    # Create more than cap
    for i in range(TRACE_CAP + 10):
        tid = await tracer.start_trace(judgment_id=f"j{i}")
        await tracer.close_trace(tid)

    # Should only have TRACE_CAP
    stats = await tracer.stats()
    assert stats["buffer_size"] == TRACE_CAP
    assert stats["total_traced"] == TRACE_CAP + 10


@pytest.mark.asyncio
async def test_stats_accuracy(tracer):
    """Test statistics are computed correctly."""
    verdicts = ["WAG", "WAG", "GROWL", "HOWL", "WAG"]

    for i, verdict in enumerate(verdicts):
        tid = await tracer.start_trace(judgment_id=f"j{i}")
        await tracer.add_node(
            tid, phase="JUDGE", component="SAGE",
            duration_ms=50.0, input_keys=[], input_sources=[],
            output_keys=[],
        )
        await tracer.close_trace(tid, final_verdict=verdict)

    stats = await tracer.stats()
    assert stats["total_traced"] == 5
    assert stats["by_verdict"]["WAG"] == 3
    assert stats["by_verdict"]["GROWL"] == 1
    assert stats["by_verdict"]["HOWL"] == 1
    assert stats["avg_duration_ms"] == 50.0


@pytest.mark.asyncio
async def test_depth_calculation(tracer):
    """Test DAG depth computation."""
    trace_id = await tracer.start_trace(judgment_id="j001")

    # Build linear chain: PERCEIVE -> JUDGE -> DECIDE -> ACT
    phases = ["JUDGE", "DECIDE", "ACT"]
    for phase in phases:
        await tracer.add_node(
            trace_id, phase=phase, component="TEST",
            duration_ms=10.0, input_keys=[], input_sources=[],
            output_keys=[],
        )

    await tracer.close_trace(trace_id)

    trace = await tracer.get_trace(trace_id)
    # Depth should be 3 (PERCEIVE is root, then 3 more nodes)
    assert trace.max_depth >= 1


@pytest.mark.asyncio
async def test_branching_factor(tracer):
    """Test branching factor computation."""
    trace_id = await tracer.start_trace(judgment_id="j001")

    # Single chain (branching = 1.0)
    await tracer.add_node(
        trace_id, phase="JUDGE", component="SAGE",
        duration_ms=10.0, input_keys=[], input_sources=[],
        output_keys=[],
    )
    await tracer.add_node(
        trace_id, phase="DECIDE", component="BRAIN",
        duration_ms=10.0, input_keys=[], input_sources=[],
        output_keys=[],
    )

    await tracer.close_trace(trace_id)

    trace = await tracer.get_trace(trace_id)
    assert trace.branching_factor >= 1.0


@pytest.mark.asyncio
async def test_error_tracking(tracer):
    """Test error detection in traces."""
    trace_id = await tracer.start_trace(judgment_id="j001")

    # Normal node
    await tracer.add_node(
        trace_id, phase="JUDGE", component="SAGE",
        duration_ms=10.0, input_keys=[], input_sources=[],
        output_keys=[],
    )

    await tracer.close_trace(trace_id)

    trace = await tracer.get_trace(trace_id)
    assert trace.has_errors is False
    assert trace.error_count == 0


@pytest.mark.asyncio
async def test_trace_to_dict(tracer):
    """Test trace serialization."""
    trace_id = await tracer.start_trace(judgment_id="j001")
    await tracer.add_node(
        trace_id, phase="JUDGE", component="SAGE",
        duration_ms=10.0, input_keys=["a"], input_sources=[],
        output_keys=["q"],
    )
    await tracer.close_trace(trace_id, final_verdict="WAG", final_q_score=75.0)

    trace = await tracer.get_trace(trace_id)
    d = trace.to_dict()

    assert d["judgment_id"] == "j001"
    assert d["final_verdict"] == "WAG"
    assert d["final_q_score"] == 75.0
    assert len(d["nodes"]) == 2


@pytest.mark.asyncio
async def test_snapshot_preserves_traces(tracer):
    """Test snapshot returns all traces."""
    for i in range(3):
        tid = await tracer.start_trace(judgment_id=f"j{i}")
        await tracer.close_trace(tid)

    snapshot = await tracer.snapshot()
    assert len(snapshot["traces"]) == 3
    assert snapshot["stats"]["total_traced"] == 3


@pytest.mark.asyncio
async def test_clear_tracer(tracer):
    """Test clearing all traces."""
    for i in range(5):
        tid = await tracer.start_trace(judgment_id=f"j{i}")
        await tracer.close_trace(tid)

    await tracer.clear()

    stats = await tracer.stats()
    assert stats["buffer_size"] == 0
    assert stats["total_traced"] == 0


@pytest.mark.asyncio
async def test_concurrent_trace_creation(tracer):
    """Test concurrent trace creation is thread-safe."""
    async def create_traces(count: int, offset: int) -> None:
        for i in range(count):
            tid = await tracer.start_trace(judgment_id=f"j_{offset}_{i}")
            await tracer.close_trace(tid, final_verdict="WAG")

    await asyncio.gather(
        create_traces(10, 0),
        create_traces(10, 1),
        create_traces(10, 2),
    )

    stats = await tracer.stats()
    assert stats["total_traced"] == 30
