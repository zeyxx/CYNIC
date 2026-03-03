"""Comprehensive tests for LNSP Regional Coordinator."""

from __future__ import annotations

import time

import pytest

from cynic.kernel.protocol.lnsp.messages import create_aggregated_state
from cynic.kernel.protocol.lnsp.regional_coordinator import (
    InstanceConnection,
    RegionalCoordinator,
)
from cynic.kernel.protocol.lnsp.types import AggregationType, LNSPMessage

# ============================================================================
# Test Fixtures
# ============================================================================


@pytest.fixture
def coordinator() -> RegionalCoordinator:
    """Create a fresh RegionalCoordinator for testing."""
    return RegionalCoordinator(
        coordinator_id="test:regional:coordinator",
        instance_timeout_sec=2.0,  # Short timeout for testing
        dedup_window_sec=1.0,  # Short window for testing
    )


@pytest.fixture
def sample_aggregation() -> LNSPMessage:
    """Create a sample Layer 2 aggregation message."""
    return create_aggregated_state(
        aggregation_type=AggregationType.SYSTEM_STATE,
        data={"health": "GOOD", "uptime": 86400},
        source="SYSTEM_GANGLIA",
        based_on=["obs_001", "obs_002"],
        instance_id="instance:001",
        region="north",
    )


@pytest.fixture
def sample_aggregation_alt() -> LNSPMessage:
    """Create an alternative sample aggregation (different instance)."""
    return create_aggregated_state(
        aggregation_type=AggregationType.SYSTEM_STATE,
        data={"health": "GOOD", "uptime": 86400},
        source="SYSTEM_GANGLIA",
        based_on=["obs_003", "obs_004"],
        instance_id="instance:002",
        region="south",
    )


# ============================================================================
# TestRegionalCoordinatorCreation
# ============================================================================


class TestRegionalCoordinatorCreation:
    """Test basic coordinator initialization."""

    async def test_coordinator_creation(self) -> None:
        """Test creating a RegionalCoordinator."""
        coordinator = RegionalCoordinator(coordinator_id="test:coord")
        assert coordinator.coordinator_id == "test:coord"
        assert coordinator.instance_timeout_sec == 30.0
        assert coordinator.dedup_window_sec == 5.0
        assert len(coordinator._instances) == 0
        assert len(coordinator._message_cache) == 0
        assert len(coordinator._subscribers) == 0

    async def test_coordinator_with_custom_timeouts(self) -> None:
        """Test creating coordinator with custom timeout values."""
        coordinator = RegionalCoordinator(
            coordinator_id="custom:coord",
            instance_timeout_sec=60.0,
            dedup_window_sec=10.0,
        )
        assert coordinator.instance_timeout_sec == 60.0
        assert coordinator.dedup_window_sec == 10.0


# ============================================================================
# TestInstanceManagement
# ============================================================================


class TestInstanceManagement:
    """Test instance registration, connection, and management."""

    async def test_register_instance(self, coordinator: RegionalCoordinator) -> None:
        """Test registering an LNSP instance."""
        await coordinator.register_instance("instance:001", "localhost", 5001)

        assert "instance:001" in coordinator._instances
        conn = coordinator._instances["instance:001"]
        assert conn.instance_id == "instance:001"
        assert conn.host == "localhost"
        assert conn.port == 5001
        assert conn.is_healthy is True

    async def test_register_multiple_instances(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test registering multiple instances."""
        await coordinator.register_instance("instance:001", "host1", 5001)
        await coordinator.register_instance("instance:002", "host2", 5002)
        await coordinator.register_instance("instance:003", "host3", 5003)

        assert len(coordinator._instances) == 3
        assert "instance:001" in coordinator._instances
        assert "instance:002" in coordinator._instances
        assert "instance:003" in coordinator._instances

    async def test_register_duplicate_instance_raises(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test that registering duplicate instance_id raises."""
        await coordinator.register_instance("instance:001", "host1", 5001)

        with pytest.raises(ValueError, match="already registered"):
            await coordinator.register_instance("instance:001", "host2", 5002)

    async def test_unregister_instance(self, coordinator: RegionalCoordinator) -> None:
        """Test unregistering an instance."""
        await coordinator.register_instance("instance:001", "localhost", 5001)
        assert "instance:001" in coordinator._instances

        await coordinator.unregister_instance("instance:001")
        assert "instance:001" not in coordinator._instances

    async def test_unregister_nonexistent_instance_raises(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test that unregistering non-existent instance raises."""
        with pytest.raises(ValueError, match="not registered"):
            await coordinator.unregister_instance("instance:does_not_exist")

    async def test_instance_connection_state(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test tracking instance connection state."""
        await coordinator.register_instance("instance:001", "localhost", 5001)
        conn = coordinator._instances["instance:001"]

        # Initially healthy
        assert conn.is_healthy is True
        assert conn.message_count == 0

        # Send heartbeat
        await conn.send_heartbeat()
        assert conn.last_heartbeat > 0

        # Check is_alive
        assert conn.is_alive() is True


# ============================================================================
# TestDeduplication
# ============================================================================


class TestDeduplication:
    """Test message de-duplication logic."""

    async def test_deduplication_same_message_id(
        self, coordinator: RegionalCoordinator, sample_aggregation: LNSPMessage
    ) -> None:
        """Test that duplicate messages (same message_id) are not forwarded twice."""
        forwarded_messages: list[LNSPMessage] = []

        def capture(msg: LNSPMessage) -> None:
            forwarded_messages.append(msg)

        coordinator.subscribe(capture)

        # Process the same message twice
        await coordinator.process_aggregation(sample_aggregation)
        assert len(forwarded_messages) == 1

        await coordinator.process_aggregation(sample_aggregation)
        assert len(forwarded_messages) == 1  # Not forwarded again (duplicate)

    async def test_deduplication_different_messages(
        self, coordinator: RegionalCoordinator, sample_aggregation: LNSPMessage
    ) -> None:
        """Test that different messages are both forwarded."""
        forwarded_messages: list[LNSPMessage] = []

        def capture(msg: LNSPMessage) -> None:
            forwarded_messages.append(msg)

        coordinator.subscribe(capture)

        # Process two different messages
        msg1 = sample_aggregation
        msg2 = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"health": "GOOD", "uptime": 86400},
            source="SYSTEM_GANGLIA",
            based_on=["obs_005"],
            instance_id="instance:001",
        )

        await coordinator.process_aggregation(msg1)
        assert len(forwarded_messages) == 1

        await coordinator.process_aggregation(msg2)
        assert len(forwarded_messages) == 2  # Both forwarded

    async def test_deduplication_window_expiry(
        self, coordinator: RegionalCoordinator, sample_aggregation: LNSPMessage
    ) -> None:
        """Test that messages are re-accepted after dedup window expires."""
        forwarded_messages: list[LNSPMessage] = []

        def capture(msg: LNSPMessage) -> None:
            forwarded_messages.append(msg)

        coordinator.subscribe(capture)

        # Process message first time
        await coordinator.process_aggregation(sample_aggregation)
        assert len(forwarded_messages) == 1

        # Try to process same message (should be duplicate)
        await coordinator.process_aggregation(sample_aggregation)
        assert len(forwarded_messages) == 1

        # Wait for dedup window to expire
        time.sleep(coordinator.dedup_window_sec + 0.1)

        # Process same message again (should be accepted now)
        await coordinator.process_aggregation(sample_aggregation)
        assert len(forwarded_messages) == 2  # Re-accepted after window

    async def test_is_duplicate_check(
        self, coordinator: RegionalCoordinator, sample_aggregation: LNSPMessage
    ) -> None:
        """Test _is_duplicate method directly."""
        assert coordinator._is_duplicate(sample_aggregation) is False

        # Add to cache manually
        coordinator._message_cache[sample_aggregation.header.message_id] = time.time()

        # Now it should be a duplicate
        assert coordinator._is_duplicate(sample_aggregation) is True

    async def test_message_cache_cleanup(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test that message cache is cleaned up after window expires."""
        msg1 = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"value": 1},
            source="SOURCE1",
            based_on=[],
            instance_id="inst1",
        )

        # Add to cache
        coordinator._message_cache[msg1.header.message_id] = time.time()
        assert len(coordinator._message_cache) == 1

        # Wait for window to expire
        time.sleep(coordinator.dedup_window_sec + 0.1)

        # Check for duplicate (which triggers cache cleanup)
        msg2 = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"value": 2},
            source="SOURCE2",
            based_on=[],
            instance_id="inst2",
        )
        coordinator._is_duplicate(msg2)

        # Old entry should be cleaned up
        assert len(coordinator._message_cache) == 0


# ============================================================================
# TestCorrelation
# ============================================================================


class TestCorrelation:
    """Test correlation window logic."""

    async def test_correlation_groups_observations(
        self, coordinator: RegionalCoordinator, sample_aggregation: LNSPMessage
    ) -> None:
        """Test that observations are added to correlation window."""
        assert len(coordinator._correlation_window.observations) == 0

        await coordinator.process_aggregation(sample_aggregation)
        assert len(coordinator._correlation_window.observations) == 1

        # Add another observation
        msg2 = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"health": "DEGRADED"},
            source="SYSTEM_GANGLIA",
            based_on=["obs_006"],
            instance_id="instance:001",
        )
        await coordinator.process_aggregation(msg2)
        assert len(coordinator._correlation_window.observations) == 2

    async def test_correlation_window_respects_time(
        self, coordinator: RegionalCoordinator, sample_aggregation: LNSPMessage
    ) -> None:
        """Test that correlation window expires old observations."""
        # Window size is 5 seconds by default in the fixture
        window_size_sec = 1.0  # Use short window for testing
        coordinator._correlation_window.window_size_sec = window_size_sec

        msg1 = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"value": 1},
            source="SOURCE1",
            based_on=[],
            instance_id="inst1",
        )
        await coordinator.process_aggregation(msg1)
        assert len(coordinator._correlation_window.observations) == 1

        # Wait for window to expire
        time.sleep(window_size_sec + 0.1)

        # Add new observation (old one should be expired)
        msg2 = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"value": 2},
            source="SOURCE2",
            based_on=[],
            instance_id="inst2",
        )
        await coordinator.process_aggregation(msg2)

        # Should have only the new message (old one expired)
        assert len(coordinator._correlation_window.observations) == 1
        assert coordinator._correlation_window.observations[0].payload["value"] == 2


# ============================================================================
# TestHeartbeatMonitoring
# ============================================================================


class TestHeartbeatMonitoring:
    """Test instance heartbeat monitoring."""

    async def test_heartbeat_monitoring(self, coordinator: RegionalCoordinator) -> None:
        """Test that heartbeats are tracked."""
        await coordinator.register_instance("instance:001", "localhost", 5001)
        conn = coordinator._instances["instance:001"]

        initial_heartbeat = conn.last_heartbeat
        await conn.send_heartbeat()

        # Heartbeat should be updated
        assert conn.last_heartbeat >= initial_heartbeat

    async def test_instance_marked_unhealthy_on_timeout(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test that instances are marked unhealthy when timeout expires."""
        await coordinator.register_instance("instance:001", "localhost", 5001)
        conn = coordinator._instances["instance:001"]

        # Initially healthy
        assert conn.is_healthy is True
        assert conn.is_alive() is True

        # Set heartbeat to past (simulate timeout)
        conn.last_heartbeat = time.time() - (coordinator.instance_timeout_sec + 1)

        # Should be marked as not alive
        assert conn.is_alive() is False

    async def test_check_instance_health(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test the check_instance_health method."""
        await coordinator.register_instance("instance:001", "localhost", 5001)
        await coordinator.register_instance("instance:002", "localhost", 5002)

        conn1 = coordinator._instances["instance:001"]
        conn2 = coordinator._instances["instance:002"]

        # Timeout conn1
        conn1.last_heartbeat = time.time() - (coordinator.instance_timeout_sec + 1)

        # Check health
        await coordinator.check_instance_health()

        # conn1 should be marked unhealthy
        assert conn1.is_healthy is False
        # conn2 should still be healthy
        assert conn2.is_healthy is True

    async def test_listen_from_instance(self, coordinator: RegionalCoordinator) -> None:
        """Test listening from an instance."""
        await coordinator.register_instance("instance:001", "localhost", 5001)

        # Should not raise
        await coordinator.listen_from_instance("instance:001")

        # Heartbeat should be updated
        conn = coordinator._instances["instance:001"]
        assert conn.last_heartbeat > 0

    async def test_listen_from_unregistered_instance_raises(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test that listening from unregistered instance raises."""
        with pytest.raises(ValueError, match="not registered"):
            await coordinator.listen_from_instance("instance:does_not_exist")


# ============================================================================
# TestForwarding
# ============================================================================


class TestForwarding:
    """Test subscriber forwarding."""

    async def test_unique_messages_forwarded_to_subscribers(
        self, coordinator: RegionalCoordinator, sample_aggregation: LNSPMessage
    ) -> None:
        """Test that unique messages are forwarded to all subscribers."""
        messages1: list[LNSPMessage] = []
        messages2: list[LNSPMessage] = []

        def subscriber1(msg: LNSPMessage) -> None:
            messages1.append(msg)

        def subscriber2(msg: LNSPMessage) -> None:
            messages2.append(msg)

        coordinator.subscribe(subscriber1)
        coordinator.subscribe(subscriber2)

        await coordinator.process_aggregation(sample_aggregation)

        # Both subscribers should receive the message
        assert len(messages1) == 1
        assert len(messages2) == 1
        assert messages1[0].header.message_id == sample_aggregation.header.message_id
        assert messages2[0].header.message_id == sample_aggregation.header.message_id

    async def test_duplicates_not_forwarded(
        self, coordinator: RegionalCoordinator, sample_aggregation: LNSPMessage
    ) -> None:
        """Test that duplicate messages are not forwarded."""
        forwarded_messages: list[LNSPMessage] = []

        def subscriber(msg: LNSPMessage) -> None:
            forwarded_messages.append(msg)

        coordinator.subscribe(subscriber)

        await coordinator.process_aggregation(sample_aggregation)
        await coordinator.process_aggregation(sample_aggregation)

        # Only first should be forwarded
        assert len(forwarded_messages) == 1

    async def test_multiple_different_messages_forwarded(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test that multiple different messages are all forwarded."""
        forwarded_messages: list[LNSPMessage] = []

        def subscriber(msg: LNSPMessage) -> None:
            forwarded_messages.append(msg)

        coordinator.subscribe(subscriber)

        # Create and process multiple unique messages
        for i in range(5):
            msg = create_aggregated_state(
                aggregation_type=AggregationType.SYSTEM_STATE,
                data={"value": i},
                source="SOURCE",
                based_on=[],
                instance_id=f"inst{i}",
            )
            await coordinator.process_aggregation(msg)

        # All should be forwarded
        assert len(forwarded_messages) == 5


# ============================================================================
# TestStatistics
# ============================================================================


class TestStatistics:
    """Test coordinator statistics reporting."""

    async def test_stats_reporting(self, coordinator: RegionalCoordinator) -> None:
        """Test stats() method returns comprehensive statistics."""
        await coordinator.register_instance("instance:001", "localhost", 5001)
        await coordinator.register_instance("instance:002", "localhost", 5002)

        coordinator.subscribe(lambda msg: None)

        stats = coordinator.stats()

        assert stats["coordinator_id"] == "test:regional:coordinator"
        assert stats["instance_count"] == 2
        assert stats["healthy_instances"] == 2
        assert stats["subscriber_count"] == 1
        assert stats["message_cache_size"] == 0
        assert stats["total_messages_processed"] == 0
        assert stats["dedup_window_sec"] == 1.0
        assert stats["instance_timeout_sec"] == 2.0

    async def test_stats_with_processed_messages(
        self,
        coordinator: RegionalCoordinator,
        sample_aggregation: LNSPMessage,
        sample_aggregation_alt: LNSPMessage,
    ) -> None:
        """Test stats after processing messages."""
        await coordinator.register_instance("instance:001", "localhost", 5001)
        await coordinator.register_instance("instance:002", "localhost", 5002)

        await coordinator.process_aggregation(sample_aggregation)
        await coordinator.process_aggregation(sample_aggregation_alt)

        stats = coordinator.stats()

        assert stats["total_messages_processed"] == 2
        assert stats["message_cache_size"] == 2
        assert stats["correlation_window_size"] == 2

    async def test_stats_unhealthy_instances(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test stats with unhealthy instances."""
        await coordinator.register_instance("instance:001", "localhost", 5001)
        await coordinator.register_instance("instance:002", "localhost", 5002)

        # Mark one as unhealthy
        coordinator._instances["instance:001"].is_healthy = False

        stats = coordinator.stats()

        assert stats["instance_count"] == 2
        assert stats["healthy_instances"] == 1


# ============================================================================
# TestInstanceConnection
# ============================================================================


class TestInstanceConnection:
    """Test InstanceConnection class directly."""

    async def test_instance_connection_creation(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test creating an InstanceConnection."""
        conn = InstanceConnection(
            instance_id="test:instance",
            host="localhost",
            port=5001,
            coordinator=coordinator,
        )

        assert conn.instance_id == "test:instance"
        assert conn.host == "localhost"
        assert conn.port == 5001
        assert conn.is_healthy is True
        assert conn.message_count == 0

    async def test_instance_connection_is_alive(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test is_alive check on InstanceConnection."""
        conn = InstanceConnection(
            instance_id="test:instance",
            host="localhost",
            port=5001,
            coordinator=coordinator,
        )

        # Should be alive initially (fresh heartbeat)
        assert conn.is_alive() is True

        # Set heartbeat to past
        conn.last_heartbeat = time.time() - (coordinator.instance_timeout_sec + 1)

        # Should be dead now
        assert conn.is_alive() is False

    async def test_instance_connection_close(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test closing an InstanceConnection."""
        conn = InstanceConnection(
            instance_id="test:instance",
            host="localhost",
            port=5001,
            coordinator=coordinator,
        )

        assert conn.is_healthy is True
        await conn.close()
        assert conn.is_healthy is False


# ============================================================================
# TestIntegration
# ============================================================================


class TestIntegration:
    """Integration tests combining multiple features."""

    async def test_end_to_end_dedup_and_forward(
        self, coordinator: RegionalCoordinator
    ) -> None:
        """Test complete flow: register, process, deduplicate, forward."""
        await coordinator.register_instance("instance:001", "localhost", 5001)
        await coordinator.register_instance("instance:002", "localhost", 5002)

        forwarded: list[LNSPMessage] = []

        def capture(msg: LNSPMessage) -> None:
            forwarded.append(msg)

        coordinator.subscribe(capture)

        # Create messages from both instances
        msg1 = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"value": 1},
            source="SOURCE",
            based_on=[],
            instance_id="instance:001",
        )

        msg2 = create_aggregated_state(
            aggregation_type=AggregationType.SYSTEM_STATE,
            data={"value": 2},
            source="SOURCE",
            based_on=[],
            instance_id="instance:002",
        )

        # Process messages
        await coordinator.process_aggregation(msg1)
        await coordinator.process_aggregation(msg2)
        await coordinator.process_aggregation(msg1)  # Duplicate of msg1

        # Should have 2 unique messages forwarded
        assert len(forwarded) == 2

        # Check message counts in instances
        assert coordinator._instances["instance:001"].message_count == 1
        assert coordinator._instances["instance:002"].message_count == 1

    async def test_multi_instance_with_health_monitoring(
        self, coordinator: RegionalCoordinator, sample_aggregation: LNSPMessage
    ) -> None:
        """Test coordinator with multiple instances and health monitoring."""
        # Register 3 instances
        await coordinator.register_instance("instance:001", "host1", 5001)
        await coordinator.register_instance("instance:002", "host2", 5002)
        await coordinator.register_instance("instance:003", "host3", 5003)

        # Process message from instance 001
        await coordinator.process_aggregation(sample_aggregation)

        # Simulate timeout for instance 002
        coordinator._instances["instance:002"].last_heartbeat = time.time() - (
            coordinator.instance_timeout_sec + 1
        )

        # Check health
        await coordinator.check_instance_health()

        stats = coordinator.stats()
        assert stats["instance_count"] == 3
        assert stats["healthy_instances"] == 2  # 001 and 003
