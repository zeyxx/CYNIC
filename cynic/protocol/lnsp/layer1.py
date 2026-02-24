"""Layer 1: Raw Observation Collection

Layer 1 collects raw telemetry from sensors and buffers them for Layer 2
processing. It provides:

1. Sensor abstract base class for observation producers
2. Layer1 manager coordinating sensors and subscribers
3. Ringbuffer for overflow-safe buffering
4. Subscription pattern for Layer 2 callbacks
"""
from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, Callable

from .ringbuffer import Ringbuffer
from .types import LNSPMessage


class Sensor(ABC):
    """Abstract base class for observation producers.

    Sensors emit LNSPMessage observations at Layer 1 (RAW). Each sensor
    has a unique ID and instance ID that tags all observations.

    Attributes:
        sensor_id: Unique identifier for this sensor
        instance_id: Organism instance ID (tags observations)
    """

    def __init__(self, sensor_id: str, instance_id: str = "instance:local") -> None:
        """Initialize sensor with ID.

        Args:
            sensor_id: Unique identifier for this sensor
            instance_id: Organism instance ID (default: "instance:local")
        """
        self.sensor_id = sensor_id
        self.instance_id = instance_id

    @abstractmethod
    async def observe(self) -> LNSPMessage | None:
        """Emit a single observation or None if no observation available.

        This method should be called repeatedly to collect telemetry.
        Return None to indicate no observation is available at this time.

        Returns:
            LNSPMessage observation, or None if no observation available
        """
        pass


@dataclass
class Layer1:
    """Layer 1 manager for raw observation collection.

    Layer1 coordinates registered sensors, buffers observations in a
    ringbuffer, and distributes them to Layer 2 subscribers via callbacks.

    The ringbuffer provides natural backpressure: when full, oldest
    observations are dropped, preventing unbounded memory growth.

    Attributes:
        ringbuffer: Circular buffer for raw observations
        sensors: Dict mapping sensor_id to Sensor instances
        subscribers: List of Layer 2 callbacks to receive observations
    """

    ringbuffer: Ringbuffer[LNSPMessage]
    sensors: dict[str, Sensor]
    subscribers: list[Callable[[LNSPMessage], None]]

    def __init__(self, capacity: int = 10000) -> None:
        """Initialize Layer1 with ringbuffer capacity.

        Args:
            capacity: Ringbuffer capacity in observations (default: 10000)
        """
        self.ringbuffer = Ringbuffer[LNSPMessage](capacity=capacity)
        self.sensors: dict[str, Sensor] = {}
        self.subscribers: list[Callable[[LNSPMessage], None]] = []

    def register_sensor(self, sensor: Sensor) -> None:
        """Register a sensor to emit observations.

        If a sensor with the same ID already exists, it is replaced.

        Args:
            sensor: Sensor instance to register
        """
        self.sensors[sensor.sensor_id] = sensor

    def subscribe(self, callback: Callable[[LNSPMessage], None]) -> None:
        """Subscribe a callback to receive observations.

        Callbacks are called with each observation (in order) as they
        are collected. They are called synchronously during observe().

        Args:
            callback: Callable that takes an LNSPMessage observation
        """
        self.subscribers.append(callback)

    async def observe(self) -> None:
        """Run all sensors and collect observations.

        For each registered sensor:
        1. Call sensor.observe() to get an observation
        2. If not None, add to ringbuffer
        3. Call all subscriber callbacks with the observation

        This method should be called repeatedly in the main event loop.
        """
        for sensor in self.sensors.values():
            observation = await sensor.observe()
            if observation is not None:
                # Add to ringbuffer (may drop oldest if full)
                self.ringbuffer.put(observation)

                # Call all subscriber callbacks
                for callback in self.subscribers:
                    callback(observation)

    def peek(self) -> LNSPMessage | None:
        """Peek at oldest observation without removing.

        Returns:
            Oldest LNSPMessage in buffer, or None if empty
        """
        return self.ringbuffer.peek()

    def get(self) -> LNSPMessage | None:
        """Get and remove oldest observation from buffer.

        Returns:
            Oldest LNSPMessage in buffer, or None if empty
        """
        return self.ringbuffer.get()

    def stats(self) -> dict[str, Any]:
        """Return current statistics about Layer1 state.

        Returns:
            Dict with keys:
                - capacity: Ringbuffer capacity
                - current_size: Number of items in buffer
                - is_full: Whether buffer is at capacity
                - sensor_count: Number of registered sensors
                - subscriber_count: Number of subscribed callbacks
        """
        return {
            "capacity": self.ringbuffer.capacity,
            "current_size": self.ringbuffer.size(),
            "is_full": self.ringbuffer.is_full(),
            "sensor_count": len(self.sensors),
            "subscriber_count": len(self.subscribers),
        }
