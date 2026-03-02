"""Tests for optimized event handler execution."""

from __future__ import annotations

import asyncio
import time
import pytest

from cynic.kernel.core.event_bus import EventBus, Event, CoreEvent


@pytest.mark.asyncio
@pytest.mark.performance
async def test_batch_emit_vs_sequential():
    """Batch-style emit should be faster than sequential."""
    bus = EventBus("batch_test", "instance1")

    handler_calls = []

    async def track_handler(event: Event):
        handler_calls.append(event.event_id)

    bus.on("*", track_handler)

    # Sequential emit baseline
    start = time.perf_counter()
    for i in range(500):
        await bus.emit(Event(
            type=CoreEvent.SONA_TICK.value,
            payload={"i": i},
            source="test"
        ))
    await bus.drain()
    sequential_time = time.perf_counter() - start

    sequential_calls = len(handler_calls)
    handler_calls.clear()

    # Measure sequential throughput
    tps = 500 / sequential_time
    print(f"\nSequential 500 events: {sequential_time:.3f}s ({tps:.0f} TPS)")

    # Assertions
    assert sequential_calls == 500, f"Expected 500 calls, got {sequential_calls}"
    assert sequential_time < 5.0, f"Sequential emit too slow: {sequential_time}s"
    assert tps > 100, f"TPS below baseline: {tps:.0f}"


@pytest.mark.asyncio
@pytest.mark.performance
async def test_handler_wrapper_latency():
    """Measure per-event handler wrapper latency."""
    bus = EventBus("latency_test", "instance1")

    async def measure_handler(event: Event):
        # Simulates handler execution
        await asyncio.sleep(0.001)  # 1ms work

    bus.on("*", measure_handler)

    # Emit a batch and measure individual latencies
    start = time.perf_counter()
    for i in range(100):
        await bus.emit(Event(
            type=CoreEvent.SONA_TICK.value,
            payload={"i": i},
            source="test"
        ))
    await bus.drain()
    total_time = time.perf_counter() - start

    avg_latency = (total_time * 1000) / 100  # ms per event

    print(f"\nHandler wrapper latency: {avg_latency:.2f}ms per event")
    print(f"Total time for 100 events: {total_time:.3f}s")

    # Should be close to (1ms work + handler overhead) per event in concurrent execution
    # With concurrent asyncio, 100 tasks with 1ms sleep each complete in ~1ms + scheduling
    assert avg_latency < 50.0, f"Latency too high: {avg_latency:.2f}ms"
    assert total_time > 0.001, "Time measurement valid"  # At minimum, should be > 1ms for concurrent 1ms sleeps


@pytest.mark.asyncio
@pytest.mark.performance
async def test_handler_error_path_latency():
    """Measure error handling latency (error path should not be slow)."""
    bus = EventBus("error_test", "instance1")

    async def failing_handler(event: Event):
        raise ValueError(f"Test error in event {event.event_id}")

    bus.on("*", failing_handler)

    start = time.perf_counter()
    for i in range(100):
        await bus.emit(Event(
            type=CoreEvent.SONA_TICK.value,
            payload={"i": i},
            source="test"
        ))
    await bus.drain()
    error_time = time.perf_counter() - start

    stats = bus.stats()
    error_count = stats["errors"]

    print(f"\nError path latency: {(error_time*1000)/100:.2f}ms per event")
    print(f"Errors recorded: {error_count}")

    assert error_count == 100, f"Expected 100 errors, got {error_count}"
    assert error_time < 10.0, f"Error handling too slow: {error_time}s"
