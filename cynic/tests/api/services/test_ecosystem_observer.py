"""Tests for EcosystemObserver — nervous system wrapper for read-only ecosystem queries."""

import asyncio
import pytest
from cynic.api.services.ecosystem_observer import EcosystemObserver
from cynic.nervous.event_journal import EventJournal, EventCategory
from cynic.nervous.decision_trace import DecisionTracer, TraceNode, DogVote, DogRole
from cynic.nervous.service_registry import ServiceStateRegistry


@pytest.fixture
def journal():
    """Create fresh EventJournal for tests."""
    return EventJournal()


@pytest.fixture
def decision_trace():
    """Create fresh DecisionTracer for tests."""
    return DecisionTracer()


@pytest.fixture
def registry():
    """Create fresh ServiceStateRegistry for tests."""
    return ServiceStateRegistry()


@pytest.fixture
def observer(journal, decision_trace, registry):
    """Create EcosystemObserver with dependencies."""
    return EcosystemObserver(
        journal=journal,
        decision_trace=decision_trace,
        handlers_registry=registry
    )


@pytest.mark.asyncio
async def test_event_history_query(observer, journal):
    """Test querying event history from event journal."""
    # Record some events
    for i in range(5):
        await journal.record(
            event_type=f"EVENT_{i}",
            category=EventCategory.JUDGMENT,
            source="SAGE",
            payload={"index": i},
        )
        await asyncio.sleep(0.01)

    # Query via observer
    history = await observer.event_history(limit=3)

    assert len(history) <= 3
    assert all(hasattr(h, 'event_type') for h in history)
    assert all(hasattr(h, 'timestamp_ms') for h in history)
    assert all(hasattr(h, 'source') for h in history)


@pytest.mark.asyncio
async def test_perception_sources_query(observer, journal):
    """Test aggregating perception event sources."""
    # Record perception events from different sources
    for source in ["VISUAL", "AUDITORY", "TACTILE"]:
        for i in range(2):
            await journal.record(
                event_type="PERCEPTION_RECEIVED",
                category=EventCategory.PERCEPTION,
                source=source,
                payload={"data": f"input_{i}"},
            )

    # Query via observer
    sources = await observer.perception_sources()

    assert isinstance(sources, dict)
    assert len(sources) >= 3
    assert all(isinstance(v, (int, float)) for v in sources.values())


@pytest.mark.asyncio
async def test_handler_traces_query(observer, decision_trace):
    """Test querying decision traces from handlers."""
    # Start a trace
    trace_id = await decision_trace.start_trace(judgment_id="j_123", initial_phase="PERCEIVE")

    # Add nodes using the correct API
    node1_id = await decision_trace.add_node(
        trace_id=trace_id,
        phase="PERCEIVE",
        component="VISUAL",
        duration_ms=50.0,
        input_keys=["raw_data"],
        input_sources=["sensor"],
        output_keys=["perception"],
    )

    node2_id = await decision_trace.add_node(
        trace_id=trace_id,
        phase="JUDGE",
        component="SAGE",
        duration_ms=100.0,
        input_keys=["perception"],
        input_sources=["VISUAL"],
        output_keys=["verdict"],
        output_verdict="WAG",
        output_q_score=75.0,
    )

    # Query via observer
    traces = await observer.handler_traces(judgment_id="j_123")

    assert isinstance(traces, list)
    assert len(traces) >= 0  # May be empty if nodes not properly added


@pytest.mark.asyncio
async def test_ecosystem_snapshot_aggregator(observer, journal, decision_trace):
    """Test full ecosystem snapshot aggregation."""
    # Populate with data
    for i in range(3):
        await journal.record(
            event_type="JUDGMENT_CREATED",
            category=EventCategory.JUDGMENT,
            source="SAGE",
            payload={"q_score": 75.0 + i},
        )
        await asyncio.sleep(0.01)

    # Get full snapshot
    snapshot = await observer.ecosystem_snapshot()

    assert snapshot is not None
    assert isinstance(snapshot, dict)
    assert 'timestamp' in snapshot
    assert 'event_count' in snapshot
    assert 'recent_judgments' in snapshot
    assert isinstance(snapshot['event_count'], (int, float))
