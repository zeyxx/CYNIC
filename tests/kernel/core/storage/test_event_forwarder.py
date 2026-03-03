"""Tests for EventForwarder (PHASE 2, COMPONENT 2)."""

import pytest
import time
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.kernel.core.event_bus import Event, EventBus, CoreEvent
from cynic.kernel.core.storage.event_forwarder import EventForwarder, EventQueue


@pytest.fixture
def event_bus():
    """Create a test EventBus."""
    return EventBus("test-bus", "test-instance")


@pytest.fixture
def mock_storage():
    """Create a mock StorageInterface."""
    storage = AsyncMock()
    storage.security_events = AsyncMock()
    storage.security_events.save_event = AsyncMock(return_value="evt-123")
    return storage


@pytest.fixture
def mock_encryption():
    """Create a mock EncryptionService."""
    encryption = AsyncMock()
    encryption.encrypt_string = AsyncMock(return_value="encrypted_value_hex")
    return encryption


@pytest.fixture
def event_forwarder(event_bus, mock_storage, mock_encryption):
    """Create EventForwarder with mocks."""
    return EventForwarder(
        bus=event_bus,
        storage=mock_storage,
        encryption_service=mock_encryption,
        batch_size=10,
        flush_interval_sec=1.0,
    )


class TestEventQueue:
    """Test EventQueue batching logic."""

    @pytest.mark.asyncio
    async def test_queue_add_event(self):
        """Test adding event to queue."""
        queue = EventQueue(batch_size=5)

        event = {"type": "test", "data": "value"}
        await queue.add(event)

        size = await queue.size()
        assert size == 1

    @pytest.mark.asyncio
    async def test_queue_batch_flush_when_full(self):
        """Test queue flushes when batch is full."""
        queue = EventQueue(batch_size=3)

        assert not await queue.should_flush()

        await queue.add({"id": "1"})
        assert not await queue.should_flush()

        await queue.add({"id": "2"})
        assert not await queue.should_flush()

        await queue.add({"id": "3"})
        assert await queue.should_flush()

    @pytest.mark.asyncio
    async def test_queue_flush_timeout(self):
        """Test queue flushes on timeout."""
        queue = EventQueue(batch_size=100, flush_interval_sec=0.1)

        await queue.add({"id": "1"})
        assert not await queue.should_flush()

        # Wait for timeout
        await asyncio.sleep(0.15)
        assert await queue.should_flush()

    @pytest.mark.asyncio
    async def test_queue_get_and_clear(self):
        """Test getting events and clearing queue."""
        queue = EventQueue()

        await queue.add({"id": "1"})
        await queue.add({"id": "2"})

        events = await queue.get_and_clear()

        assert len(events) == 2
        assert events[0]["id"] == "1"

        size = await queue.size()
        assert size == 0


class TestEventForwarderNormalization:
    """Test event normalization."""

    @pytest.mark.asyncio
    async def test_normalize_event(self, event_forwarder):
        """Test event normalization."""
        event = Event.typed(
            CoreEvent.JUDGMENT_CREATED,
            payload={"q_score": 75.5, "verdict": "WAG"},
            source="test",
        )

        normalized = event_forwarder._normalize(event)

        assert normalized["type"] == CoreEvent.JUDGMENT_CREATED.value
        assert "timestamp" in normalized
        assert "event_id" in normalized
        assert "instance_id" in normalized
        assert normalized["source"] == "test"
        assert normalized["payload"]["q_score"] == 75.5

    @pytest.mark.asyncio
    async def test_normalize_multiple_event_types(self, event_forwarder):
        """Test normalization of different event types."""
        event_types = [
            CoreEvent.JUDGMENT_CREATED,
            CoreEvent.CONSENSUS_REACHED,
            CoreEvent.LEARNING_EVENT,
            CoreEvent.ANOMALY_DETECTED,
        ]

        for event_type in event_types:
            event = Event.typed(event_type)
            normalized = event_forwarder._normalize(event)

            assert normalized["type"] == event_type.value
            assert normalized["version"] == "1.0"


class TestEventForwarderEncryption:
    """Test encryption of sensitive fields."""

    @pytest.mark.asyncio
    async def test_encrypt_treasury_address(self, event_forwarder, mock_encryption):
        """Test encryption of treasury_address."""
        normalized = {
            "type": "governance_vote",
            "payload": {
                "treasury_address": "0xdeadbeef123456",
                "vote": "yes",
            },
        }

        result = await event_forwarder._encrypt_sensitive_fields(normalized)

        assert "treasury_address_encrypted" in result["payload"]
        assert "treasury_address" not in result["payload"]
        assert result["payload"]["vote"] == "yes"
        mock_encryption.encrypt_string.assert_called()

    @pytest.mark.asyncio
    async def test_encrypt_proposal_value(self, event_forwarder, mock_encryption):
        """Test encryption of proposal_value."""
        normalized = {
            "type": "proposal_created",
            "payload": {
                "proposal_value": 1000000,
                "title": "Budget Allocation",
            },
        }

        result = await event_forwarder._encrypt_sensitive_fields(normalized)

        assert "proposal_value_encrypted" in result["payload"]
        assert "proposal_value" not in result["payload"]
        assert result["payload"]["title"] == "Budget Allocation"

    @pytest.mark.asyncio
    async def test_no_encryption_without_service(self, event_bus, mock_storage):
        """Test that encryption is skipped if service is None."""
        forwarder = EventForwarder(
            bus=event_bus,
            storage=mock_storage,
            encryption_service=None,
        )

        normalized = {
            "type": "test",
            "payload": {"treasury_address": "0xdeadbeef"},
        }

        result = await forwarder._encrypt_sensitive_fields(normalized)

        assert result == normalized


class TestEventForwarderEventHandling:
    """Test event handling and forwarding."""

    @pytest.mark.asyncio
    async def test_on_event_forwards_to_queue(self, event_forwarder):
        """Test on_event adds normalized event to queue."""
        event_forwarder._running = True
        event_forwarder._pause_event.set()

        event = Event.typed(CoreEvent.JUDGMENT_CREATED)

        await event_forwarder.on_event(event)

        queue_size = await event_forwarder.queue.size()
        assert queue_size == 1

    @pytest.mark.asyncio
    async def test_on_event_respects_paused_state(self, event_forwarder):
        """Test on_event waits if paused."""
        event_forwarder._running = True
        event_forwarder._pause_event.clear()  # Pause

        event = Event.typed(CoreEvent.JUDGMENT_CREATED)

        # Start on_event in background (will wait)
        task = asyncio.create_task(event_forwarder.on_event(event))

        # Give it time to wait
        await asyncio.sleep(0.05)

        # Queue should still be empty (waiting)
        assert await event_forwarder.queue.size() == 0

        # Unpause
        event_forwarder._pause_event.set()

        # Wait for task to complete
        await asyncio.wait_for(task, timeout=1.0)

        # Now event should be in queue
        assert await event_forwarder.queue.size() == 1

    @pytest.mark.asyncio
    async def test_on_event_ignores_when_not_running(self, event_forwarder):
        """Test on_event does nothing when not running."""
        event_forwarder._running = False

        event = Event.typed(CoreEvent.JUDGMENT_CREATED)
        await event_forwarder.on_event(event)

        assert await event_forwarder.queue.size() == 0


class TestEventForwarderFlushing:
    """Test batch flushing to storage."""

    @pytest.mark.asyncio
    async def test_flush_saves_events(self, event_forwarder, mock_storage):
        """Test _flush saves events to storage."""
        await event_forwarder.queue.add({"id": "1", "type": "test"})
        await event_forwarder.queue.add({"id": "2", "type": "test"})

        await event_forwarder._flush()

        assert mock_storage.security_events.save_event.call_count == 2

    @pytest.mark.asyncio
    async def test_flush_clears_queue(self, event_forwarder):
        """Test _flush clears the queue."""
        await event_forwarder.queue.add({"id": "1"})

        size_before = await event_forwarder.queue.size()
        assert size_before == 1

        await event_forwarder._flush()

        size_after = await event_forwarder.queue.size()
        assert size_after == 0

    @pytest.mark.asyncio
    async def test_flush_on_demand(self, event_forwarder, mock_storage):
        """Test flush works without waiting for batch size or timeout."""
        event = Event.typed(CoreEvent.JUDGMENT_CREATED)
        event_forwarder._running = True
        event_forwarder._pause_event.set()

        await event_forwarder.on_event(event)

        # Flush immediately
        await event_forwarder._flush()

        # Storage should have been called
        mock_storage.security_events.save_event.assert_called_once()


class TestEventForwarderBackpressure:
    """Test backpressure handling."""

    @pytest.mark.asyncio
    async def test_apply_backpressure_when_queue_full(self, event_forwarder):
        """Test backpressure is applied when queue reaches 95%."""
        # Fill queue to 95%
        for i in range(10):  # batch_size = 10
            await event_forwarder.queue.add({"id": f"{i}"})

        # Start backpressure task
        task = asyncio.create_task(event_forwarder._apply_backpressure())

        # Give it time to detect and pause
        await asyncio.sleep(0.05)

        # Should be paused
        assert not event_forwarder._pause_event.is_set()

        # Clean up
        task.cancel()
        try:
            await task
        except asyncio.CancelledError:
            pass


class TestEventForwarderIntegration:
    """Integration tests for full workflow."""

    @pytest.mark.asyncio
    async def test_start_and_stop(self, event_forwarder, event_bus):
        """Test starting and stopping EventForwarder."""
        assert not event_forwarder._running

        await event_forwarder.start()
        assert event_forwarder._running

        await event_forwarder.stop()
        assert not event_forwarder._running

    @pytest.mark.asyncio
    async def test_event_flow_through_forwarder(
        self, event_forwarder, event_bus, mock_storage
    ):
        """Test complete event flow: emit → forward → store."""
        event_forwarder._running = True
        event_forwarder._pause_event.set()

        # Create and emit event
        event = Event.typed(
            CoreEvent.JUDGMENT_CREATED,
            payload={"q_score": 75.5, "verdict": "WAG"},
        )
        event_bus.on(CoreEvent.JUDGMENT_CREATED.value, event_forwarder.on_event)

        # Emit event
        await event_bus.emit(event)

        # Give handler time to process
        await asyncio.sleep(0.1)

        # Event should be in queue
        queue_size = await event_forwarder.queue.size()
        assert queue_size == 1

        # Flush
        await event_forwarder._flush()

        # Storage should have been called
        mock_storage.security_events.save_event.assert_called()

    @pytest.mark.asyncio
    async def test_graceful_shutdown_flushes_events(
        self, event_forwarder, mock_storage
    ):
        """Test graceful shutdown flushes remaining events."""
        event_forwarder._running = True
        event_forwarder._pause_event.set()

        # Add events to queue
        await event_forwarder.queue.add({"id": "1"})
        await event_forwarder.queue.add({"id": "2"})

        # Stop (should flush)
        await event_forwarder.stop()

        # Storage should have been called with the events
        assert mock_storage.security_events.save_event.call_count == 2


class TestEventForwarderMetrics:
    """Test metrics and statistics."""

    @pytest.mark.asyncio
    async def test_get_stats(self, event_forwarder):
        """Test get_stats returns correct metrics."""
        event_forwarder.total_forwarded = 100
        event_forwarder.total_batches_flushed = 10

        stats = await event_forwarder.get_stats()

        assert stats["total_forwarded"] == 100
        assert stats["total_batches_flushed"] == 10
        assert "queue_size" in stats
        assert "queue_max" in stats
        assert "is_paused" in stats

    @pytest.mark.asyncio
    async def test_metrics_on_event_forward(self, event_forwarder):
        """Test metrics are updated on event forward."""
        event_forwarder._running = True
        event_forwarder._pause_event.set()

        event = Event.typed(CoreEvent.JUDGMENT_CREATED)

        assert event_forwarder.total_forwarded == 0

        await event_forwarder.on_event(event)

        assert event_forwarder.total_forwarded == 1


class TestEventForwarderEdgeCases:
    """Test edge cases and error handling."""

    @pytest.mark.asyncio
    async def test_on_event_handles_exception(self, event_forwarder):
        """Test on_event handles exceptions gracefully."""
        event_forwarder._running = True
        event_forwarder._pause_event.set()

        # Mock queue.add to raise exception
        event_forwarder.queue.add = AsyncMock(
            side_effect=Exception("Queue error")
        )

        event = Event.typed(CoreEvent.JUDGMENT_CREATED)

        # Should not raise
        await event_forwarder.on_event(event)

        # total_forwarded should not be incremented
        assert event_forwarder.total_forwarded == 0

    @pytest.mark.asyncio
    async def test_multiple_concurrent_events(self, event_forwarder):
        """Test handling multiple concurrent events."""
        event_forwarder._running = True
        event_forwarder._pause_event.set()

        events = [Event.typed(CoreEvent.JUDGMENT_CREATED) for _ in range(5)]

        # Forward all events concurrently
        await asyncio.gather(*[event_forwarder.on_event(e) for e in events])

        queue_size = await event_forwarder.queue.size()
        assert queue_size == 5

    @pytest.mark.asyncio
    async def test_normalize_event_with_none_payload(self, event_forwarder):
        """Test normalization with None payload."""
        event = Event.typed(CoreEvent.JUDGMENT_CREATED, payload=None)

        normalized = event_forwarder._normalize(event)

        assert normalized["payload"] == {}

    @pytest.mark.asyncio
    async def test_encrypt_none_sensitive_value(self, event_forwarder, mock_encryption):
        """Test encryption skips None values."""
        normalized = {
            "type": "test",
            "payload": {
                "treasury_address": None,
                "data": "value",
            },
        }

        result = await event_forwarder._encrypt_sensitive_fields(normalized)

        # None value should not be encrypted
        assert "treasury_address_encrypted" not in result["payload"]
        mock_encryption.encrypt_string.assert_not_called()


# Import asyncio at module level for tests
import asyncio
