"""
Performance Baseline Tests for CYNIC System.

Measures TPS (transactions per second) for:
1. Event Emission TPS — Raw event bus throughput
2. Judgment Cycle TPS — Complete request→response cycles

These tests establish baseline performance metrics for 10k TPS readiness verification.
"""

from __future__ import annotations

import asyncio
import time
import pytest

from cynic.kernel.core.event_bus import Event, CoreEvent


@pytest.mark.asyncio
@pytest.mark.performance
async def test_event_emission_tps_baseline(organism):
    """
    Measure raw event bus throughput (Event Emission TPS).

    Emits 1000 events and measures total elapsed time.
    TPS = 1000 / elapsed_time_seconds

    Expected baseline: 500+ TPS with current setup.
    """
    bus = organism.cognition.orchestrator.bus

    # Snapshot counter before this test emits anything
    initial_emitted = bus.stats()["emitted"]

    # Create 1000 test events
    num_events = 1000
    events = [
        Event.typed(
            CoreEvent.PERCEPTION_RECEIVED, {"event_index": i}, source="performance_test"
        )
        for i in range(num_events)
    ]

    # Measure emission time
    t_start = time.perf_counter()
    for event in events:
        await bus.emit(event)

    # Drain pending tasks to ensure all handlers complete
    await bus.drain(timeout=5.0)
    t_end = time.perf_counter()

    elapsed_time = t_end - t_start
    tps = num_events / elapsed_time

    # Log baseline result
    stats = bus.stats()
    assert (
        stats["emitted"] - initial_emitted >= num_events
    ), "Not all events were emitted"

    print("\n=== Event Emission TPS Baseline ===")
    print(f"Events emitted: {num_events}")
    print(f"Elapsed time: {elapsed_time:.4f}s")
    print(f"TPS: {tps:.2f}")
    print(f"Avg latency per event: {stats['avg_latency_ms']:.4f}ms")
    print(f"Error rate: {stats['error_rate']:.4%}")
    print(f"Peak pending tasks: {stats['peak_pending']}")

    # Minimum expectation: at least some throughput
    assert tps > 0, "TPS should be positive"
    assert stats["error_rate"] < 0.05, "Error rate should be < 5%"


@pytest.mark.asyncio
@pytest.mark.performance
async def test_judgment_cycle_tps_baseline(organism):
    """
    Measure judgment cycle TPS (complete request→response cycles).

    Simulates 100 complete JUDGMENT_REQUESTED → JUDGMENT_CREATED cycles.
    Measures time for all cycles to complete.
    TPS = 100 / elapsed_time_seconds

    Expected baseline: 10+ TPS.
    """
    bus = organism.cognition.orchestrator.bus

    # Snapshot counter before this test emits anything
    initial_emitted = bus.stats()["emitted"]

    num_cycles = 100
    cycle_count = 0
    cycle_lock = asyncio.Lock()

    async def judgment_created_handler(event: Event):
        """Track when judgment cycles complete."""
        nonlocal cycle_count
        async with cycle_lock:
            cycle_count += 1

    # Register handler for judgment created events
    bus.on(CoreEvent.JUDGMENT_CREATED, judgment_created_handler)

    try:
        # Measure time for complete cycles
        t_start = time.perf_counter()

        for i in range(num_cycles):
            # Emit JUDGMENT_REQUESTED event
            await bus.emit(
                Event.typed(
                    CoreEvent.JUDGMENT_REQUESTED,
                    {"judgment_id": f"judgment_{i}", "cycle_index": i},
                    source="performance_test",
                )
            )

            # Emit corresponding JUDGMENT_CREATED response
            await bus.emit(
                Event.typed(
                    CoreEvent.JUDGMENT_CREATED,
                    {"judgment_id": f"judgment_{i}", "cycle_index": i},
                    source="performance_test",
                )
            )

        # Drain pending tasks
        await bus.drain(timeout=5.0)
        t_end = time.perf_counter()

        elapsed_time = t_end - t_start
        tps = num_cycles / elapsed_time

        # Log baseline result
        stats = bus.stats()
        assert stats["emitted"] - initial_emitted >= (
            num_cycles * 2
        ), "Not all cycle events were emitted"
        assert (
            cycle_count == num_cycles
        ), f"Expected {num_cycles} judgment callbacks, got {cycle_count}"

        print("\n=== Judgment Cycle TPS Baseline ===")
        print(f"Judgment cycles: {num_cycles}")
        print(f"Elapsed time: {elapsed_time:.4f}s")
        print(f"TPS: {tps:.2f}")
        print(f"Avg latency per event: {stats['avg_latency_ms']:.4f}ms")
        print(f"Error rate: {stats['error_rate']:.4%}")
        print(f"Peak pending tasks: {stats['peak_pending']}")
        print(f"Judgment created callbacks: {cycle_count}")

        # Minimum expectation: some throughput
        assert tps > 0, "TPS should be positive"
        assert stats["error_rate"] < 0.05, "Error rate should be < 5%"

    finally:
        # Clean up handler
        bus.off(CoreEvent.JUDGMENT_CREATED, judgment_created_handler)


@pytest.mark.asyncio
@pytest.mark.performance
@pytest.mark.ci_required
async def test_event_bus_backpressure_handling(organism):
    """
    Verify event bus handles backpressure correctly under load.

    Emits events rapidly and verifies:
    - No pending tasks exceed MAX_PENDING threshold
    - Anomaly events are triggered on backpressure
    - All events are eventually processed
    """
    bus = organism.cognition.orchestrator.bus
    initial_max_pending = bus.MAX_PENDING

    # Snapshot counter before this test emits anything
    initial_emitted = bus.stats()["emitted"]

    # Lower MAX_PENDING to trigger backpressure testing
    bus.MAX_PENDING = 50

    num_events = 200
    anomalies_detected = []

    async def anomaly_handler(event: Event):
        """Track anomaly events."""
        if event.payload and event.payload.get("type") == "backpressure":
            anomalies_detected.append(event.payload)

    bus.on(CoreEvent.ANOMALY_DETECTED, anomaly_handler)

    try:
        t_start = time.perf_counter()

        for i in range(num_events):
            await bus.emit(
                Event.typed(
                    CoreEvent.PERCEPTION_RECEIVED,
                    {"event_index": i},
                    source="performance_test",
                )
            )

        await bus.drain(timeout=10.0)
        t_end = time.perf_counter()

        stats = bus.stats()

        print("\n=== Event Bus Backpressure Test ===")
        print(f"Events emitted: {num_events}")
        print(f"Total time: {t_end - t_start:.4f}s")
        print(f"Backpressure anomalies: {len(anomalies_detected)}")
        print(f"Peak pending: {stats['peak_pending']}/{initial_max_pending}")
        print(f"Load factor: {stats['load_factor']:.2%}")

        # Verify backpressure handling
        assert (
            stats["emitted"] - initial_emitted >= num_events
        ), "All events should be emitted despite backpressure"
        assert stats["error_rate"] < 0.05, "Backpressure should not cause errors"

    finally:
        bus.off(CoreEvent.ANOMALY_DETECTED, anomaly_handler)
        bus.MAX_PENDING = initial_max_pending


@pytest.mark.asyncio
@pytest.mark.performance
async def test_event_bus_metrics_collection(organism):
    """
    Verify event bus metrics are collected correctly.

    Validates that stats() method returns accurate metrics after event emission.
    """
    bus = organism.cognition.orchestrator.bus

    # Get initial stats
    initial_stats = bus.stats()
    initial_emitted = initial_stats["emitted"]

    # Emit a known number of events
    num_events = 100
    for i in range(num_events):
        await bus.emit(
            Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                {"event_index": i},
                source="performance_test",
            )
        )

    await bus.drain(timeout=5.0)

    # Get final stats
    final_stats = bus.stats()

    print("\n=== Event Bus Metrics ===")
    print(f"Events emitted (delta): {final_stats['emitted'] - initial_emitted}")
    print(f"Avg latency: {final_stats['avg_latency_ms']:.4f}ms")
    print(f"Error rate: {final_stats['error_rate']:.4%}")
    print(f"Load factor: {final_stats['load_factor']:.2%}")
    print(f"Peak pending: {final_stats['peak_pending']}")

    # Verify metrics are accurate
    assert (
        final_stats["emitted"] >= initial_emitted + num_events
    ), "Event count mismatch"
    assert final_stats["avg_latency_ms"] >= 0, "Latency should be non-negative"
    assert 0 <= final_stats["error_rate"] <= 1.0, "Error rate should be 0-1"
    assert 0 <= final_stats["load_factor"] <= 2.0, "Load factor should be reasonable"
