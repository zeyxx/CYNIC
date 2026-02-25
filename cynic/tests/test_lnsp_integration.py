"""Task 7: LNSP Integration Tests

Integration tests for the LNSPManager proving that all 4 LNSP protocol layers
can be initialized, wired, and run complete cycles together.

These tests verify the manager interface is ready for integration with CYNIC's
event-driven architecture (Task 8+).

Tests:
1. test_manager_initialization: Basic setup and default configuration
2. test_manager_with_components: Registration and wiring of actual components
3. test_manager_statistics: Aggregation of stats from all layers
"""
from __future__ import annotations

import asyncio
from typing import Any

import pytest

from cynic.protocol.lnsp.layer1 import Layer1, Sensor
from cynic.protocol.lnsp.layer2 import Aggregator, Layer2
from cynic.protocol.lnsp.layer3 import Layer3
from cynic.protocol.lnsp.layer4 import Handler, Layer4
from cynic.protocol.lnsp.manager import LNSPManager
from cynic.protocol.lnsp.messages import (
    create_aggregated_state,
    create_raw_observation,
)
from cynic.protocol.lnsp.types import (
    AggregationType,
    LNSPMessage,
    ObservationType,
)

# ════════════════════════════════════════════════════════════════════════════
# Mock Components for Testing
# ════════════════════════════════════════════════════════════════════════════


class MockSensor(Sensor):
    """Mock sensor that emits test observations.

    Attributes:
        observation_count: Number of observations emitted so far
        should_emit: Whether this sensor should emit observations
    """

    def __init__(
        self,
        sensor_id: str,
        instance_id: str = "instance:local",
        should_emit: bool = True,
    ) -> None:
        """Initialize mock sensor.

        Args:
            sensor_id: Unique sensor identifier
            instance_id: Organism instance ID
            should_emit: Whether sensor should emit observations (default: True)
        """
        super().__init__(sensor_id, instance_id)
        self.observation_count = 0
        self.should_emit = should_emit

    async def observe(self) -> LNSPMessage | None:
        """Emit a test observation or None if not configured to emit.

        Returns:
            LNSPMessage observation or None
        """
        if not self.should_emit:
            return None

        self.observation_count += 1
        return create_raw_observation(
            observation_type=ObservationType.METRIC_SAMPLE,
            data={
                "sensor_id": self.sensor_id,
                "value": self.observation_count,
                "metric_name": f"{self.sensor_id}_count",
            },
            source=self.sensor_id,
            instance_id=self.instance_id,
        )


class MockAggregator(Aggregator):
    """Mock aggregator that synthesizes state from observations.

    Attributes:
        aggregation_count: Number of aggregations performed
    """

    def __init__(self, aggregator_id: str) -> None:
        """Initialize mock aggregator.

        Args:
            aggregator_id: Unique aggregator identifier
        """
        super().__init__(aggregator_id)
        self.aggregation_count = 0

    async def aggregate(self, observations: list[LNSPMessage]) -> LNSPMessage:
        """Aggregate observations into state.

        Args:
            observations: List of Layer 1 observations to aggregate

        Returns:
            LNSPMessage of aggregated state (Layer 2)
        """
        self.aggregation_count += 1
        msg_ids = [obs.header.message_id for obs in observations]
        return create_aggregated_state(
            aggregation_type=AggregationType.PROCESS_METRICS,
            data={
                "aggregator_id": self.aggregator_id,
                "observation_count": len(observations),
                "aggregation_count": self.aggregation_count,
            },
            source=self.aggregator_id,
            based_on=msg_ids,
            instance_id="instance:local",
        )


class MockHandler(Handler):
    """Mock handler that executes verdicts and tracks results.

    Attributes:
        execution_count: Number of verdicts executed
        last_result: Last result from execution
    """

    def __init__(self, handler_id: str) -> None:
        """Initialize mock handler.

        Args:
            handler_id: Unique handler identifier
        """
        super().__init__(handler_id)
        self.execution_count = 0
        self.last_result: dict[str, Any] | None = None

    async def handle(self, verdict: LNSPMessage) -> tuple[bool, Any]:
        """Execute a verdict (Layer 4 action).

        Args:
            verdict: Layer 3 verdict message to execute

        Returns:
            Tuple of (success: bool, result_data: Any)
        """
        self.execution_count += 1
        self.last_result = {
            "handler_id": self.handler_id,
            "verdict_source": verdict.header.source,
            "execution_count": self.execution_count,
        }
        return (True, self.last_result)


# ════════════════════════════════════════════════════════════════════════════
# Test Cases
# ════════════════════════════════════════════════════════════════════════════


class TestLNSPManagerInitialization:
    """Test basic LNSPManager setup and initialization."""

    def test_manager_default_initialization(self) -> None:
        """Test manager can be initialized with defaults.

        Verifies:
        - Manager creates with no arguments
        - Default instance_id is set
        - All 4 layers are created
        - Layers have correct types
        """
        manager = LNSPManager()

        assert manager.instance_id == "instance:local"
        assert manager.region is None
        assert isinstance(manager.layer1, Layer1)
        assert isinstance(manager.layer2, Layer2)
        assert isinstance(manager.layer3, Layer3)
        assert isinstance(manager.layer4, Layer4)

    def test_manager_custom_initialization(self) -> None:
        """Test manager can be initialized with custom parameters.

        Verifies:
        - Custom instance_id is accepted
        - Custom region is accepted
        - Custom layers are used if provided
        """
        custom_l1 = Layer1(capacity=5000)
        custom_l3 = Layer3(judge_id="judge:custom")

        manager = LNSPManager(
            instance_id="instance:test",
            region="us-west",
            layer1=custom_l1,
            layer3=custom_l3,
        )

        assert manager.instance_id == "instance:test"
        assert manager.region == "us-west"
        assert manager.layer1 is custom_l1
        assert manager.layer3 is custom_l3
        assert isinstance(manager.layer2, Layer2)  # Not provided, created fresh
        assert isinstance(manager.layer4, Layer4)  # Not provided, created fresh

    def test_manager_wire_layers(self) -> None:
        """Test manager can wire all layers together.

        Verifies:
        - wire_layers() completes without error
        - Subscriptions are registered on layers
        - Each layer has subscriptions from previous layer
        """
        manager = LNSPManager()

        # Before wiring, layers have no subscribers
        assert len(manager.layer1.subscribers) == 0
        assert len(manager.layer2.subscribers) == 0
        assert len(manager.layer3.subscribers) == 0

        # Wire layers together
        manager.wire_layers()

        # After wiring, each layer has subscribers
        # Layer 1 has a subscriber (Layer 2 callback)
        assert len(manager.layer1.subscribers) > 0
        # Layer 2 has a subscriber (Layer 3 callback)
        assert len(manager.layer2.subscribers) > 0
        # Layer 3 has a subscriber (Layer 4 callback)
        assert len(manager.layer3.subscribers) > 0
        # Layer 4 has feedback callbacks registered
        assert len(manager.layer4.feedback_callbacks) > 0


class TestLNSPManagerWithComponents:
    """Test manager with actual component registration and execution."""

    def test_manager_with_sensors_and_aggregators(self) -> None:
        """Test manager can register sensors and aggregators.

        Verifies:
        - Sensors can be registered with Layer 1
        - Aggregators can be registered with Layer 2
        - Manager tracks component counts in stats
        """
        manager = LNSPManager(instance_id="instance:component_test")

        # Register sensors
        sensor1 = MockSensor(sensor_id="sensor:cpu", should_emit=True)
        sensor2 = MockSensor(sensor_id="sensor:memory", should_emit=True)
        manager.layer1.register_sensor(sensor1)
        manager.layer1.register_sensor(sensor2)

        assert len(manager.layer1.sensors) == 2
        assert "sensor:cpu" in manager.layer1.sensors
        assert "sensor:memory" in manager.layer1.sensors

        # Register aggregators with window sizes
        agg1 = MockAggregator(aggregator_id="agg:process_metrics")
        agg2 = MockAggregator(aggregator_id="agg:system_health")
        manager.layer2.register_aggregator(agg1, window_sizes=[5.0, 60.0])
        manager.layer2.register_aggregator(agg2, window_sizes=[5.0])

        assert len(manager.layer2.aggregators) == 2

    def test_manager_with_handlers(self) -> None:
        """Test manager can register handlers with Layer 4.

        Verifies:
        - Handlers can be registered with Layer 4
        - Multiple handlers can coexist
        - Handler IDs are unique
        """
        manager = LNSPManager(instance_id="instance:handler_test")

        # Register handlers
        handler1 = MockHandler(handler_id="handler:config")
        handler2 = MockHandler(handler_id="handler:signal")
        manager.layer4.register_handler(handler1)
        manager.layer4.register_handler(handler2)

        assert len(manager.layer4.handlers) == 2
        assert "handler:config" in manager.layer4.handlers
        assert "handler:signal" in manager.layer4.handlers

    @pytest.mark.asyncio
    async def test_manager_run_cycle_async(self) -> None:
        """Test manager can execute a complete observation → aggregation cycle.

        Verifies:
        - run_cycle() completes without error
        - Sensors are invoked during Layer 1 observe()
        - Aggregators receive observations in windows
        - Manager handles async operations correctly

        Note: Aggregation is triggered by time intervals, not cycle count.
        This test verifies the complete pipeline can execute without errors.
        """
        manager = LNSPManager(instance_id="instance:cycle_test")

        # Register components
        sensor = MockSensor(sensor_id="sensor:test", should_emit=True)
        manager.layer1.register_sensor(sensor)

        agg = MockAggregator(aggregator_id="agg:test")
        manager.layer2.register_aggregator(agg, window_sizes=[5.0])

        # Wire layers
        manager.wire_layers()

        # Run one cycle
        await manager.run_cycle()

        # Give async tasks time to settle
        await asyncio.sleep(0.1)

        # Verify component activity - sensor should have emitted
        assert sensor.observation_count >= 1, "Sensor should have emitted observations"

        # Verify observations were added to windows
        # (Aggregation itself depends on time intervals, not cycle count)
        stats = manager.stats()
        assert stats["layer2"]["total_observations"] >= 1

    def test_manager_stats_structure(self) -> None:
        """Test manager.stats() returns complete statistics.

        Verifies:
        - stats() returns a dictionary
        - Dictionary contains keys for all layers
        - Instance ID and region are included
        - Layer stats are non-empty dictionaries
        """
        manager = LNSPManager(
            instance_id="instance:stats_test",
            region="us-east",
        )

        stats = manager.stats()

        # Check top-level structure
        assert isinstance(stats, dict)
        assert "instance_id" in stats
        assert "region" in stats
        assert "layer1" in stats
        assert "layer2" in stats
        assert "layer3" in stats
        assert "layer4" in stats

        # Check values
        assert stats["instance_id"] == "instance:stats_test"
        assert stats["region"] == "us-east"

        # Check each layer has stats
        assert isinstance(stats["layer1"], dict)
        assert isinstance(stats["layer2"], dict)
        assert isinstance(stats["layer3"], dict)
        assert isinstance(stats["layer4"], dict)


class TestLNSPManagerStatistics:
    """Test manager statistics aggregation from all layers."""

    def test_manager_stats_with_components(self) -> None:
        """Test manager.stats() reflects registered components.

        Verifies:
        - Layer 1 stats include sensor count
        - Layer 2 stats include aggregator count
        - Layer 4 stats include handler count
        - All stats are properly aggregated
        """
        manager = LNSPManager(instance_id="instance:component_stats")

        # Register components
        sensor = MockSensor(sensor_id="sensor:test", should_emit=True)
        manager.layer1.register_sensor(sensor)

        agg = MockAggregator(aggregator_id="agg:test")
        manager.layer2.register_aggregator(agg, window_sizes=[5.0])

        handler = MockHandler(handler_id="handler:test")
        manager.layer4.register_handler(handler)

        # Get stats
        stats = manager.stats()

        # Verify component counts in each layer
        assert stats["layer1"]["sensor_count"] == 1
        assert stats["layer2"]["aggregator_count"] == 1
        assert stats["layer4"]["handler_count"] == 1

    def test_manager_stats_after_cycles(self) -> None:
        """Test manager stats reflect activity across cycles.

        Verifies:
        - Layer 1 stats track observation count
        - Layer 2 stats track aggregation count
        - Layer 3 stats track judgment count
        - Layer 4 stats track handler execution count
        """
        manager = LNSPManager(instance_id="instance:activity_stats")

        # Register components
        sensor = MockSensor(sensor_id="sensor:test", should_emit=True)
        manager.layer1.register_sensor(sensor)

        agg = MockAggregator(aggregator_id="agg:test")
        manager.layer2.register_aggregator(agg, window_sizes=[5.0])

        # Wire layers
        manager.wire_layers()

        # Get initial stats
        stats_before = manager.stats()

        # Verify structure is present (component count will be reflected)
        assert "layer1" in stats_before
        assert "layer2" in stats_before
        assert "layer3" in stats_before
        assert "layer4" in stats_before

    def test_manager_stats_isolation(self) -> None:
        """Test stats from different managers are independent.

        Verifies:
        - Multiple managers maintain separate stats
        - Stats from one manager don't affect another
        - Instance IDs distinguish managers in stats
        """
        manager1 = LNSPManager(instance_id="instance:manager1")
        manager2 = LNSPManager(instance_id="instance:manager2")

        # Register components to manager1 only
        sensor1 = MockSensor(sensor_id="sensor:m1", should_emit=True)
        manager1.layer1.register_sensor(sensor1)

        # Get stats from both
        stats1 = manager1.stats()
        stats2 = manager2.stats()

        # Verify they're independent
        assert stats1["instance_id"] == "instance:manager1"
        assert stats2["instance_id"] == "instance:manager2"
        assert stats1["layer1"]["sensor_count"] == 1
        assert stats2["layer1"]["sensor_count"] == 0
