"""Regional Coordinator for LNSP â€" Multi-Instance Coordination

The Regional Coordinator enables LNSP to scale from single-machine to multi-instance
deployment by:

1. Accepting Layer 2 aggregated state from multiple LNSP instances
2. De-duplicating messages (same message_id from different instances)
3. Correlating related observations using temporal windows
4. Forwarding unified, deduplicated state to a central Judge

This enables distributed governance where multiple organism instances share a
unified view of ecosystem state.
"""

from __future__ import annotations

import time
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from .layer2 import TemporalWindow
from .types import LNSPMessage


@dataclass
class InstanceConnection:
    """Manages connection to a single LNSP instance.

    Each instance connection tracks the health and message flow from a remote
    LNSP instance, including heartbeat monitoring and message receipt.

    Attributes:
        instance_id: Unique identifier for the LNSP instance
        host: Hostname or IP address of the instance
        port: Port number for connection
        coordinator: Reference to parent RegionalCoordinator
        last_heartbeat: Unix timestamp of last received heartbeat
        is_healthy: Whether the instance is currently healthy
        message_count: Total messages received from this instance
    """

    instance_id: str
    host: str
    port: int
    coordinator: RegionalCoordinator
    last_heartbeat: float = field(default_factory=time.time)
    is_healthy: bool = True
    message_count: int = 0

    async def connect(self) -> None:
        """Establish TCP connection to instance.

        In the current MVP implementation, this is a placeholder.
        A full implementation would establish an actual TCP socket connection.
        """
        # MVP: Connection tracking is managed at the coordinator level
        # Full implementation would handle TCP socket creation here
        pass

    async def send_heartbeat(self) -> None:
        """Send keepalive heartbeat.

        Updates the last_heartbeat timestamp. In a full implementation,
        this would send an actual keepalive message to the instance.
        """
        self.last_heartbeat = time.time()

    async def receive_message(self) -> LNSPMessage | None:
        """Receive next message from instance.

        In the MVP implementation, messages are pushed to the coordinator
        via process_aggregation(). A full TCP implementation would block
        on socket recv() here.

        Returns:
            Next available LNSPMessage or None if no message ready
        """
        # MVP: Messages are pushed to coordinator, not pulled from instance
        return None

    def is_alive(self) -> bool:
        """Check if instance heartbeat is recent.

        Returns True if the last heartbeat was within the coordinator's
        instance_timeout_sec window.

        Returns:
            True if instance appears healthy, False if timeout exceeded
        """
        now = time.time()
        timeout_sec = self.coordinator.instance_timeout_sec
        return (now - self.last_heartbeat) <= timeout_sec

    async def close(self) -> None:
        """Close connection to instance.

        In the MVP, this marks the instance as unhealthy.
        A full TCP implementation would close the socket.
        """
        self.is_healthy = False


@dataclass
class RegionalCoordinator:
    """Coordinates LNSP instances across a region.

    The Regional Coordinator receives Layer 2 aggregated state messages from
    multiple LNSP instances, de-duplicates them based on message_id, correlates
    related observations using temporal windows, and forwards unified state
    to subscribed judges/handlers.

    Attributes:
        coordinator_id: Unique identifier for this coordinator
        instance_timeout_sec: Heartbeat timeout in seconds (default: 30.0)
        dedup_window_sec: De-duplication window in seconds (default: 5.0)
        _instances: Dict mapping instance_id to InstanceConnection
        _message_cache: Dict mapping message_id to timestamp for de-duplication
        _correlation_window: TemporalWindow for correlating observations
        _subscribers: List of callbacks to receive deduplicated messages
    """

    coordinator_id: str = "regional:coordinator"
    instance_timeout_sec: float = 30.0
    dedup_window_sec: float = 5.0

    # Internal state
    _instances: dict[str, InstanceConnection] = field(default_factory=dict)
    _message_cache: dict[str, float] = field(default_factory=dict)
    _correlation_window: TemporalWindow = field(
        default_factory=lambda: TemporalWindow(window_size_sec=5.0)
    )
    _subscribers: list[Callable[[LNSPMessage], None]] = field(default_factory=list)

    async def register_instance(self, instance_id: str, host: str, port: int) -> None:
        """Register a new LNSP instance to coordinate with.

        Creates an InstanceConnection for the given instance and begins
        tracking it for health monitoring and message receipt.

        Args:
            instance_id: Unique identifier for the instance
            host: Hostname or IP address
            port: Port number for connection
        """
        if instance_id in self._instances:
            raise ValueError(f"Instance {instance_id} already registered")

        connection = InstanceConnection(
            instance_id=instance_id,
            host=host,
            port=port,
            coordinator=self,
        )
        self._instances[instance_id] = connection
        await connection.connect()

    async def listen_from_instance(self, instance_id: str) -> None:
        """Listen for Layer 2 messages from an instance.

        In the MVP implementation, this is a placeholder for the async
        listening task. A full implementation would run a receive loop
        on the TCP connection and call process_aggregation() for each
        message received.

        Args:
            instance_id: Instance to listen from

        Raises:
            ValueError: If instance_id is not registered
        """
        if instance_id not in self._instances:
            raise ValueError(f"Instance {instance_id} not registered")

        connection = self._instances[instance_id]

        # MVP: In a full implementation, this would:
        # - Loop on socket.recv()
        # - Call process_aggregation() for each message
        # - Handle disconnection and reconnection
        # For now, this is a placeholder for testing the interface
        await connection.send_heartbeat()

    async def process_aggregation(self, msg: LNSPMessage) -> None:
        """Process Layer 2 aggregation from an instance.

        Implements the core de-duplication and correlation logic:
        1. Check if message_id already seen (skip if duplicate)
        2. Add to message cache and correlation window
        3. Forward to subscribers if unique

        Args:
            msg: Layer 2 aggregated state message to process
        """
        # De-duplication: Check if we've seen this message_id recently
        if self._is_duplicate(msg):
            return

        # Record message in cache for de-duplication
        self._message_cache[msg.header.message_id] = time.time()

        # Add to correlation window for grouping related observations
        self._correlation_window.add(msg)

        # Update instance heartbeat if we know the source
        instance_id = msg.metadata.instance_id
        if instance_id in self._instances:
            await self._instances[instance_id].send_heartbeat()
            self._instances[instance_id].message_count += 1

        # Forward to all subscribers
        for callback in self._subscribers:
            callback(msg)

    def _is_duplicate(self, msg: LNSPMessage) -> bool:
        """Check if message_id seen in last dedup_window_sec.

        Uses the message cache to detect duplicates. If the message_id
        is in the cache and within the dedup window, returns True.
        Cleans up old entries from the cache as a side effect.

        Args:
            msg: Message to check for duplication

        Returns:
            True if message is a duplicate, False if unique
        """
        now = time.time()
        msg_id = msg.header.message_id

        # Clean up old cache entries
        self._message_cache = {
            mid: ts for mid, ts in self._message_cache.items() if now - ts <= self.dedup_window_sec
        }

        # Check if this message_id is in the cache
        if msg_id in self._message_cache:
            return True

        return False

    def subscribe(self, callback: Callable[[LNSPMessage], None]) -> None:
        """Subscribe to de-duplicated, correlated aggregations.

        Adds a callback to the subscriber list. Callbacks are invoked
        synchronously for each unique aggregation message that passes
        de-duplication checks.

        Args:
            callback: Callable that takes an LNSPMessage
        """
        self._subscribers.append(callback)

    async def unregister_instance(self, instance_id: str) -> None:
        """Unregister and disconnect from an instance.

        Marks the instance as unhealthy and removes it from the active
        instance tracking.

        Args:
            instance_id: Instance to unregister

        Raises:
            ValueError: If instance_id is not registered
        """
        if instance_id not in self._instances:
            raise ValueError(f"Instance {instance_id} not registered")

        connection = self._instances[instance_id]
        await connection.close()
        del self._instances[instance_id]

    async def check_instance_health(self) -> None:
        """Check health of all registered instances.

        Iterates through all instances and marks any with expired heartbeats
        as unhealthy. This should be called periodically in an event loop.
        """
        unhealthy_ids = []
        for instance_id, connection in self._instances.items():
            if not connection.is_alive():
                connection.is_healthy = False
                unhealthy_ids.append(instance_id)

        # Log unhealthy instances for debugging (can be replaced with logging)
        if unhealthy_ids:
            # Could emit an alert or trigger reconnection logic here
            pass

    def stats(self) -> dict[str, Any]:
        """Return coordinator statistics.

        Returns comprehensive statistics about the coordinator state,
        including instance count, message cache size, correlation window
        state, and subscriber count.

        Returns:
            Dict with coordinator statistics
        """
        healthy_count = sum(1 for conn in self._instances.values() if conn.is_healthy)
        total_messages = sum(conn.message_count for conn in self._instances.values())

        return {
            "coordinator_id": self.coordinator_id,
            "instance_count": len(self._instances),
            "healthy_instances": healthy_count,
            "message_cache_size": len(self._message_cache),
            "total_messages_processed": total_messages,
            "correlation_window_size": len(self._correlation_window.observations),
            "subscriber_count": len(self._subscribers),
            "dedup_window_sec": self.dedup_window_sec,
            "instance_timeout_sec": self.instance_timeout_sec,
        }
