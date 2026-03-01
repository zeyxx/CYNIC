"""Comprehensive tests for LNSP Layer 2 aggregation and temporal windows."""
from __future__ import annotations

import time

import pytest

from cynic.kernel.protocol.lnsp.layer2 import Aggregator, Layer2, TemporalWindow
from cynic.kernel.protocol.lnsp.messages import create_aggregated_state, create_raw_observation
from cynic.kernel.protocol.lnsp.types import AggregationType, LNSPMessage, ObservationType

# ============================================================================
# Test Fixtures
# ============================================================================


@pytest.fixture
def sample_observation():
    """Create a sample Layer 1 observation."""
    return create_raw_observation(
        observation_type=ObservationType.METRIC_SAMPLE,
        data={"cpu": 45.2, "memory": 78.9},
        source="TEST_SENSOR",
        instance_id="test_instance",
    )


@pytest.fixture
def sample_observations():
    """Create multiple sample observations."""
    return [
        create_raw_observation(
            observation_type=ObservationType.METRIC_SAMPLE,
            data={"cpu": 45.2, "memory": 78.9, "value": 1},
            source="TEST_SENSOR",
            instance_id="test_instance",
        ),
        create_raw_observation(
            observation_type=ObservationType.METRIC_SAMPLE,
            data={"cpu": 50.0, "memory": 80.0, "value": 2},
            source="TEST_SENSOR",
            instance_id="test_instance",
        ),
        create_raw_observation(
            observation_type=ObservationType.PROCESS_CREATED,
            data={"process_id": "p001", "name": "worker", "value": 3},
            source="TEST_SENSOR",
            instance_id="test_instance",
        ),
    ]


class CountingAggregator(Aggregator):
    """Test aggregator that counts observations."""

    async def aggregate(self, observations: list[LNSPMessage]) -> LNSPMessage | None:
        """Count observations and return summary."""
        if not observations:
            return None

        return create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"observation_count": len(observations)},
            source=f"aggregator:{self.aggregator_id}",
            based_on=[o.header.message_id for o in observations],
        )


class SumAggregator(Aggregator):
    """Test aggregator that sums values."""

    async def aggregate(self, observations: list[LNSPMessage]) -> LNSPMessage | None:
        """Sum 'value' fields and return summary."""
        if not observations:
            return None

        total = sum(obs.payload.get("value", 0) for obs in observations)

        return create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"total_value": total, "count": len(observations)},
            source=f"aggregator:{self.aggregator_id}",
            based_on=[o.header.message_id for o in observations],
        )


class EmptyAggregator(Aggregator):
    """Test aggregator that returns None."""

    async def aggregate(self, observations: list[LNSPMessage]) -> LNSPMessage | None:
        """Always return None."""
        return None


# ============================================================================
# TestTemporalWindow â€” Temporal Window Tests
# ============================================================================


class TestTemporalWindow:
    """Test TemporalWindow sliding window implementation."""

    def test_temporal_window_creation(self):
        """Test creating a temporal window."""
        window = TemporalWindow(window_size_sec=5.0)
        assert window.window_size_sec == 5.0
        assert window.observations == []
        assert isinstance(window.last_aggregation_time, float)

    def test_temporal_window_add_single_observation(self, sample_observation):
        """Test adding a single observation to window."""
        window = TemporalWindow(window_size_sec=5.0)
        window.add(sample_observation)
        assert len(window.observations) == 1
        assert window.observations[0] == sample_observation

    def test_temporal_window_add_multiple_observations(self, sample_observations):
        """Test adding multiple observations to window."""
        window = TemporalWindow(window_size_sec=60.0)
        for obs in sample_observations:
            window.add(obs)
        assert len(window.observations) == 3

    def test_temporal_window_add_expires_old_observations(self, sample_observation):
        """Test that old observations are expired when adding new ones."""
        window = TemporalWindow(window_size_sec=0.1)  # 100ms window

        # Add first observation
        window.add(sample_observation)
        assert len(window.observations) == 1

        # Wait for observation to become stale
        time.sleep(0.15)

        # Add second observation - should expire the first
        window.add(sample_observation)
        assert len(window.observations) == 1

    def test_temporal_window_should_aggregate_initially_false(self):
        """Test that should_aggregate returns True initially (0 time elapsed)."""
        window = TemporalWindow(window_size_sec=5.0)
        # Window was just created, so last_aggregation_time is recent
        # should_aggregate with default 5s interval will be False
        assert not window.should_aggregate(interval_sec=5.0)

    def test_temporal_window_should_aggregate_after_interval(self):
        """Test that should_aggregate returns True after interval elapsed."""
        window = TemporalWindow(window_size_sec=5.0)
        # Set last_aggregation_time to the past
        window.last_aggregation_time = time.time() - 10.0
        # Now should_aggregate should return True (10s > 5s)
        assert window.should_aggregate(interval_sec=5.0)

    def test_temporal_window_should_aggregate_checks_interval(self):
        """Test that should_aggregate respects interval parameter."""
        window = TemporalWindow(window_size_sec=5.0)
        window.last_aggregation_time = time.time() - 2.0  # 2 seconds ago

        # With 1s interval, should return True (2s > 1s)
        assert window.should_aggregate(interval_sec=1.0)

        # With 5s interval, should return False (2s < 5s)
        assert not window.should_aggregate(interval_sec=5.0)

    def test_temporal_window_reset_updates_aggregation_time(self):
        """Test that reset() updates last_aggregation_time."""
        window = TemporalWindow(window_size_sec=5.0)
        old_time = window.last_aggregation_time
        time.sleep(0.1)
        window.reset()
        new_time = window.last_aggregation_time
        assert new_time > old_time

    def test_temporal_window_reset_allows_next_aggregation(self):
        """Test that reset() resets aggregation timer."""
        window = TemporalWindow(window_size_sec=5.0)
        window.last_aggregation_time = time.time() - 10.0
        assert window.should_aggregate(interval_sec=5.0)

        window.reset()
        # After reset, should_aggregate should be False again
        assert not window.should_aggregate(interval_sec=5.0)


# ============================================================================
# TestAggregatorInterface â€” Aggregator Abstract Base Class
# ============================================================================


class TestAggregatorInterface:
    """Test Aggregator abstract base class."""

    def test_aggregator_has_aggregator_id(self):
        """Test aggregator stores aggregator_id."""
        agg = CountingAggregator("agg_001")
        assert agg.aggregator_id == "agg_001"

    @pytest.mark.asyncio
    async def test_counting_aggregator_counts_observations(self, sample_observations):
        """Test CountingAggregator aggregates observations."""
        agg = CountingAggregator("agg_001")
        result = await agg.aggregate(sample_observations)

        assert result is not None
        assert result.payload["observation_count"] == 3

    @pytest.mark.asyncio
    async def test_aggregator_returns_none_for_empty_list(self):
        """Test aggregator can return None for empty observations."""
        agg = CountingAggregator("agg_001")
        result = await agg.aggregate([])
        assert result is None

    @pytest.mark.asyncio
    async def test_sum_aggregator_sums_values(self, sample_observations):
        """Test SumAggregator sums observation values."""
        agg = SumAggregator("agg_001")
        result = await agg.aggregate(sample_observations)

        assert result is not None
        # Values are 1, 2, 3
        assert result.payload["total_value"] == 6
        assert result.payload["count"] == 3

    @pytest.mark.asyncio
    async def test_empty_aggregator_always_returns_none(self, sample_observations):
        """Test EmptyAggregator always returns None."""
        agg = EmptyAggregator("agg_001")
        result = await agg.aggregate(sample_observations)
        assert result is None


# ============================================================================
# TestLayer2Registration â€” Aggregator Registration
# ============================================================================


class TestLayer2Registration:
    """Test registering aggregators with Layer2."""

    def test_layer2_creation(self):
        """Test creating Layer2 manager."""
        layer2 = Layer2()
        assert layer2.aggregators == {}
        assert layer2.windows == {}
        assert layer2.subscribers == []

    def test_layer2_register_single_aggregator(self):
        """Test registering a single aggregator."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")

        layer2.register_aggregator(agg, window_sizes=[5.0, 60.0])

        assert "agg_001" in layer2.aggregators
        assert layer2.aggregators["agg_001"] is agg
        assert "agg_001" in layer2.windows
        assert 5.0 in layer2.windows["agg_001"]
        assert 60.0 in layer2.windows["agg_001"]

    def test_layer2_register_multiple_aggregators(self):
        """Test registering multiple aggregators."""
        layer2 = Layer2()
        agg1 = CountingAggregator("agg_001")
        agg2 = SumAggregator("agg_002")

        layer2.register_aggregator(agg1, window_sizes=[5.0])
        layer2.register_aggregator(agg2, window_sizes=[5.0])

        assert len(layer2.aggregators) == 2
        assert "agg_001" in layer2.aggregators
        assert "agg_002" in layer2.aggregators

    def test_layer2_register_multiple_window_sizes(self):
        """Test aggregator can have multiple window sizes."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")

        window_sizes = [5.0, 60.0, 300.0, 3600.0]
        layer2.register_aggregator(agg, window_sizes=window_sizes)

        assert len(layer2.windows["agg_001"]) == 4
        for size in window_sizes:
            assert size in layer2.windows["agg_001"]
            assert isinstance(layer2.windows["agg_001"][size], TemporalWindow)


# ============================================================================
# TestLayer2Subscription â€” Subscription Pattern
# ============================================================================


class TestLayer2Subscription:
    """Test subscription pattern for Layer 3 Judge callbacks."""

    def test_layer2_subscribe_callback(self):
        """Test subscribing a callback."""
        layer2 = Layer2()

        def callback(msg: LNSPMessage) -> None:
            pass

        layer2.subscribe(callback)
        assert len(layer2.subscribers) == 1
        assert callback in layer2.subscribers

    def test_layer2_subscribe_multiple_callbacks(self):
        """Test subscribing multiple callbacks."""
        layer2 = Layer2()

        def callback1(msg: LNSPMessage) -> None:
            pass

        def callback2(msg: LNSPMessage) -> None:
            pass

        layer2.subscribe(callback1)
        layer2.subscribe(callback2)

        assert len(layer2.subscribers) == 2
        assert callback1 in layer2.subscribers
        assert callback2 in layer2.subscribers


# ============================================================================
# TestLayer2ProcessObservation â€” Observation Routing
# ============================================================================


class TestLayer2ProcessObservation:
    """Test routing observations to windows."""

    @pytest.mark.asyncio
    async def test_layer2_process_observation_single_aggregator(
        self, sample_observation
    ):
        """Test processing observation to single aggregator."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0])

        await layer2.process_observation(sample_observation)

        # Check observation reached the window
        window = layer2.windows["agg_001"][5.0]
        assert len(window.observations) == 1
        assert window.observations[0] == sample_observation

    @pytest.mark.asyncio
    async def test_layer2_process_observation_multiple_aggregators(
        self, sample_observation
    ):
        """Test processing observation to multiple aggregators."""
        layer2 = Layer2()
        agg1 = CountingAggregator("agg_001")
        agg2 = SumAggregator("agg_002")

        layer2.register_aggregator(agg1, window_sizes=[5.0])
        layer2.register_aggregator(agg2, window_sizes=[5.0])

        await layer2.process_observation(sample_observation)

        # Check observation reached both windows
        assert len(layer2.windows["agg_001"][5.0].observations) == 1
        assert len(layer2.windows["agg_002"][5.0].observations) == 1

    @pytest.mark.asyncio
    async def test_layer2_process_observation_multiple_windows(
        self, sample_observation
    ):
        """Test processing observation to multiple windows of same aggregator."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0, 60.0])

        await layer2.process_observation(sample_observation)

        # Check observation reached both windows
        assert len(layer2.windows["agg_001"][5.0].observations) == 1
        assert len(layer2.windows["agg_001"][60.0].observations) == 1

    @pytest.mark.asyncio
    async def test_layer2_process_multiple_observations(self, sample_observations):
        """Test processing multiple observations."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0])

        for obs in sample_observations:
            await layer2.process_observation(obs)

        window = layer2.windows["agg_001"][5.0]
        assert len(window.observations) == 3


# ============================================================================
# TestLayer2Aggregation â€” Aggregation Cycle
# ============================================================================


class TestLayer2Aggregation:
    """Test aggregation cycle and subscriber notification."""

    @pytest.mark.asyncio
    async def test_layer2_aggregate_calls_aggregator(self, sample_observations):
        """Test aggregate() calls aggregator for ready windows."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0])

        # Add observations
        for obs in sample_observations:
            await layer2.process_observation(obs)

        # Force window to be ready for aggregation
        window = layer2.windows["agg_001"][5.0]
        window.last_aggregation_time = time.time() - 10.0

        # Aggregate
        received = []

        def callback(msg):
            received.append(msg)

        layer2.subscribe(callback)
        await layer2.aggregate()

        # Should have received aggregated message
        assert len(received) == 1
        assert received[0].payload["observation_count"] == 3

    @pytest.mark.asyncio
    async def test_layer2_aggregate_calls_subscribers(self, sample_observations):
        """Test aggregate() sends results to all subscribers."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0])

        for obs in sample_observations:
            await layer2.process_observation(obs)

        window = layer2.windows["agg_001"][5.0]
        window.last_aggregation_time = time.time() - 10.0

        received1 = []
        received2 = []

        def callback1(msg):
            received1.append(msg)

        def callback2(msg):
            received2.append(msg)

        layer2.subscribe(callback1)
        layer2.subscribe(callback2)
        await layer2.aggregate()

        # Both subscribers should receive the message
        assert len(received1) == 1
        assert len(received2) == 1

    @pytest.mark.asyncio
    async def test_layer2_aggregate_respects_should_aggregate(self, sample_observations):
        """Test aggregate() only aggregates ready windows."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0])

        for obs in sample_observations:
            await layer2.process_observation(obs)

        received = []

        def callback(msg):
            received.append(msg)

        layer2.subscribe(callback)

        # First aggregate - window not ready (just created)
        await layer2.aggregate()
        assert len(received) == 0

        # Force window to be ready
        window = layer2.windows["agg_001"][5.0]
        window.last_aggregation_time = time.time() - 10.0

        # Second aggregate - window ready
        await layer2.aggregate()
        assert len(received) == 1

    @pytest.mark.asyncio
    async def test_layer2_aggregate_ignores_none_results(self, sample_observations):
        """Test aggregate() ignores None results from aggregators."""
        layer2 = Layer2()
        agg = EmptyAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0])

        for obs in sample_observations:
            await layer2.process_observation(obs)

        window = layer2.windows["agg_001"][5.0]
        window.last_aggregation_time = time.time() - 10.0

        received = []

        def callback(msg):
            received.append(msg)

        layer2.subscribe(callback)
        await layer2.aggregate()

        # Should not receive anything (aggregator returned None)
        assert len(received) == 0

    @pytest.mark.asyncio
    async def test_layer2_aggregate_resets_window(self, sample_observations):
        """Test aggregate() resets window after aggregation."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0])

        for obs in sample_observations:
            await layer2.process_observation(obs)

        window = layer2.windows["agg_001"][5.0]
        old_time = window.last_aggregation_time
        window.last_aggregation_time = time.time() - 10.0

        layer2.subscribe(lambda msg: None)
        await layer2.aggregate()

        # Window should be reset
        new_time = window.last_aggregation_time
        assert new_time > old_time

    @pytest.mark.asyncio
    async def test_layer2_aggregate_multiple_aggregators(self, sample_observations):
        """Test aggregation with multiple aggregators."""
        layer2 = Layer2()
        agg1 = CountingAggregator("agg_001")
        agg2 = SumAggregator("agg_002")

        layer2.register_aggregator(agg1, window_sizes=[5.0])
        layer2.register_aggregator(agg2, window_sizes=[5.0])

        for obs in sample_observations:
            await layer2.process_observation(obs)

        # Force both windows to be ready
        for agg_id in ["agg_001", "agg_002"]:
            window = layer2.windows[agg_id][5.0]
            window.last_aggregation_time = time.time() - 10.0

        received = []

        def callback(msg):
            received.append(msg)

        layer2.subscribe(callback)
        await layer2.aggregate()

        # Should receive results from both aggregators
        assert len(received) == 2


# ============================================================================
# TestLayer2Stats â€” Statistics
# ============================================================================


class TestLayer2Stats:
    """Test statistics reporting."""

    def test_layer2_stats_empty(self):
        """Test stats on empty Layer2."""
        layer2 = Layer2()
        stats = layer2.stats()

        assert "aggregator_count" in stats
        assert "total_observations" in stats
        assert "subscriber_count" in stats

        assert stats["aggregator_count"] == 0
        assert stats["total_observations"] == 0
        assert stats["subscriber_count"] == 0

    @pytest.mark.asyncio
    async def test_layer2_stats_with_observations(self, sample_observations):
        """Test stats with observations in windows."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0, 60.0])

        for obs in sample_observations:
            await layer2.process_observation(obs)

        stats = layer2.stats()

        # Each observation is in both windows (5.0 and 60.0)
        assert stats["aggregator_count"] == 1
        assert stats["total_observations"] == 6  # 3 obs * 2 windows
        assert stats["subscriber_count"] == 0

    @pytest.mark.asyncio
    async def test_layer2_stats_with_subscribers(self, sample_observations):
        """Test stats with subscribed callbacks."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0])

        layer2.subscribe(lambda msg: None)
        layer2.subscribe(lambda msg: None)

        stats = layer2.stats()
        assert stats["subscriber_count"] == 2

    @pytest.mark.asyncio
    async def test_layer2_stats_multiple_aggregators(self, sample_observations):
        """Test stats with multiple aggregators."""
        layer2 = Layer2()
        agg1 = CountingAggregator("agg_001")
        agg2 = SumAggregator("agg_002")

        layer2.register_aggregator(agg1, window_sizes=[5.0])
        layer2.register_aggregator(agg2, window_sizes=[5.0])

        for obs in sample_observations:
            await layer2.process_observation(obs)

        stats = layer2.stats()

        assert stats["aggregator_count"] == 2
        assert stats["total_observations"] == 6  # 3 obs * 2 aggregators


# ============================================================================
# TestLayer2Integration â€” Integration Tests
# ============================================================================


class TestLayer2Integration:
    """Integration tests for complete Layer2 workflows."""

    @pytest.mark.asyncio
    async def test_full_workflow(self, sample_observations):
        """Test complete Layer2 workflow from observations to aggregation."""
        layer2 = Layer2()
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[5.0, 60.0])

        # Subscribe to results
        results = []

        def callback(msg):
            results.append(msg)

        layer2.subscribe(callback)

        # Process observations
        for obs in sample_observations:
            await layer2.process_observation(obs)

        # Force windows to be ready
        for window_size in [5.0, 60.0]:
            window = layer2.windows["agg_001"][window_size]
            window.last_aggregation_time = time.time() - 10.0

        # Run aggregation
        await layer2.aggregate()

        # Should have results from both windows
        assert len(results) == 2
        for result in results:
            assert result.payload["observation_count"] == 3

    @pytest.mark.asyncio
    async def test_multiple_aggregation_cycles(self, sample_observation):
        """Test multiple aggregation cycles with window expiration."""
        layer2 = Layer2()
        # Use a very short window for testing
        agg = CountingAggregator("agg_001")
        layer2.register_aggregator(agg, window_sizes=[0.1])

        results = []

        def callback(msg):
            results.append(msg)

        layer2.subscribe(callback)

        # First cycle
        await layer2.process_observation(sample_observation)
        window = layer2.windows["agg_001"][0.1]
        window.last_aggregation_time = time.time() - 10.0
        await layer2.aggregate()

        assert len(results) == 1
        assert results[0].payload["observation_count"] == 1

        # Wait for observations to expire from window
        time.sleep(0.15)

        # Second cycle - observations from first cycle should be expired
        await layer2.process_observation(sample_observation)
        window.last_aggregation_time = time.time() - 10.0
        await layer2.aggregate()

        assert len(results) == 2
        assert results[1].payload["observation_count"] == 1
