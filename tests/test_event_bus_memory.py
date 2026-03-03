"""
Test suite for EventBus memory management.

Verifies that event history buffers are bounded and properly cleanup.
This addresses the MEMORY_MANAGEMENT blue screen issue (Leak #2).
"""

import pytest

from cynic.kernel.core.event_bus import Event, EventBus
from cynic.kernel.core.phi import fibonacci


@pytest.fixture
def event_bus():
    """Create fresh EventBus for testing."""
    return EventBus(bus_id="TEST")


@pytest.mark.asyncio
async def test_event_history_bounded_at_fibonacci_10(event_bus):
    """Event history should be capped at F(10)=55, not 1000."""
    # Emit more events than the limit
    for i in range(100):
        event = Event(type="test.event", payload={"counter": i}, source="test")
        await event_bus.emit(event)

    # Should NOT exceed F(10)=55
    max_size = fibonacci(10)  # 55
    assert len(event_bus._history) <= max_size, (
        f"History size {len(event_bus._history)} exceeds " f"Fibonacci(10)={max_size}"
    )


@pytest.mark.asyncio
async def test_three_buses_total_memory_bounded(event_bus):
    """Combined 3 buses should not hold 3000 events in memory."""

    max_per_bus = fibonacci(10)  # 55

    # Create three buses
    buses = [EventBus(f"BUS_{i}") for i in range(3)]

    for bus_idx, bus in enumerate(buses):
        for i in range(100):
            event = Event(
                type=f"test.event.{bus_idx}",
                payload={"counter": i, "bus": bus_idx},
                source="test",
            )
            await bus.emit(event)

    # Each bus should be bounded
    for bus in buses:
        assert len(bus._history) <= max_per_bus, (
            f"{bus.bus_id} history size {len(bus._history)} exceeds "
            f"Fibonacci(10)={max_per_bus}"
        )

    # Total should be ~165 events max, not 3000
    total_events = sum(len(bus._history) for bus in buses)
    assert (
        total_events <= max_per_bus * 3
    ), f"Total events {total_events} exceeds {max_per_bus * 3}"


@pytest.mark.asyncio
async def test_event_history_fifo_order(event_bus):
    """Oldest events should be removed first (FIFO)."""
    # Emit events with identifiable IDs
    event_ids = []
    for i in range(fibonacci(10) + 20):  # Exceed buffer
        event = Event(type="test.event", payload={"index": i}, source="test")
        event_ids.append(event.event_id)
        await event_bus.emit(event)

    # First ~20 events should have been removed
    remaining_ids = {e.event_id for e in event_bus._history}

    # Oldest event_ids should NOT be in remaining history
    for old_id in event_ids[:20]:
        assert (
            old_id not in remaining_ids
        ), f"Old event {old_id} should have been removed"


@pytest.mark.asyncio
async def test_event_payload_not_growing_unbounded(event_bus):
    """Large event payloads should not accumulate in history."""
    # Create events with large payloads
    for i in range(fibonacci(10) + 10):
        large_payload = {
            "data": "x" * 10000,  # 10KB per event
            "index": i,
        }
        event = Event(type="test.event", payload=large_payload, source="test")
        await event_bus.emit(event)

    # History should be bounded
    assert len(event_bus._history) <= fibonacci(10)

    # Total memory should be reasonable
    import sys

    total_size = sum(
        sys.getsizeof(e) + sys.getsizeof(e.payload) for e in event_bus._history
    )

    # Max ~5MB for F(10)=55 events Ã— 10KB payloads
    assert (
        total_size < 5_000_000
    ), f"Event history memory {total_size} bytes exceeds reasonable limit"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
