"""Test Sensor interface â€” contract validation."""

import asyncio

import pytest

from cynic.kernel.organism.perception.senses.sensor_interface import Observation, Sensor


class MockSensor(Sensor):
    """Test implementation of Sensor interface."""

    def __init__(self, sensor_id: str = "test_sensor"):
        self._id = sensor_id
        self._call_count = 0

    @property
    def sensor_id(self) -> str:
        return self._id

    async def startup(self) -> None:
        """Startup â€” no-op for mock."""
        self._call_count = 0

    async def perceive(self) -> Observation | None:
        """Return mock observation every other call."""
        self._call_count += 1
        if self._call_count % 2 == 0:
            return Observation(
                sensor_id=self.sensor_id,
                timestamp=asyncio.get_event_loop().time(),
                data={"count": self._call_count},
                quality=0.9,
            )
        return None

    async def shutdown(self) -> None:
        """Shutdown â€” no-op for mock."""
        pass


@pytest.mark.asyncio
async def test_sensor_interface_contract():
    """Verify Sensor interface works as designed."""
    sensor = MockSensor(sensor_id="disk_monitor")  # Non-synthetic sensor ID

    # Startup
    await sensor.startup()
    assert sensor.sensor_id == "disk_monitor"

    # First perceive returns None
    obs1 = await sensor.perceive()
    assert obs1 is None

    # Second perceive returns Observation
    obs2 = await sensor.perceive()
    assert obs2 is not None
    assert obs2.sensor_id == "disk_monitor"
    assert obs2.quality == 0.9
    assert obs2.data["count"] == 2

    # Test is_synthetic
    assert not obs2.is_synthetic()

    # Shutdown
    await sensor.shutdown()


@pytest.mark.asyncio
async def test_synthetic_sensor_detection():
    """Verify synthetic sensor detection."""
    obs_real = Observation(
        sensor_id="disk",
        timestamp=0.0,
        data={},
        quality=1.0,
    )
    assert not obs_real.is_synthetic()

    obs_synthetic = Observation(
        sensor_id="synthetic_empirical",
        timestamp=0.0,
        data={},
        quality=0.8,
    )
    assert obs_synthetic.is_synthetic()

    obs_mock = Observation(
        sensor_id="mock_test_sensor",
        timestamp=0.0,
        data={},
        quality=0.8,
    )
    assert obs_mock.is_synthetic()


@pytest.mark.asyncio
async def test_check_available():
    """Verify check_available() works."""
    sensor = MockSensor()
    await sensor.startup()
    available = await sensor.check_available()
    # Will be True or False depending on perceive result
    assert isinstance(available, bool)
    await sensor.shutdown()


@pytest.mark.asyncio
async def test_stats():
    """Verify stats() returns dict."""
    sensor = MockSensor(sensor_id="stats_test")
    stats = sensor.stats()
    assert isinstance(stats, dict)
    assert stats["sensor_id"] == "stats_test"
    assert "timestamp" in stats
