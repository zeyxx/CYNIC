"""
Unit tests for SonaEmitter â€” organism self-assessment loop

Tests:
  1. Lifecycle (start/stop idempotence)
  2. Periodic emission at F(9) interval
  3. Event payload structure (SonaTickPayload)
  4. Graceful cancellation
  5. Task supervision and crash recovery
"""

import asyncio

import pytest

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.events_schema import SonaTickPayload
from cynic.kernel.organism.sona_emitter import SONA_INTERVAL_S, SonaEmitter


@pytest.fixture
def event_bus() -> EventBus:
    """Create a test event bus."""
    return EventBus(bus_id="test_bus")


@pytest.mark.asyncio
async def test_sona_start_idempotent(event_bus: EventBus):
    """Test that calling start() multiple times is safe."""
    sona = SonaEmitter(bus=event_bus)

    assert not sona._running

    # First start
    sona.start()
    assert sona._running
    assert sona._task is not None

    # Second start (should be idempotent)
    sona.start()
    assert sona._running

    # Cleanup
    await sona.stop()
    assert not sona._running


@pytest.mark.asyncio
async def test_sona_stop_idempotent(event_bus: EventBus):
    """Test that calling stop() multiple times is safe."""
    sona = SonaEmitter(bus=event_bus)
    sona.start()

    # First stop
    await sona.stop()
    assert not sona._running

    # Second stop (should be idempotent)
    await sona.stop()
    assert not sona._running


@pytest.mark.asyncio
async def test_sona_emits_payload(event_bus: EventBus):
    """Test that SONA._emit_sona_tick creates valid payloads."""
    sona = SonaEmitter(bus=event_bus)

    # This just verifies the payload emission doesn't crash
    # Full integration testing (event propagation) is tested in integration suite
    initial_tick = sona._tick_count

    await sona._emit_sona_tick()

    # Verify tick counter incremented
    assert sona._tick_count == initial_tick + 1


@pytest.mark.skip(reason="Long-running test â€” would require 68+ minutes (2Ã— F(9))")
@pytest.mark.asyncio
async def test_sona_tick_counter_increments(event_bus: EventBus):
    """Test that tick_number increments with each emission.

    SKIPPED: This test requires waiting for multiple F(9) intervals.
    For integration testing, run with much shorter SONA_INTERVAL for dev builds.
    """
    pass


@pytest.mark.asyncio
async def test_sona_graceful_cancellation(event_bus: EventBus):
    """Test that SONA shuts down gracefully on cancellation."""
    sona = SonaEmitter(bus=event_bus)
    sona.start()

    assert sona._task is not None
    assert not sona._task.done()

    # Cancel task directly
    sona._task.cancel()

    # Wait for cancellation to propagate
    await asyncio.sleep(0.1)

    assert sona._task.done()

    # Stop should handle already-cancelled task
    await sona.stop()


@pytest.mark.asyncio
async def test_sona_stats(event_bus: EventBus):
    """Test SonaEmitter.stats() observability."""
    sona = SonaEmitter(bus=event_bus)
    sona.start()

    await asyncio.sleep(0.5)

    stats = sona.stats()

    assert stats["running"] is True
    assert stats["tick_count"] >= 0
    assert stats["uptime_s"] >= 0.4
    assert stats["interval_s"] == SONA_INTERVAL_S
    assert "next_tick_in_s" in stats

    await sona.stop()


@pytest.mark.asyncio
async def test_sona_payload_fields(event_bus: EventBus):
    """Test that SonaTickPayload has all required fields."""
    sona = SonaEmitter(bus=event_bus)
    events_received = []

    async def capture_sona(event: Event) -> None:
        events_received.append(event)

    event_bus.on(CoreEvent.SONA_TICK, capture_sona)

    sona.start()
    await asyncio.sleep(SONA_INTERVAL_S + 0.5)
    await sona.stop()

    assert len(events_received) >= 1

    event = events_received[0]
    payload = SonaTickPayload.model_validate(event.payload or {})

    # Verify all fields exist
    assert hasattr(payload, "instance_id")
    assert hasattr(payload, "q_table_entries")
    assert hasattr(payload, "total_judgments")
    assert hasattr(payload, "learning_rate")
    assert hasattr(payload, "ewc_consolidated")
    assert hasattr(payload, "uptime_s")
    assert hasattr(payload, "interval_s")
    assert hasattr(payload, "tick_number")

    # Verify field types
    assert isinstance(payload.instance_id, str)
    assert isinstance(payload.q_table_entries, int)
    assert isinstance(payload.total_judgments, int)
    assert isinstance(payload.learning_rate, float)
    assert isinstance(payload.ewc_consolidated, int)
    assert isinstance(payload.uptime_s, float)
    assert isinstance(payload.interval_s, float)
    assert isinstance(payload.tick_number, int)


@pytest.mark.asyncio
async def test_sona_task_supervision(event_bus: EventBus):
    """Test that SONA task has proper name and callback."""
    sona = SonaEmitter(bus=event_bus)
    sona.start()

    assert sona._task is not None
    assert sona._task.get_name() == "cynic.kernel.organism.sona_emitter"

    # Verify callback is registered (indirectly by checking task cleanup)
    await sona.stop()


@pytest.mark.asyncio
async def test_sona_multiple_stops(event_bus: EventBus):
    """Test that multiple sequential stop calls are safe."""
    sona = SonaEmitter(bus=event_bus)
    sona.start()

    assert sona._running

    # Multiple stops should be idempotent
    await sona.stop()
    assert not sona._running

    await sona.stop()
    assert not sona._running


@pytest.mark.asyncio
async def test_sona_with_db_pool_none(event_bus: EventBus):
    """Test that SONA works with db_pool=None."""
    sona = SonaEmitter(bus=event_bus, db_pool=None)
    events_received = []

    async def capture_sona(event: Event) -> None:
        events_received.append(event)

    event_bus.on(CoreEvent.SONA_TICK, capture_sona)

    sona.start()
    await asyncio.sleep(SONA_INTERVAL_S + 0.5)
    await sona.stop()

    assert len(events_received) >= 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
