"""
Track D: SONA Wiring + Infrastructure Fixes
Test suite for SONA injection, re-entrancy elimination, and snapshot immutability.
"""
import asyncio
import pytest
from typing import Any, Dict, List

from cynic.core.event_bus import EventBus, CoreEvent, Event
from cynic.organism.sona_emitter import SonaEmitter
from cynic.learning.qlearning import LearningLoop
from cynic.learning.loops import SONA
from cynic.organism.state_manager import StateSnapshot, _FrozenDict
from cynic.core.events_schema import SonaTickPayload, LearningEventPayload


# ════════════════════════════════════════════════════════════════════════════
# TEST 1: SONA Emitter carries real QTable stats
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_sona_emitter_carries_real_qtable_stats():
    """
    SONA should emit real Q-table telemetry when QTable is injected.

    Scenario:
    1. Create SonaEmitter with fresh bus
    2. Subscribe to SONA_TICK, capture payload
    3. Call _emit_sona_tick() → all zeros (no injection)
    4. Inject mock QTable with stats
    5. Call _emit_sona_tick() again → assert telemetry is populated
    6. Inject mock orchestrator with _judgment_count
    7. Call _emit_sona_tick() again → assert judgment count populated
    """
    bus = EventBus(bus_id="test_sona_emitter_1")
    sona = SonaEmitter(bus=bus)

    captured_payloads: List[SonaTickPayload] = []

    async def capture_tick(event: Event) -> None:
        captured_payloads.append(event.as_typed(SonaTickPayload))

    # Subscribe to SONA_TICK
    bus.on(CoreEvent.SONA_TICK, capture_tick)
    await asyncio.sleep(0.01)  # Let subscription settle

    # Test 1a: Before injection, all zeros
    await sona._emit_sona_tick()
    await asyncio.sleep(0.01)
    assert len(captured_payloads) == 1
    assert captured_payloads[0].q_table_entries == 0
    assert captured_payloads[0].learning_rate == 0.0
    assert captured_payloads[0].ewc_consolidated == 0
    assert captured_payloads[0].total_judgments == 0

    # Test 1b: Inject mock QTable
    class MockQTable:
        def stats(self) -> Dict[str, Any]:
            return {
                "entries": 12,
                "learning_rate": 0.05,
                "ewc_consolidated": 3,
            }

    sona.set_qtable(MockQTable())
    await sona._emit_sona_tick()
    await asyncio.sleep(0.01)
    assert len(captured_payloads) == 2
    assert captured_payloads[1].q_table_entries == 12
    assert captured_payloads[1].learning_rate == 0.05
    assert captured_payloads[1].ewc_consolidated == 3
    assert captured_payloads[1].total_judgments == 0  # Still not injected

    # Test 1c: Inject mock orchestrator
    class MockOrchestrator:
        def __init__(self):
            self._judgment_count = 42

    sona.set_orchestrator(MockOrchestrator())
    await sona._emit_sona_tick()
    await asyncio.sleep(0.01)
    assert len(captured_payloads) == 3
    assert captured_payloads[2].q_table_entries == 12
    assert captured_payloads[2].total_judgments == 42


# ════════════════════════════════════════════════════════════════════════════
# TEST 2: No re-entrancy on LEARNING_EVENT → double Q-table update
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_sona_no_reentrance_on_learning_event():
    """
    SONA emits SONA_AGGREGATED, not LEARNING_EVENT.
    This prevents LearningLoop from re-triggering and double-updating Q-table.

    Scenario:
    1. Wire LearningLoop + SONA to fresh bus
    2. Fire 1 LEARNING_EVENT
    3. Wait for propagation
    4. Assert Q-table updated exactly once (not twice)
    """
    bus = EventBus(bus_id="test_no_reentrance")

    # Create mock QTable that tracks update count
    class TrackingQTable:
        def __init__(self):
            self.update_count = 0
            self._alpha = 0.01  # Learning rate for LearningLoop.__init__

        def update(self, *args, **kwargs):
            self.update_count += 1

        def stats(self):
            return {
                "entries": self.update_count,
                "learning_rate": self._alpha,
                "ewc_consolidated": 0,
            }

    qtable = TrackingQTable()

    # Create learning loop with mock QTable
    loop = LearningLoop(qtable=qtable)
    loop.start(bus)
    await asyncio.sleep(0.01)

    # Create initial learning payload and emit
    initial_event = Event.typed(
        CoreEvent.LEARNING_EVENT,
        LearningEventPayload(
            reward=0.8,
            action="WAG",
            state_key="test_state",
            judgment_id="judge_1",
            loop_name="TEST_LOOP",
        ),
        source="test",
    )
    await bus.emit(initial_event)

    # Wait for event propagation
    await asyncio.sleep(0.1)

    # Assert only 1 update (not 2 from re-entrancy)
    assert qtable.update_count == 1, f"Expected 1 update, got {qtable.update_count}"

    # Clean up
    loop.stop()


# ════════════════════════════════════════════════════════════════════════════
# TEST 3: SONA_AGGREGATED event is emitted (no LEARNING_EVENT re-emission)
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_sona_aggregated_event_emitted():
    """
    SONA._aggregate_learning() emits SONA_AGGREGATED, not LEARNING_EVENT.

    Scenario:
    1. Subscribe to all events (wildcard)
    2. Wire SONA to bus
    3. Fire 1 LEARNING_EVENT
    4. Wait for propagation
    5. Assert SONA_AGGREGATED in event types
    6. Assert LEARNING_EVENT count == 1 (original only, not re-emitted)
    """
    bus = EventBus(bus_id="test_aggregated_event")

    emitted_types: List[str] = []

    async def capture_all(event: Event) -> None:
        emitted_types.append(event.type)

    # Subscribe to all event types
    bus.on("*", capture_all)
    await asyncio.sleep(0.01)

    # Wire SONA
    sona = SONA(event_bus=bus)
    sona.start(bus)
    await asyncio.sleep(0.01)

    # Fire initial LEARNING_EVENT
    event = Event.typed(
        CoreEvent.LEARNING_EVENT,
        LearningEventPayload(
            reward=0.5,
            action="WAG",
            state_key="state1",
            judgment_id="j1",
            loop_name="TEST",
        ),
        source="test",
    )
    await bus.emit(event)

    # Wait for propagation and aggregation
    await asyncio.sleep(0.1)

    # Verify event types
    assert CoreEvent.SONA_AGGREGATED in emitted_types, \
        f"SONA_AGGREGATED not in emitted events: {emitted_types}"

    # Count LEARNING_EVENT emissions (should be exactly 1 — the original)
    learning_event_count = emitted_types.count(CoreEvent.LEARNING_EVENT)
    assert learning_event_count == 1, \
        f"Expected 1 LEARNING_EVENT, got {learning_event_count}. Emitted: {emitted_types}"

    # Clean up
    sona.stop()


# ════════════════════════════════════════════════════════════════════════════
# TEST 4: StateSnapshot immutability via _FrozenDict
# ════════════════════════════════════════════════════════════════════════════

def test_snapshot_immutable():
    """StateSnapshot dicts are immutable — raise on write attempts."""
    snap = StateSnapshot(
        memory={"key": "value"},
        persistent={"persist_key": "persist_value"},
        checkpoint={"checkpoint_key": "checkpoint_value"},
    )

    # Verify all three layers are _FrozenDict
    assert isinstance(snap.memory, _FrozenDict)
    assert isinstance(snap.persistent, _FrozenDict)
    assert isinstance(snap.checkpoint, _FrozenDict)

    # Test read access (should work)
    assert snap.memory["key"] == "value"
    assert snap.persistent["persist_key"] == "persist_value"
    assert snap.checkpoint["checkpoint_key"] == "checkpoint_value"

    # Test write attempts (should raise AttributeError)
    with pytest.raises(AttributeError, match="StateSnapshot is immutable"):
        snap.memory["new_key"] = "new_value"

    with pytest.raises(AttributeError, match="StateSnapshot is immutable"):
        snap.persistent["new_key"] = "new_value"

    with pytest.raises(AttributeError, match="StateSnapshot is immutable"):
        snap.checkpoint["new_key"] = "new_value"

    # Test other mutation methods
    with pytest.raises(AttributeError, match="StateSnapshot is immutable"):
        del snap.memory["key"]

    with pytest.raises(AttributeError, match="StateSnapshot is immutable"):
        snap.memory.update({"key2": "value2"})

    with pytest.raises(AttributeError, match="StateSnapshot is immutable"):
        snap.memory.pop("key")

    with pytest.raises(AttributeError, match="StateSnapshot is immutable"):
        snap.memory.clear()


def test_snapshot_returns_copies():
    """
    StateSnapshot should be independent of the original dicts.
    Mutations to the original should not affect the snapshot.
    """
    original = {"key": "original"}

    snap = StateSnapshot(
        memory=original,
        persistent={},
        checkpoint={},
    )

    # Verify snapshot captured the value
    assert snap.memory["key"] == "original"

    # Mutate the original dict (outside the snapshot)
    original["key"] = "mutated"
    original["new_key"] = "new_value"

    # Snapshot should be unaffected
    assert snap.memory["key"] == "original"
    assert "new_key" not in snap.memory
    assert len(snap.memory) == 1
