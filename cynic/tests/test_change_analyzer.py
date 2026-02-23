"""Tests for ChangeAnalyzer — semantic analysis of code changes."""

import asyncio
import pytest
from unittest.mock import AsyncMock, MagicMock

from cynic.core.event_bus import Event, CoreEvent, get_core_bus, reset_all_buses
from cynic.core.topology.payloads import SourceChangedPayload
from cynic.core.topology.change_analyzer import ChangeAnalyzer
from cynic.core.events_schema import ChangeAnalyzedPayload


@pytest.fixture
def analyzer():
    """Create a fresh ChangeAnalyzer."""
    return ChangeAnalyzer()


@pytest.fixture
async def bus():
    """Get the core bus."""
    reset_all_buses()
    yield get_core_bus()
    reset_all_buses()


@pytest.mark.asyncio
async def test_analyzer_classifies_kernel_changes(analyzer):
    """Test that kernel files are classified as CRITICAL."""
    payload = SourceChangedPayload(
        category="handlers",
        files=["cynic/core/phi.py"],
        timestamp=1234567890.0,
    )
    event = Event.typed(CoreEvent.SOURCE_CHANGED, payload, source="test")

    await analyzer.on_source_changed(event)

    # Check the analysis
    recent = analyzer.recent_analyses(limit=1)
    assert len(recent) == 1
    assert recent[0]["subsystems"] == ["kernel"]
    assert recent[0]["impact_level"] == "CRITICAL"
    assert recent[0]["risk_estimate"] == 0.9
    assert recent[0]["suggested_action"] == "ALERT"


@pytest.mark.asyncio
async def test_analyzer_classifies_api_changes(analyzer):
    """Test that API files are classified as HIGH."""
    payload = SourceChangedPayload(
        category="handlers",
        files=["cynic/api/routers/core.py"],
        timestamp=1234567890.0,
    )
    event = Event.typed(CoreEvent.SOURCE_CHANGED, payload, source="test")

    await analyzer.on_source_changed(event)

    recent = analyzer.recent_analyses(limit=1)
    assert len(recent) == 1
    assert recent[0]["subsystems"] == ["api"]
    assert recent[0]["impact_level"] == "HIGH"


@pytest.mark.asyncio
async def test_analyzer_classifies_test_changes(analyzer):
    """Test that test files are classified as LOW."""
    payload = SourceChangedPayload(
        category="handlers",
        files=["tests/test_something.py"],
        timestamp=1234567890.0,
    )
    event = Event.typed(CoreEvent.SOURCE_CHANGED, payload, source="test")

    await analyzer.on_source_changed(event)

    recent = analyzer.recent_analyses(limit=1)
    assert len(recent) == 1
    assert recent[0]["subsystems"] == ["tests"]
    assert recent[0]["impact_level"] == "LOW"
    assert recent[0]["suggested_action"] == "MONITOR"


@pytest.mark.asyncio
async def test_analyzer_takes_max_impact_across_files(analyzer):
    """Test that analyzer takes the max impact level across multiple files."""
    payload = SourceChangedPayload(
        category="handlers",
        files=[
            "cynic/core/phi.py",  # CRITICAL
            "tests/test_something.py",  # LOW
            "cynic/api/routers/core.py",  # HIGH
        ],
        timestamp=1234567890.0,
    )
    event = Event.typed(CoreEvent.SOURCE_CHANGED, payload, source="test")

    await analyzer.on_source_changed(event)

    recent = analyzer.recent_analyses(limit=1)
    assert len(recent) == 1
    assert recent[0]["impact_level"] == "CRITICAL"  # max of CRITICAL, HIGH, LOW
    assert recent[0]["subsystems"] == ["api", "kernel", "tests"]  # sorted


@pytest.mark.asyncio
async def test_analyzer_emits_change_analyzed_event(analyzer, bus):
    """Test that ChangeAnalyzer emits CHANGE_ANALYZED event."""
    received_events = []

    async def handler(event: Event):
        received_events.append(event)

    bus.on(CoreEvent.CHANGE_ANALYZED, handler)

    payload = SourceChangedPayload(
        category="handlers",
        files=["cynic/core/phi.py"],
        timestamp=1234567890.0,
    )
    event = Event.typed(CoreEvent.SOURCE_CHANGED, payload, source="test")

    await analyzer.on_source_changed(event)

    # Give async task time to run
    await asyncio.sleep(0.1)

    # Check that CHANGE_ANALYZED event was emitted
    assert len(received_events) > 0
    emitted = received_events[0]
    assert emitted.type == CoreEvent.CHANGE_ANALYZED
    p = emitted.as_typed(ChangeAnalyzedPayload)
    assert p.impact_level == "CRITICAL"


@pytest.mark.asyncio
async def test_analyzer_ring_buffer_cap(analyzer):
    """Test that analyzer ring buffer respects F(8)=21 cap."""
    # Add more than 21 analyses
    for i in range(30):
        payload = SourceChangedPayload(
            category="handlers",
            files=[f"cynic/api/router{i}.py"],
            timestamp=1234567890.0 + i,
        )
        event = Event.typed(CoreEvent.SOURCE_CHANGED, payload, source="test")
        await analyzer.on_source_changed(event)

    # Check that buffer respects cap
    all_analyses = analyzer.recent_analyses(limit=100)
    assert len(all_analyses) == 21  # F(8) = 21

    # Check that latest analyses are retained (not oldest)
    timestamps = [a["timestamp"] for a in all_analyses]
    assert timestamps[-1] > timestamps[0]  # Last one has higher timestamp


@pytest.mark.asyncio
async def test_analyzer_stats(analyzer):
    """Test that analyzer provides correct statistics."""
    payload = SourceChangedPayload(
        category="handlers",
        files=["cynic/core/phi.py"],
        timestamp=1234567890.0,
    )
    event = Event.typed(CoreEvent.SOURCE_CHANGED, payload, source="test")

    await analyzer.on_source_changed(event)
    await analyzer.on_source_changed(event)
    await analyzer.on_source_changed(event)

    stats = analyzer.stats()
    assert stats["total_analyzed"] == 3
    assert stats["buffer_size"] == 3
    assert stats["buffer_cap"] == 21
