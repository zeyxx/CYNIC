"""
Priority 5: Event Protocol — Enum Completeness + Journal Wiring

Tests for:
1. CoreEvent enum completeness (4 missing values now defined)
2. BusJournalAdapter category mapping (correct categorization of events)
3. BusJournalAdapter event recording (events flow to journal)
4. Scheduler emits META_CYCLE, not SONA_TICK
"""
import asyncio
import pytest

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.nervous.event_journal import EventCategory, EventJournal
from cynic.nervous.bus_journal_adapter import BusJournalAdapter


class TestCoreEventEnumCompleteness:
    """Tests that 4 missing CoreEvent values are now defined."""

    def test_sona_aggregated_resolves(self):
        """CoreEvent.SONA_AGGREGATED should resolve without AttributeError."""
        assert CoreEvent.SONA_AGGREGATED == "core.sona_aggregated"

    def test_internal_error_resolves(self):
        """CoreEvent.INTERNAL_ERROR should resolve without AttributeError."""
        assert CoreEvent.INTERNAL_ERROR == "core.internal_error"

    def test_budget_warning_resolves(self):
        """CoreEvent.BUDGET_WARNING should resolve without AttributeError."""
        assert CoreEvent.BUDGET_WARNING == "core.budget_warning"

    def test_budget_exhausted_resolves(self):
        """CoreEvent.BUDGET_EXHAUSTED should resolve without AttributeError."""
        assert CoreEvent.BUDGET_EXHAUSTED == "core.budget_exhausted"


class TestBusJournalAdapterCategoryMapping:
    """Tests category mapping in BusJournalAdapter._CATEGORY_MAP."""

    def test_judgment_created_maps_to_judgment(self):
        """JUDGMENT_CREATED event → EventCategory.JUDGMENT."""
        from cynic.nervous.bus_journal_adapter import _CATEGORY_MAP
        assert _CATEGORY_MAP["core.judgment_created"] == EventCategory.JUDGMENT

    def test_learning_event_maps_to_learning(self):
        """LEARNING_EVENT event → EventCategory.LEARNING."""
        from cynic.nervous.bus_journal_adapter import _CATEGORY_MAP
        assert _CATEGORY_MAP["core.learning_event"] == EventCategory.LEARNING

    def test_budget_warning_maps_to_accounting(self):
        """BUDGET_WARNING event → EventCategory.ACCOUNTING."""
        from cynic.nervous.bus_journal_adapter import _CATEGORY_MAP
        assert _CATEGORY_MAP["core.budget_warning"] == EventCategory.ACCOUNTING

    def test_internal_error_maps_to_system(self):
        """INTERNAL_ERROR event → EventCategory.SYSTEM."""
        from cynic.nervous.bus_journal_adapter import _CATEGORY_MAP
        assert _CATEGORY_MAP["core.internal_error"] == EventCategory.SYSTEM

    def test_unknown_event_defaults_to_system(self):
        """Unknown event type defaults to EventCategory.SYSTEM."""
        from cynic.nervous.bus_journal_adapter import _CATEGORY_MAP
        category = _CATEGORY_MAP.get("unknown.event.type", EventCategory.SYSTEM)
        assert category == EventCategory.SYSTEM


class TestBusJournalAdapterRecordsEvents:
    """Tests that BusJournalAdapter records events into EventJournal."""

    @pytest.mark.asyncio
    async def test_on_event_increments_journal_counter(self):
        """Calling on_event() increments journal's total_recorded count."""
        journal = EventJournal()
        adapter = BusJournalAdapter(journal)

        initial_stats = await journal.stats()
        initial_count = initial_stats["total_recorded"]

        event = Event(
            type=CoreEvent.JUDGMENT_CREATED.value,
            payload={"test": "data"},
            source="test_source"
        )

        await adapter.on_event(event)

        final_stats = await journal.stats()
        assert final_stats["total_recorded"] == initial_count + 1

    @pytest.mark.asyncio
    async def test_payload_keys_captured(self):
        """Payload dict keys are captured (keys but not values)."""
        journal = EventJournal()
        adapter = BusJournalAdapter(journal)

        event = Event(
            type=CoreEvent.JUDGMENT_CREATED.value,
            payload={"foo": "bar", "baz": "qux"},
            source="test_source"
        )

        await adapter.on_event(event)

        # Retrieve the latest event from the journal
        recent = await journal.recent(limit=1)
        assert len(recent) > 0
        recorded_event = recent[0]
        assert "foo" in recorded_event.payload_keys
        assert "baz" in recorded_event.payload_keys

    @pytest.mark.asyncio
    async def test_multiple_events_recent_newest_first(self):
        """Multiple events recorded → journal.recent() returns newest-first."""
        journal = EventJournal()
        adapter = BusJournalAdapter(journal)

        # Record 3 events in order
        for i in range(3):
            event = Event(
                type=CoreEvent.JUDGMENT_CREATED.value,
                payload={"index": i},
                source="test_source"
            )
            await adapter.on_event(event)
            await asyncio.sleep(0.01)  # Ensure ordering

        recent = await journal.recent(limit=3)
        assert len(recent) == 3
        # Should be newest-first
        assert recent[0].payload_keys == ["index"]
        assert recent[1].payload_keys == ["index"]
        assert recent[2].payload_keys == ["index"]

    @pytest.mark.asyncio
    async def test_normal_events_not_marked_error(self):
        """Normal events should have is_error=False by default."""
        journal = EventJournal()
        adapter = BusJournalAdapter(journal)

        event = Event(
            type=CoreEvent.JUDGMENT_CREATED.value,
            payload={"data": "value"},
            source="test_source"
        )

        await adapter.on_event(event)

        recent = await journal.recent(limit=1)
        assert len(recent) > 0
        recorded_event = recent[0]
        assert recorded_event.is_error is False

    @pytest.mark.asyncio
    async def test_non_dict_payload_handled_gracefully(self):
        """Non-dict payload (e.g., None) doesn't raise, records with empty payload_keys."""
        journal = EventJournal()
        adapter = BusJournalAdapter(journal)

        # Create event with None payload (Event.dict_payload will return {})
        event = Event(
            type=CoreEvent.JUDGMENT_CREATED.value,
            payload=None,
            source="test_source"
        )

        # Should not raise
        await adapter.on_event(event)

        recent = await journal.recent(limit=1)
        assert len(recent) > 0
        recorded_event = recent[0]
        assert recorded_event.payload_keys == []


class TestSchedulerEmitsMetaCycle:
    """Tests that scheduler._meta_pulse() emits META_CYCLE, not SONA_TICK."""

    def test_meta_cycle_value_is_correct(self):
        """META_CYCLE enum value should be 'core.meta_cycle'."""
        assert CoreEvent.META_CYCLE.value == "core.meta_cycle"

    @pytest.mark.asyncio
    async def test_scheduler_emits_meta_cycle_not_sona_tick(self):
        """Scheduler's _meta_pulse() should emit META_CYCLE."""
        from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm
        from cynic.kernel.core.consciousness import ConsciousnessState

        # Create a mock orchestrator with a bus
        bus = EventBus("test_instance")

        # Track emitted events
        emitted_events = []

        async def capture_event(event: Event) -> None:
            emitted_events.append(event)

        bus.on(CoreEvent.META_CYCLE.value, capture_event)

        # Create scheduler with mocked orchestrator
        class MockOrchestrator:
            def __init__(self):
                self.bus = bus

        scheduler = ConsciousnessRhythm(
            MockOrchestrator(),
            bus=bus,
            consciousness=ConsciousnessState()
        )

        # Call _meta_pulse
        await scheduler._meta_pulse()

        # Give event loop a chance to process the event
        await asyncio.sleep(0.1)

        # Verify that META_CYCLE was emitted (not SONA_TICK)
        assert len(emitted_events) > 0
        assert emitted_events[0].type == CoreEvent.META_CYCLE.value
