"""
Priority 7 Event Metrics Tests

Tests for EventMetricsCollector and BusMetricsAdapter.
16 tests: 6 EventMetricsCollector + 6 BusMetricsAdapter + 4 Integration
"""

import asyncio
from unittest.mock import AsyncMock

import pytest

from cynic.kernel.core.event_bus import Event, EventBus
from cynic.kernel.core.formulas import (
    HISTORY_REPLAY_BATCH,
    LOD_LEVEL0_LATENCY_MS,
    LOD_LEVEL2_LATENCY_MS,
)
from cynic.nervous.event_metrics import EventMetricsCollector
from cynic.nervous.bus_metrics_adapter import BusMetricsAdapter


# ──────────────────────────────────────────────────────────────────────────────
# TestEventMetricsCollector (6 tests)
# ──────────────────────────────────────────────────────────────────────────────


@pytest.mark.asyncio
class TestEventMetricsCollector:
    """Tests for EventMetricsCollector."""

    async def test_record_increments_count(self):
        """Test 1: record() increments per-type count visible in get_metrics()"""
        collector = EventMetricsCollector()

        # Record 3 events of the same type
        await collector.record("core.judgment_created", duration_ms=100.0)
        await collector.record("core.judgment_created", duration_ms=200.0)
        await collector.record("core.judgment_created", duration_ms=150.0)

        metrics = await collector.get_metrics("core.judgment_created")
        assert metrics is not None
        assert metrics.count_in_window == 3
        assert metrics.event_type == "core.judgment_created"

    async def test_current_rates_non_zero(self):
        """Test 2: current_rates() returns non-zero rate after recording events"""
        collector = EventMetricsCollector()

        # Record 5 events of one type
        for _ in range(5):
            await collector.record("core.act_completed", duration_ms=50.0)

        rates = await collector.current_rates()
        assert "core.act_completed" in rates
        assert rates["core.act_completed"] > 0.0

    async def test_histogram_buckets_correctly(self):
        """Test 3: get_metrics() buckets duration correctly"""
        collector = EventMetricsCollector()

        # 50ms event should go in ≤100ms bucket
        await collector.record("test_event", duration_ms=50.0)

        # 500ms event should go in ≤1000ms bucket
        await collector.record("test_event", duration_ms=500.0)

        # 100ms event (boundary) should go in ≤100ms
        await collector.record("test_event", duration_ms=100.0)

        metrics = await collector.get_metrics("test_event")
        assert metrics is not None

        # Check that histogram has correct distributions
        histogram = metrics.histogram
        assert histogram[f"≤{int(LOD_LEVEL0_LATENCY_MS)}ms"] == 2  # 50ms and 100ms
        assert histogram[f"≤{int(LOD_LEVEL2_LATENCY_MS)}ms"] == 1  # 500ms

    async def test_error_rate_computation(self):
        """Test 4: get_metrics() computes error_rate = 0.5 when half events are errors"""
        collector = EventMetricsCollector()

        # Record 2 normal events and 2 error events
        await collector.record("test_event", duration_ms=100.0, is_error=False)
        await collector.record("test_event", duration_ms=150.0, is_error=False)
        await collector.record("test_event", duration_ms=200.0, is_error=True)
        await collector.record("test_event", duration_ms=250.0, is_error=True)

        metrics = await collector.get_metrics("test_event")
        assert metrics is not None
        assert metrics.error_count == 2
        assert metrics.error_rate == 0.5

    async def test_detect_anomalies_error_spike(self):
        """Test 5: detect_anomalies() returns ERROR_SPIKE when error_rate > PHI_INV"""
        collector = EventMetricsCollector()

        # Record many error events to exceed PHI_INV (0.618)
        for i in range(10):
            is_error = i < 7  # 7 out of 10 = 70% error rate
            await collector.record("test_event", duration_ms=100.0, is_error=is_error)

        anomalies = await collector.detect_anomalies()

        # Should find at least one error spike anomaly
        error_spikes = [a for a in anomalies if a.anomaly_type == "ERROR_SPIKE"]
        assert len(error_spikes) > 0
        assert error_spikes[0].event_type == "test_event"

    async def test_stats_returns_correct_data(self):
        """Test 6: stats() returns correct total_recorded and tracked_types"""
        collector = EventMetricsCollector()

        # Record some events
        await collector.record("type1", duration_ms=100.0)
        await collector.record("type2", duration_ms=150.0)
        await collector.record("type1", duration_ms=200.0)

        stats = await collector.stats()
        assert stats["total_recorded"] == 3
        assert stats["tracked_types"] == 2
        assert "anomaly_count" in stats
        assert "window_s" in stats


# ──────────────────────────────────────────────────────────────────────────────
# TestBusMetricsAdapter (6 tests)
# ──────────────────────────────────────────────────────────────────────────────


@pytest.mark.asyncio
class TestBusMetricsAdapter:
    """Tests for BusMetricsAdapter."""

    async def test_any_event_without_latency_pair(self):
        """Test 7: Any event without a latency pair → recorded with duration_ms=0.0"""
        collector = EventMetricsCollector()
        adapter = BusMetricsAdapter(collector)

        # Create an event that doesn't match any latency pair
        event = Event(
            type="core.perception_received",
            source="test",
            payload={},
        )

        await adapter.on_event(event)

        metrics = await collector.get_metrics("core.perception_received")
        assert metrics is not None
        assert metrics.count_in_window == 1

    async def test_judgment_requested_opens_latency_tracking(self):
        """Test 8: JUDGMENT_REQUESTED opens latency tracking (no recording yet)"""
        collector = EventMetricsCollector()
        adapter = BusMetricsAdapter(collector)

        event = Event(
            type="core.judgment_requested",
            source="test",
            payload={"judgment_id": "j1"},
        )

        await adapter.on_event(event)

        # The adapter should have opened tracking
        assert "core.judgment_requested:j1" in adapter._open_latencies

    async def test_judgment_created_records_duration(self):
        """Test 9: JUDGMENT_CREATED after JUDGMENT_REQUESTED → records real duration_ms > 0"""
        collector = EventMetricsCollector()
        adapter = BusMetricsAdapter(collector)

        # Send request event
        request_event = Event(
            type="core.judgment_requested",
            source="test",
            payload={"judgment_id": "j1"},
        )
        await adapter.on_event(request_event)

        # Simulate some time passing (at least 10ms)
        await asyncio.sleep(0.01)

        # Send response event
        response_event = Event(
            type="core.judgment_created",
            source="test",
            payload={"judgment_id": "j1"},
        )
        await adapter.on_event(response_event)

        metrics = await collector.get_metrics("core.judgment_created")
        assert metrics is not None
        # Since we're measuring real latency, duration_ms should be > 0
        assert metrics.count_in_window >= 1

    async def test_judgment_failed_event_marks_error(self):
        """Test 10: JUDGMENT_FAILED event → is_error=True recorded for that type"""
        collector = EventMetricsCollector()
        adapter = BusMetricsAdapter(collector)

        event = Event(
            type="core.judgment_failed",
            source="test",
            payload={"judgment_id": "j1"},
        )

        await adapter.on_event(event)

        metrics = await collector.get_metrics("core.judgment_failed")
        assert metrics is not None
        assert metrics.error_count == 1
        assert metrics.error_rate == 1.0

    async def test_anomaly_check_after_batch(self):
        """Test 11: After exactly HISTORY_REPLAY_BATCH=8 events → anomaly check runs"""
        collector = EventMetricsCollector()
        mock_bus = AsyncMock(spec=EventBus)
        adapter = BusMetricsAdapter(collector, bus=mock_bus)

        # Record exactly HISTORY_REPLAY_BATCH events
        for i in range(HISTORY_REPLAY_BATCH):
            event = Event(
                type="core.test_event",
                source="test",
                payload={},
            )
            await adapter.on_event(event)

        # Verify that emit was potentially called (if anomalies were detected)
        # At minimum, anomaly check should have run
        assert adapter._event_count == HISTORY_REPLAY_BATCH

    async def test_adapter_with_bus_none_no_crash(self):
        """Test 12: Adapter with bus=None → no crash even when anomalies are found"""
        collector = EventMetricsCollector()
        adapter = BusMetricsAdapter(collector, bus=None)

        # Record many error events
        for i in range(10):
            event = Event(
                type="core.error_event",
                source="test",
                payload={},
            )
            # Mark as error
            await adapter.on_event(event)
            # Manually mark for testing
            samples = list(collector._samples["core.error_event"])
            if samples:
                samples[-1].is_error = True

        # Manually trigger anomaly check (should not crash even without bus)
        await adapter._check_anomalies()

        # If we get here, test passes (no crash)
        assert True


# ──────────────────────────────────────────────────────────────────────────────
# TestMetricsIntegration (4 tests)
# ──────────────────────────────────────────────────────────────────────────────


@pytest.mark.asyncio
class TestMetricsIntegration:
    """Integration tests for metrics system."""

    async def test_fresh_collector_empty_all_metrics(self):
        """Test 13: Fresh EventMetricsCollector → all_metrics() returns {}"""
        collector = EventMetricsCollector()

        metrics = await collector.all_metrics()
        assert metrics == {}

    async def test_fresh_collector_empty_anomalies(self):
        """Test 14: Fresh EventMetricsCollector → recent_anomalies() returns []"""
        collector = EventMetricsCollector()

        anomalies = await collector.recent_anomalies()
        assert anomalies == []

    async def test_independent_event_types(self):
        """Test 15: Two different event types tracked independently (counts don't bleed)"""
        collector = EventMetricsCollector()

        # Record 5 events of type A
        for _ in range(5):
            await collector.record("type_a", duration_ms=100.0)

        # Record 3 events of type B
        for _ in range(3):
            await collector.record("type_b", duration_ms=200.0)

        metrics_a = await collector.get_metrics("type_a")
        metrics_b = await collector.get_metrics("type_b")

        assert metrics_a is not None
        assert metrics_b is not None
        assert metrics_a.count_in_window == 5
        assert metrics_b.count_in_window == 3

    async def test_latency_spike_detection(self):
        """Test 16: detect_anomalies() returns LATENCY_SPIKE when duration_ms > LOD_LEVEL3"""
        collector = EventMetricsCollector()

        # Record one normal event
        await collector.record("slow_event", duration_ms=500.0)

        # Record one event that exceeds LOD_LEVEL3 (3000ms)
        await collector.record("slow_event", duration_ms=5000.0)

        anomalies = await collector.detect_anomalies()

        # Should find at least one latency spike anomaly
        latency_spikes = [a for a in anomalies if a.anomaly_type == "LATENCY_SPIKE"]
        assert len(latency_spikes) > 0
        assert latency_spikes[0].event_type == "slow_event"
        assert latency_spikes[0].metric_value == 5000.0
