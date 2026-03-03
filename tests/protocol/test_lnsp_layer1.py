"""Comprehensive tests for LNSP Layer 1 sensor interface and ringbuffer."""

from __future__ import annotations

import pytest

from cynic.kernel.protocol.lnsp.layer1 import Layer1, Sensor
from cynic.kernel.protocol.lnsp.messages import create_raw_observation
from cynic.kernel.protocol.lnsp.ringbuffer import Ringbuffer
from cynic.kernel.protocol.lnsp.types import LNSPMessage, ObservationType

# ============================================================================
# Test Fixtures
# ============================================================================


class MockSensor(Sensor):
    """Mock sensor that returns preset observations."""

    def __init__(self, sensor_id: str, observations: list[LNSPMessage | None]):
        super().__init__(sensor_id)
        self.observations = observations
        self.index = 0
        self.call_count = 0

    async def observe(self) -> LNSPMessage | None:
        """Return next observation in sequence."""
        self.call_count += 1
        if self.index >= len(self.observations):
            return None
        msg = self.observations[self.index]
        self.index += 1
        return msg


@pytest.fixture
def sample_message():
    """Create a sample LNSPMessage for testing."""
    return create_raw_observation(
        observation_type=ObservationType.METRIC_SAMPLE,
        data={"cpu": 45.2, "memory": 78.9},
        source="TEST_SENSOR",
        instance_id="test_instance",
    )


@pytest.fixture
def sample_messages(sample_message):
    """Create multiple sample messages."""
    return [
        sample_message,
        create_raw_observation(
            observation_type=ObservationType.PROCESS_CREATED,
            data={"process_id": "p001", "name": "worker"},
            source="TEST_SENSOR",
            instance_id="test_instance",
        ),
        create_raw_observation(
            observation_type=ObservationType.PROCESS_TERMINATED,
            data={"process_id": "p001"},
            source="TEST_SENSOR",
            instance_id="test_instance",
        ),
    ]


# ============================================================================
# TestRingbuffer â€” Circular Buffer Tests
# ============================================================================


class TestRingbuffer:
    """Test the Ringbuffer generic circular buffer implementation."""

    def test_ringbuffer_creation(self):
        """Test creating a ringbuffer with capacity."""
        rb = Ringbuffer[int](capacity=5)
        assert rb.is_empty()
        assert not rb.is_full()
        assert rb.size() == 0

    def test_ringbuffer_put_single_item(self):
        """Test adding a single item."""
        rb = Ringbuffer[int](capacity=3)
        rb.put(1)
        assert not rb.is_empty()
        assert rb.size() == 1
        assert not rb.is_full()

    def test_ringbuffer_put_fill_to_capacity(self):
        """Test filling buffer to capacity."""
        rb = Ringbuffer[int](capacity=3)
        rb.put(1)
        rb.put(2)
        rb.put(3)
        assert rb.is_full()
        assert rb.size() == 3

    def test_ringbuffer_get_removes_oldest(self):
        """Test get() removes and returns oldest item."""
        rb = Ringbuffer[int](capacity=3)
        rb.put(1)
        rb.put(2)
        rb.put(3)

        assert rb.get() == 1
        assert rb.size() == 2
        assert rb.get() == 2
        assert rb.size() == 1
        assert rb.get() == 3
        assert rb.size() == 0
        assert rb.is_empty()

    def test_ringbuffer_peek_doesnt_remove(self):
        """Test peek() returns oldest without removing."""
        rb = Ringbuffer[int](capacity=3)
        rb.put(1)
        rb.put(2)
        rb.put(3)

        assert rb.peek() == 1
        assert rb.size() == 3
        assert rb.peek() == 1  # Still there
        assert rb.size() == 3

    def test_ringbuffer_get_empty_returns_none(self):
        """Test get() on empty buffer returns None."""
        rb = Ringbuffer[int](capacity=3)
        assert rb.get() is None

    def test_ringbuffer_peek_empty_returns_none(self):
        """Test peek() on empty buffer returns None."""
        rb = Ringbuffer[int](capacity=3)
        assert rb.peek() is None

    def test_ringbuffer_overflow_drops_oldest(self):
        """Test that overflow drops oldest item."""
        rb = Ringbuffer[int](capacity=3)
        rb.put(1)
        rb.put(2)
        rb.put(3)
        assert rb.is_full()

        # This should overwrite the oldest (1)
        rb.put(4)
        assert rb.size() == 3
        assert rb.is_full()

        # Check that 1 is gone
        assert rb.get() == 2
        assert rb.get() == 3
        assert rb.get() == 4
        assert rb.is_empty()

    def test_ringbuffer_multiple_overflows(self):
        """Test multiple overflow cycles."""
        rb = Ringbuffer[int](capacity=2)
        rb.put(1)
        rb.put(2)
        rb.put(3)  # Overwrites 1
        rb.put(4)  # Overwrites 2
        rb.put(5)  # Overwrites 3

        assert rb.get() == 4
        assert rb.get() == 5
        assert rb.is_empty()

    def test_ringbuffer_drain_empties_all(self):
        """Test drain() removes and returns all items."""
        rb = Ringbuffer[int](capacity=5)
        rb.put(1)
        rb.put(2)
        rb.put(3)

        items = rb.drain()
        assert items == [1, 2, 3]
        assert rb.is_empty()
        assert rb.size() == 0

    def test_ringbuffer_drain_empty_returns_empty_list(self):
        """Test drain() on empty buffer returns empty list."""
        rb = Ringbuffer[int](capacity=3)
        assert rb.drain() == []

    def test_ringbuffer_generic_with_complex_types(self, sample_message):
        """Test ringbuffer works with complex types."""
        rb = Ringbuffer[LNSPMessage](capacity=3)
        rb.put(sample_message)
        assert rb.size() == 1
        retrieved = rb.get()
        assert retrieved == sample_message


# ============================================================================
# TestSensorInterface â€” Sensor Abstract Base Class
# ============================================================================


class TestSensorInterface:
    """Test the Sensor abstract base class."""

    def test_sensor_has_sensor_id(self):
        """Test sensor stores sensor_id."""
        mock = MockSensor("sensor_001", [])
        assert mock.sensor_id == "sensor_001"

    def test_sensor_default_instance_id(self):
        """Test sensor has default instance_id."""
        mock = MockSensor("sensor_001", [])
        assert mock.instance_id == "instance:local"

    def test_sensor_custom_instance_id(self):
        """Test sensor can override instance_id."""
        mock = MockSensor("sensor_001", [])
        mock.instance_id = "org_123"
        assert mock.instance_id == "org_123"

    @pytest.mark.asyncio
    async def test_sensor_observe_returns_none_when_exhausted(self):
        """Test sensor observe() returns None when no more observations."""
        mock = MockSensor("sensor_001", [])
        result = await mock.observe()
        assert result is None

    @pytest.mark.asyncio
    async def test_mock_sensor_returns_observations(self, sample_messages):
        """Test mock sensor returns preset observations."""
        mock = MockSensor("sensor_001", sample_messages)
        assert await mock.observe() == sample_messages[0]
        assert await mock.observe() == sample_messages[1]
        assert await mock.observe() == sample_messages[2]
        assert await mock.observe() is None


# ============================================================================
# TestLayer1SensorRegistration â€” Sensor Registration
# ============================================================================


class TestLayer1SensorRegistration:
    """Test registering sensors with Layer1."""

    def test_layer1_creation(self):
        """Test creating Layer1 manager."""
        layer1 = Layer1()
        assert layer1.ringbuffer is not None
        assert layer1.ringbuffer.size() == 0
        assert len(layer1.sensors) == 0
        assert len(layer1.subscribers) == 0

    def test_layer1_register_single_sensor(self):
        """Test registering a single sensor."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", [])
        layer1.register_sensor(sensor)

        assert "sensor_001" in layer1.sensors
        assert layer1.sensors["sensor_001"] is sensor

    def test_layer1_register_multiple_sensors(self):
        """Test registering multiple sensors."""
        layer1 = Layer1()
        sensor1 = MockSensor("sensor_001", [])
        sensor2 = MockSensor("sensor_002", [])

        layer1.register_sensor(sensor1)
        layer1.register_sensor(sensor2)

        assert len(layer1.sensors) == 2
        assert layer1.sensors["sensor_001"] is sensor1
        assert layer1.sensors["sensor_002"] is sensor2

    def test_layer1_register_overwrites_same_id(self):
        """Test registering sensor with same ID overwrites."""
        layer1 = Layer1()
        sensor1 = MockSensor("sensor_001", [])
        sensor2 = MockSensor("sensor_001", [])

        layer1.register_sensor(sensor1)
        layer1.register_sensor(sensor2)

        assert len(layer1.sensors) == 1
        assert layer1.sensors["sensor_001"] is sensor2


# ============================================================================
# TestLayer1Observation â€” Observation Collection
# ============================================================================


class TestLayer1Observation:
    """Test collecting observations from sensors."""

    @pytest.mark.asyncio
    async def test_layer1_observe_single_sensor(self, sample_message):
        """Test observing from single sensor."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", [sample_message])

        layer1.register_sensor(sensor)
        await layer1.observe()

        assert layer1.ringbuffer.size() == 1
        assert layer1.ringbuffer.peek() == sample_message

    @pytest.mark.asyncio
    async def test_layer1_observe_multiple_sensors(self, sample_messages):
        """Test observing from multiple sensors."""
        layer1 = Layer1()
        sensor1 = MockSensor("sensor_001", [sample_messages[0]])
        sensor2 = MockSensor("sensor_002", [sample_messages[1]])

        layer1.register_sensor(sensor1)
        layer1.register_sensor(sensor2)
        await layer1.observe()

        # Both sensors called once during single observe() call
        assert layer1.ringbuffer.size() == 2

    @pytest.mark.asyncio
    async def test_layer1_observe_ignores_none_results(self, sample_message):
        """Test that None observations are ignored."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", [sample_message, None, None])

        layer1.register_sensor(sensor)
        await layer1.observe()

        assert layer1.ringbuffer.size() == 1

    @pytest.mark.asyncio
    async def test_layer1_observe_preserves_order(self, sample_messages):
        """Test observations are stored in order."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", sample_messages)

        layer1.register_sensor(sensor)
        # Need to call observe() 3 times to get all 3 messages
        await layer1.observe()
        await layer1.observe()
        await layer1.observe()

        assert layer1.ringbuffer.get() == sample_messages[0]
        assert layer1.ringbuffer.get() == sample_messages[1]
        assert layer1.ringbuffer.get() == sample_messages[2]

    @pytest.mark.asyncio
    async def test_layer1_peek_without_removing(self, sample_message):
        """Test peek() doesn't remove observation."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", [sample_message])

        layer1.register_sensor(sensor)
        await layer1.observe()

        peeked = layer1.peek()
        assert peeked == sample_message
        assert layer1.ringbuffer.size() == 1

        gotten = layer1.get()
        assert gotten == sample_message
        assert layer1.ringbuffer.size() == 0

    @pytest.mark.asyncio
    async def test_layer1_get_removes_observation(self, sample_message):
        """Test get() removes observation."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", [sample_message])

        layer1.register_sensor(sensor)
        await layer1.observe()

        assert layer1.ringbuffer.size() == 1
        retrieved = layer1.get()
        assert retrieved == sample_message
        assert layer1.ringbuffer.size() == 0


# ============================================================================
# TestLayer1Subscription â€” Subscription Pattern
# ============================================================================


class TestLayer1Subscription:
    """Test subscription pattern for Layer 2 callbacks."""

    def test_layer1_subscribe_callback(self):
        """Test subscribing a callback."""
        layer1 = Layer1()

        def callback(msg: LNSPMessage) -> None:
            pass

        layer1.subscribe(callback)
        assert len(layer1.subscribers) == 1
        assert callback in layer1.subscribers

    def test_layer1_subscribe_multiple_callbacks(self):
        """Test subscribing multiple callbacks."""
        layer1 = Layer1()

        def callback1(msg: LNSPMessage) -> None:
            pass

        def callback2(msg: LNSPMessage) -> None:
            pass

        layer1.subscribe(callback1)
        layer1.subscribe(callback2)

        assert len(layer1.subscribers) == 2
        assert callback1 in layer1.subscribers
        assert callback2 in layer1.subscribers

    @pytest.mark.asyncio
    async def test_layer1_observe_calls_subscribers(self, sample_message):
        """Test observe() calls all subscriber callbacks."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", [sample_message])

        received_messages = []

        def callback(msg):
            received_messages.append(msg)

        layer1.register_sensor(sensor)
        layer1.subscribe(callback)
        await layer1.observe()

        assert len(received_messages) == 1
        assert received_messages[0] == sample_message

    @pytest.mark.asyncio
    async def test_layer1_observe_calls_all_subscribers(self, sample_messages):
        """Test observe() calls all subscribers with each message."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", sample_messages)

        received1 = []
        received2 = []

        def callback1(msg):
            received1.append(msg)

        def callback2(msg):
            received2.append(msg)

        layer1.register_sensor(sensor)
        layer1.subscribe(callback1)
        layer1.subscribe(callback2)

        # Call observe() 3 times to get all observations
        await layer1.observe()
        await layer1.observe()
        await layer1.observe()

        assert len(received1) == 3
        assert len(received2) == 3
        assert received1 == sample_messages
        assert received2 == sample_messages

    @pytest.mark.asyncio
    async def test_layer1_subscribers_ignore_none_observations(self, sample_message):
        """Test subscribers don't receive None observations."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", [sample_message, None])

        received = []

        def callback(msg):
            received.append(msg)

        layer1.register_sensor(sensor)
        layer1.subscribe(callback)
        await layer1.observe()

        assert len(received) == 1
        assert received[0] == sample_message


# ============================================================================
# TestLayer1Overflow â€” Overflow and Backpressure
# ============================================================================


class TestLayer1Overflow:
    """Test ringbuffer overflow behavior."""

    def test_layer1_ringbuffer_capacity(self):
        """Test Layer1 ringbuffer has correct default capacity."""
        layer1 = Layer1()
        stats = layer1.stats()
        assert stats["capacity"] == 10000

    @pytest.mark.asyncio
    async def test_layer1_overflow_drops_oldest(self, sample_message):
        """Test overflow drops oldest message."""
        # Create Layer1 with small capacity for testing
        layer1 = Layer1(capacity=2)

        # Create multiple messages
        messages = [
            create_raw_observation(
                observation_type=ObservationType.METRIC_SAMPLE,
                data={"value": i},
                source="TEST",
                instance_id="test",
            )
            for i in range(4)
        ]

        sensor = MockSensor("sensor_001", messages)
        layer1.register_sensor(sensor)

        # Call observe() 4 times to process all 4 messages
        for _ in range(4):
            await layer1.observe()

        # After overflow, only last 2 messages should be available
        assert layer1.ringbuffer.size() == 2
        retrieved = layer1.get()
        assert retrieved.payload["value"] == 2  # Third message
        retrieved = layer1.get()
        assert retrieved.payload["value"] == 3  # Fourth message

    @pytest.mark.asyncio
    async def test_layer1_data_still_accessible_after_overflow(self, sample_message):
        """Test data is accessible even after overflow."""
        layer1 = Layer1(capacity=1)

        messages = [
            create_raw_observation(
                observation_type=ObservationType.METRIC_SAMPLE,
                data={"value": i},
                source="TEST",
                instance_id="test",
            )
            for i in range(3)
        ]

        sensor = MockSensor("sensor_001", messages)
        layer1.register_sensor(sensor)

        # Call observe() 3 times to process all 3 messages
        for _ in range(3):
            await layer1.observe()

        # Last message should still be accessible
        assert layer1.ringbuffer.size() == 1
        retrieved = layer1.get()
        assert retrieved.payload["value"] == 2


# ============================================================================
# TestLayer1Stats â€” Statistics
# ============================================================================


class TestLayer1Stats:
    """Test statistics reporting."""

    def test_layer1_stats_empty(self):
        """Test stats on empty Layer1."""
        layer1 = Layer1()
        stats = layer1.stats()

        assert "capacity" in stats
        assert "current_size" in stats
        assert "is_full" in stats
        assert "sensor_count" in stats
        assert "subscriber_count" in stats

        assert stats["capacity"] == 10000
        assert stats["current_size"] == 0
        assert stats["is_full"] is False
        assert stats["sensor_count"] == 0
        assert stats["subscriber_count"] == 0

    @pytest.mark.asyncio
    async def test_layer1_stats_with_data(self, sample_messages):
        """Test stats with data in buffer and subscribers."""
        layer1 = Layer1()
        sensor = MockSensor("sensor_001", sample_messages)
        layer1.register_sensor(sensor)
        layer1.subscribe(lambda msg: None)
        layer1.subscribe(lambda msg: None)

        # Call observe() 3 times to get all messages
        await layer1.observe()
        await layer1.observe()
        await layer1.observe()

        stats = layer1.stats()
        assert stats["current_size"] == 3
        assert stats["sensor_count"] == 1
        assert stats["subscriber_count"] == 2
        assert stats["is_full"] is False

    def test_layer1_stats_capacity_custom(self):
        """Test stats reflect custom capacity."""
        layer1 = Layer1(capacity=100)
        stats = layer1.stats()
        assert stats["capacity"] == 100

    @pytest.mark.asyncio
    async def test_layer1_stats_is_full(self):
        """Test is_full reflects buffer state."""
        layer1 = Layer1(capacity=2)
        messages = [
            create_raw_observation(
                observation_type=ObservationType.METRIC_SAMPLE,
                data={"value": i},
                source="TEST",
                instance_id="test",
            )
            for i in range(2)
        ]

        sensor = MockSensor("sensor_001", messages)
        layer1.register_sensor(sensor)

        # Call observe() 2 times to fill the 2-capacity buffer
        await layer1.observe()
        await layer1.observe()

        stats = layer1.stats()
        assert stats["is_full"] is True
        assert stats["current_size"] == 2
