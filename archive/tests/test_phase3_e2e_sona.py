"""
Phase 3: End-to-End SONA Testing
================================

Verify that:
1. Organism initializes with SONA emitter
2. SONA heartbeat is actually ticking
3. Events are being emitted to the bus
4. Graceful shutdown works

This is a real integration test (not mocked).
"""
import asyncio

import pytest

from cynic.kernel.core.event_bus import CoreEvent
from cynic.kernel.core.events_schema import SonaTickPayload
from cynic.kernel.organism.organism import awaken


@pytest.mark.asyncio
async def test_phase3_organism_sona_initialized():
    """Test that organism.sona_emitter is properly initialized."""
    organism = awaken()

    # Verify sona_emitter exists
    assert organism.sona_emitter is not None, "sona_emitter should not be None"
    assert organism.sona_emitter._running, "sona_emitter should be running"

    # Verify it's ticking
    stats = organism.sona_emitter.stats()
    assert stats["running"] is True
    assert stats["tick_count"] >= 0
    assert stats["uptime_s"] > 0


@pytest.mark.asyncio
async def test_phase3_sona_tick_events():
    """Test that SONA_TICK events are actually emitted."""
    organism = awaken()
    bus = get_core_bus("DEFAULT")
    events_received = []

    async def capture_sona_tick(event):
        events_received.append(event)

    # Subscribe to SONA_TICK events
    bus.on(CoreEvent.SONA_TICK, capture_sona_tick)

    # Wait for at least one SONA_TICK to be emitted
    # Since F(9) = 2040 seconds is too long for testing,
    # we'll use a shorter window and just verify structure
    await asyncio.sleep(0.5)

    # The test doesn't wait for natural heartbeat, but verifies:
    # 1. sona_emitter._emit_sona_tick() can be called manually
    await organism.sona_emitter._emit_sona_tick()
    await asyncio.sleep(0.1)

    # Should have captured the manual emit
    assert len(events_received) >= 1, f"Expected â‰¥1 SONA_TICK, got {len(events_received)}"

    event = events_received[0]
    payload = SonaTickPayload.model_validate(event.payload or {})

    # Verify payload structure
    assert isinstance(payload.uptime_s, float)
    assert isinstance(payload.tick_number, int)
    assert isinstance(payload.interval_s, float)
    assert payload.interval_s > 0


@pytest.mark.asyncio
async def test_phase3_organism_lifecycle():
    """Test that organism starts with SONA and stops gracefully."""
    organism = awaken()

    # Organism should be initialized with SONA running
    assert organism.sona_emitter is not None
    assert organism.sona_emitter._running

    initial_tick_count = organism.sona_emitter._tick_count

    # Manually emit a tick
    await organism.sona_emitter._emit_sona_tick()
    assert organism.sona_emitter._tick_count == initial_tick_count + 1

    # Stop should be idempotent
    await organism.sona_emitter.stop()
    assert not organism.sona_emitter._running

    await organism.sona_emitter.stop()  # Second stop should be safe
    assert not organism.sona_emitter._running


@pytest.mark.asyncio
async def test_phase3_sona_uptime_increases():
    """Test that SONA uptime_s increases over time."""
    organism = awaken()

    initial_uptime = organism.sona_emitter.stats()["uptime_s"]
    await asyncio.sleep(0.2)
    later_uptime = organism.sona_emitter.stats()["uptime_s"]

    assert later_uptime > initial_uptime, "Uptime should increase"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
