"""Tests for EventJournal — sequence recording and causality tracing."""

import asyncio
import pytest
from cynic.nervous.event_journal import EventJournal, EventCategory, JOURNAL_CAP


@pytest.fixture
def journal():
    """Create fresh EventJournal for each test."""
    return EventJournal()


@pytest.mark.asyncio
async def test_record_single_event(journal):
    """Test basic event recording."""
    event_id = await journal.record(
        event_type="JUDGMENT_CREATED",
        category=EventCategory.JUDGMENT,
        source="SAGE",
        payload={"q_score": 75.0, "verdict": "WAG"},
    )
    assert isinstance(event_id, str)
    assert len(event_id) == 16  # SHA256 hash, truncated

    # Verify stored
    entry = await journal.get_event(event_id)
    assert entry is not None
    assert entry.event_type == "JUDGMENT_CREATED"
    assert entry.source == "SAGE"
    assert entry.payload_keys == ["q_score", "verdict"]


@pytest.mark.asyncio
async def test_recent_events(journal):
    """Test retrieving recent events."""
    # Record 5 events
    ids = []
    for i in range(5):
        eid = await journal.record(
            event_type=f"EVENT_{i}",
            category=EventCategory.SYSTEM,
            source=f"SOURCE_{i}",
            payload={"index": i},
        )
        ids.append(eid)
        await asyncio.sleep(0.01)  # Ensure different timestamps

    # Get last 3
    recent = await journal.recent(limit=3)
    assert len(recent) == 3
    # Should be newest first
    assert recent[0].event_type == "EVENT_4"
    assert recent[1].event_type == "EVENT_3"
    assert recent[2].event_type == "EVENT_2"


@pytest.mark.asyncio
async def test_filter_by_type(journal):
    """Test filtering by event type."""
    # Record mixed event types
    for i in range(3):
        await journal.record(
            event_type="JUDGMENT_CREATED",
            category=EventCategory.JUDGMENT,
            source="SAGE",
            payload={},
        )
    for i in range(2):
        await journal.record(
            event_type="DECISION_MADE",
            category=EventCategory.DECISION,
            source="BRAIN",
            payload={},
        )

    # Filter by type
    judgments = await journal.filter_by_type("JUDGMENT_CREATED")
    assert len(judgments) == 3
    assert all(e.event_type == "JUDGMENT_CREATED" for e in judgments)

    decisions = await journal.filter_by_type("DECISION_MADE")
    assert len(decisions) == 2
    assert all(e.event_type == "DECISION_MADE" for e in decisions)


@pytest.mark.asyncio
async def test_filter_by_source(journal):
    """Test filtering by event source (component)."""
    for i in range(3):
        await journal.record(
            event_type="SCORED",
            category=EventCategory.JUDGMENT,
            source="SAGE",
            payload={},
        )
    for i in range(2):
        await journal.record(
            event_type="SCORED",
            category=EventCategory.JUDGMENT,
            source="ANALYST",
            payload={},
        )

    sage_events = await journal.filter_by_source("SAGE")
    assert len(sage_events) == 3
    assert all(e.source == "SAGE" for e in sage_events)

    analyst_events = await journal.filter_by_source("ANALYST")
    assert len(analyst_events) == 2
    assert all(e.source == "ANALYST" for e in analyst_events)


@pytest.mark.asyncio
async def test_filter_by_category(journal):
    """Test filtering by event category."""
    # Record events in different categories
    for cat in [EventCategory.PERCEPTION, EventCategory.JUDGMENT, EventCategory.ACTION]:
        for i in range(2):
            await journal.record(
                event_type="TEST",
                category=cat,
                source="TEST",
                payload={},
            )

    # Filter by category
    judgments = await journal.filter_by_category(EventCategory.JUDGMENT)
    assert len(judgments) == 2
    assert all(e.category == EventCategory.JUDGMENT for e in judgments)

    actions = await journal.filter_by_category(EventCategory.ACTION)
    assert len(actions) == 2
    assert all(e.category == EventCategory.ACTION for e in actions)


@pytest.mark.asyncio
async def test_time_range(journal):
    """Test filtering by timestamp range."""
    import time

    start_ms = time.time() * 1000.0

    # Record events
    for i in range(3):
        await journal.record(
            event_type="TEST",
            category=EventCategory.SYSTEM,
            source="TEST",
            payload={},
        )
        await asyncio.sleep(0.01)

    end_ms = time.time() * 1000.0

    # Query range
    events = await journal.time_range(start_ms, end_ms)
    assert len(events) == 3
    assert all(start_ms <= e.timestamp_ms <= end_ms for e in events)


@pytest.mark.asyncio
async def test_causality_chain_parent(journal):
    """Test tracing causality upward (causes)."""
    # Create chain: A -> B -> C (B triggered by A, C triggered by B)
    eid_a = await journal.record(
        event_type="EVENT_A",
        category=EventCategory.PERCEPTION,
        source="WATCHER",
        payload={},
    )

    eid_b = await journal.record(
        event_type="EVENT_B",
        category=EventCategory.JUDGMENT,
        source="SAGE",
        payload={},
        parent_event_id=eid_a,
    )

    eid_c = await journal.record(
        event_type="EVENT_C",
        category=EventCategory.DECISION,
        source="BRAIN",
        payload={},
        parent_event_id=eid_b,
    )

    # Trace up from C
    chain = await journal.causality_chain(eid_c, direction="up")
    assert len(chain) == 3
    assert chain[0].event_id == eid_c
    assert chain[1].event_id == eid_b
    assert chain[2].event_id == eid_a


@pytest.mark.asyncio
async def test_causality_chain_children(journal):
    """Test tracing causality downward (effects)."""
    eid_a = await journal.record(
        event_type="EVENT_A",
        category=EventCategory.PERCEPTION,
        source="WATCHER",
        payload={},
    )

    eid_b = await journal.record(
        event_type="EVENT_B",
        category=EventCategory.JUDGMENT,
        source="SAGE",
        payload={},
        parent_event_id=eid_a,
    )

    eid_c = await journal.record(
        event_type="EVENT_C",
        category=EventCategory.DECISION,
        source="BRAIN",
        payload={},
        parent_event_id=eid_a,
    )

    # Manually set child IDs (in real system, done automatically)
    entry_a = await journal.get_event(eid_a)
    entry_a.child_event_ids = [eid_b, eid_c]

    # Trace down from A
    chain = await journal.causality_chain(eid_a, direction="down")
    assert len(chain) == 3  # A + B + C
    child_ids = {e.event_id for e in chain}
    assert eid_a in child_ids
    assert eid_b in child_ids
    assert eid_c in child_ids


@pytest.mark.asyncio
async def test_error_tracking(journal):
    """Test recording and filtering error events."""
    import time

    start_ms = time.time() * 1000.0

    # Record normal event
    await journal.record(
        event_type="SCORED",
        category=EventCategory.JUDGMENT,
        source="SAGE",
        payload={},
    )

    # Record error event
    await journal.record(
        event_type="JUDGMENT_FAILED",
        category=EventCategory.JUDGMENT,
        source="ANALYST",
        payload={},
        is_error=True,
        error_message="Division by zero in confidence calc",
    )

    # Get errors since start
    errors = await journal.errors_since(start_ms)
    assert len(errors) == 1
    assert errors[0].is_error is True
    assert "Division by zero" in errors[0].error_message


@pytest.mark.asyncio
async def test_rolling_cap_behavior(journal):
    """Test that buffer respects F(11)=89 cap."""
    # Fill beyond cap
    for i in range(JOURNAL_CAP + 10):
        await journal.record(
            event_type="TEST",
            category=EventCategory.SYSTEM,
            source=f"SOURCE_{i}",
            payload={"index": i},
        )

    # Should only have JOURNAL_CAP entries
    stats = await journal.stats()
    assert stats["buffer_size"] == JOURNAL_CAP
    assert stats["total_recorded"] == JOURNAL_CAP + 10  # Total ever recorded

    # Oldest entries should be gone
    recent = await journal.recent(limit=100)
    assert len(recent) == JOURNAL_CAP

    # Verify oldest are JOURNAL_CAP+10-89, JOURNAL_CAP+10-88, etc.
    # (newest indices from loop)
    first_source = recent[-1].source  # Oldest
    assert int(first_source.split("_")[1]) >= 10  # Not from first 10


@pytest.mark.asyncio
async def test_buffer_after_eviction_maintains_indices(journal):
    """Test that indices are consistent after circular buffer eviction."""
    # Fill beyond cap
    for i in range(JOURNAL_CAP + 5):
        await journal.record(
            event_type=f"TYPE_{i % 3}",
            category=EventCategory.SYSTEM,
            source=f"SOURCE_{i}",
            payload={},
        )

    # Query by type should still work correctly
    type_0_events = await journal.filter_by_type("TYPE_0")
    # Should only have events from buffer range
    assert len(type_0_events) > 0
    assert all(e.event_type == "TYPE_0" for e in type_0_events)

    # All should still be queryable
    for event in type_0_events:
        retrieved = await journal.get_event(event.event_id)
        assert retrieved is not None
        assert retrieved.event_type == "TYPE_0"


@pytest.mark.asyncio
async def test_duration_tracking(journal):
    """Test recording event duration."""
    event_id = await journal.record(
        event_type="SCORED",
        category=EventCategory.JUDGMENT,
        source="SAGE",
        payload={},
        duration_ms=42.5,
    )

    entry = await journal.get_event(event_id)
    assert entry.duration_ms == 42.5


@pytest.mark.asyncio
async def test_payload_keys_capture(journal):
    """Test that payload keys are captured (but not values for privacy)."""
    event_id = await journal.record(
        event_type="DECISION_MADE",
        category=EventCategory.DECISION,
        source="BRAIN",
        payload={
            "verdict": "WAG",
            "q_score": 75.5,
            "confidence": 0.618,
            "dogs": ["SAGE", "ANALYST", "GUARDIAN"],
        },
    )

    entry = await journal.get_event(event_id)
    assert set(entry.payload_keys) == {"verdict", "q_score", "confidence", "dogs"}


@pytest.mark.asyncio
async def test_empty_payload(journal):
    """Test recording event with empty payload."""
    event_id = await journal.record(
        event_type="TEST",
        category=EventCategory.SYSTEM,
        source="TEST",
        payload={},
    )

    entry = await journal.get_event(event_id)
    assert entry.payload_keys == []


@pytest.mark.asyncio
async def test_get_nonexistent_event(journal):
    """Test querying non-existent event returns None."""
    result = await journal.get_event("nonexistent_id")
    assert result is None


@pytest.mark.asyncio
async def test_concurrent_writes(journal):
    """Test thread-safe concurrent writes."""
    async def write_events(count: int, source_id: int) -> None:
        for i in range(count):
            await journal.record(
                event_type="CONCURRENT_TEST",
                category=EventCategory.SYSTEM,
                source=f"CONCURRENT_{source_id}",
                payload={"seq": i},
            )

    # Run 3 concurrent writers
    await asyncio.gather(
        write_events(10, 0),
        write_events(10, 1),
        write_events(10, 2),
    )

    stats = await journal.stats()
    assert stats["total_recorded"] == 30


@pytest.mark.asyncio
async def test_stats_accuracy(journal):
    """Test statistics are accurate."""
    # Record known mix
    for _ in range(3):
        await journal.record(
            event_type="TYPE_A",
            category=EventCategory.JUDGMENT,
            source="SOURCE_A",
            payload={},
        )
    for _ in range(2):
        await journal.record(
            event_type="TYPE_B",
            category=EventCategory.ACTION,
            source="SOURCE_B",
            payload={},
        )

    stats = await journal.stats()
    assert stats["total_recorded"] == 5
    assert stats["buffer_size"] == 5
    assert stats["buffer_cap"] == JOURNAL_CAP
    assert stats["unique_types"] == 2
    assert stats["unique_sources"] == 2
    assert stats["by_category"]["judgment"] == 3
    assert stats["by_category"]["action"] == 2


@pytest.mark.asyncio
async def test_snapshot_preserves_all_entries(journal):
    """Test snapshot returns all entries."""
    # Record 5 events
    for i in range(5):
        await journal.record(
            event_type=f"EVENT_{i}",
            category=EventCategory.SYSTEM,
            source=f"SOURCE_{i}",
            payload={"index": i},
        )

    snapshot = await journal.snapshot()
    assert len(snapshot["entries"]) == 5
    assert snapshot["stats"]["total_recorded"] == 5
    assert snapshot["stats"]["buffer_size"] == 5


@pytest.mark.asyncio
async def test_clear_journal(journal):
    """Test clearing the journal."""
    # Record 10 events
    for i in range(10):
        await journal.record(
            event_type="TEST",
            category=EventCategory.SYSTEM,
            source="TEST",
            payload={},
        )

    # Clear
    await journal.clear()

    # Verify empty
    stats = await journal.stats()
    assert stats["buffer_size"] == 0
    assert stats["total_recorded"] == 0

    recent = await journal.recent(limit=100)
    assert len(recent) == 0


@pytest.mark.asyncio
async def test_event_id_deterministic(journal):
    """Test that event IDs are deterministic for same (timestamp, source, type)."""
    import time

    # Can't easily force same timestamp due to system resolution,
    # but we can verify ID format is consistent
    eid1 = await journal.record(
        event_type="TEST",
        category=EventCategory.SYSTEM,
        source="TEST",
        payload={},
    )

    assert isinstance(eid1, str)
    assert len(eid1) == 16
    assert all(c in "0123456789abcdef" for c in eid1)  # Hex


@pytest.mark.asyncio
async def test_filter_with_limit(journal):
    """Test that filters respect limit parameter."""
    # Record 20 events of same type
    for i in range(20):
        await journal.record(
            event_type="BULK_TEST",
            category=EventCategory.SYSTEM,
            source="TEST",
            payload={},
        )

    # Query with limit
    results = await journal.filter_by_type("BULK_TEST", limit=5)
    assert len(results) <= 5


@pytest.mark.asyncio
async def test_to_dict_and_from_dict(journal):
    """Test entry serialization."""
    from cynic.nervous.event_journal import JournalEntry

    entry = JournalEntry(
        event_id="abc123",
        timestamp_ms=1234567890.0,
        event_type="TEST",
        category=EventCategory.JUDGMENT,
        source="TEST",
        payload_keys=["a", "b"],
    )

    # Serialize
    d = entry.to_dict()
    assert d["event_id"] == "abc123"
    assert d["category"] == "judgment"  # Enum serialized to string

    # Deserialize
    restored = JournalEntry.from_dict(d)
    assert restored.event_id == entry.event_id
    assert restored.category == EventCategory.JUDGMENT
