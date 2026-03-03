"""
EventForwarder: Bridge EventBus (in-memory)  SurrealDB (persistent)

PHASE 2, COMPONENT 2: Event Ingestion Pipeline

Architecture:
  EventBus (30+ CoreEvent types)
     emits events
    
  EventForwarder (this class)
     normalizes + batches
    
  SurrealDB via SecurityEventRepo
     persists to security_event table
    
  Real-Time Detection (COMPONENT 4: LIVE SELECT)
"""

from __future__ import annotations

import asyncio
import logging
import time
import uuid
from typing import Any, Optional, TYPE_CHECKING

from prometheus_client import Counter, Histogram, Gauge

if TYPE_CHECKING:
    from cynic.kernel.core.event_bus import Event, EventBus
    from cynic.kernel.core.storage.interface import StorageInterface
    from cynic.kernel.security.encryption import EncryptionService

logger = logging.getLogger("cynic.storage.event_forwarder")

# Prometheus metrics
events_forwarded_total = Counter(
    "cynic_event_forwarder_events_forwarded_total",
    "Total events forwarded to SurrealDB",
    ["event_type"],
)

batches_flushed_total = Counter(
    "cynic_event_forwarder_batches_flushed_total",
    "Total batches flushed to storage",
)

batch_flush_latency_seconds = Histogram(
    "cynic_event_forwarder_batch_flush_latency_seconds",
    "Time to flush a batch to storage",
    buckets=(0.01, 0.05, 0.1, 0.5, 1.0, 5.0),
)

queue_size_gauge = Gauge(
    "cynic_event_forwarder_queue_size",
    "Current size of event queue",
)

# Sensitive fields that should be encrypted
SENSITIVE_FIELDS = {
    "treasury_address",
    "community_token",
    "proposal_value",
    "voter_id",
    "api_key",
    "secret",
}


class EventQueue:
    """Accumulate events and flush when batch is full or timeout occurs."""

    def __init__(
        self,
        batch_size: int = 100,
        flush_interval_sec: float = 5.0,
    ):
        self.batch_size = batch_size
        self.flush_interval = flush_interval_sec
        self.events: list[dict[str, Any]] = []
        self._last_flush = time.time()
        self._lock = asyncio.Lock()

    async def add(self, event: dict[str, Any]) -> None:
        """Add event to batch."""
        async with self._lock:
            self.events.append(event)
            queue_size_gauge.set(len(self.events))

    async def should_flush(self) -> bool:
        """Check if batch should be flushed."""
        async with self._lock:
            # Flush if batch is full
            if len(self.events) >= self.batch_size:
                return True
            # Flush if timeout elapsed
            if time.time() - self._last_flush > self.flush_interval:
                return True
            return False

    async def get_and_clear(self) -> list[dict[str, Any]]:
        """Get all events and clear the queue."""
        async with self._lock:
            events = self.events.copy()
            self.events.clear()
            self._last_flush = time.time()
            queue_size_gauge.set(0)
            return events

    async def size(self) -> int:
        """Get current queue size."""
        async with self._lock:
            return len(self.events)


class EventForwarder:
    """
    Subscribe to EventBus and forward events to persistent storage.

    Features:
    - Normalizes 30+ CoreEvent types to dict
    - Batches events for efficient persistence
    - Handles backpressure (pause/resume/drop)
    - Encrypts sensitive fields
    - Metrics & monitoring
    - Graceful shutdown
    """

    def __init__(
        self,
        bus: EventBus,
        storage: StorageInterface,
        encryption_service: Optional[EncryptionService] = None,
        batch_size: int = 100,
        flush_interval_sec: float = 5.0,
    ):
        self.bus = bus
        self.storage = storage
        self.encryption = encryption_service
        self.queue = EventQueue(batch_size, flush_interval_sec)
        self.total_forwarded = 0
        self.total_batches_flushed = 0
        self._running = False
        self._pause_event = asyncio.Event()
        self._pause_event.set()  # Start unpaused

    async def start(self) -> None:
        """Subscribe to EventBus and start background flush task."""
        logger.info("EventForwarder starting")
        self._running = True

        # Subscribe to all event types
        self.bus.on("*", self.on_event)

        # Start background flush task
        asyncio.create_task(self._flush_loop())
        logger.info("EventForwarder started - subscribed to EventBus")

    async def stop(self) -> None:
        """Stop EventForwarder and flush remaining events."""
        logger.info("EventForwarder stopping")
        self._running = False

        # Flush remaining events
        await self._flush()

        # Unsubscribe from EventBus
        self.bus.off("*", self.on_event)
        logger.info("EventForwarder stopped")

    async def on_event(self, event: Event) -> None:
        """Handle event from EventBus (non-blocking)."""
        if not self._running:
            return

        # Wait if paused (backpressure)
        await self._pause_event.wait()

        try:
            # Normalize event
            normalized = self._normalize(event)

            # Encrypt sensitive fields
            if self.encryption:
                normalized = await self._encrypt_sensitive_fields(normalized)

            # Add to queue
            await self.queue.add(normalized)

            # Metrics
            events_forwarded_total.labels(event_type=event.type).inc()
            self.total_forwarded += 1

        except Exception as exc:
            logger.error(f"Failed to forward event {event.type}: {exc}", exc_info=True)

    def _normalize(self, event: Event) -> dict[str, Any]:
        """Normalize Event to dict for SurrealDB."""
        payload = event.dict_payload if hasattr(event, "dict_payload") else {}

        return {
            "id": str(uuid.uuid4()),
            "type": event.type,
            "timestamp": event.timestamp,
            "event_id": event.event_id,
            "instance_id": getattr(event, "instance_id", "unknown"),
            "source": getattr(event, "source", "unknown"),
            "payload": payload,
            "version": "1.0",
        }

    async def _encrypt_sensitive_fields(self, normalized: dict[str, Any]) -> dict[str, Any]:
        """Encrypt sensitive fields in payload."""
        if not self.encryption or "payload" not in normalized:
            return normalized

        payload = normalized["payload"]
        encrypted_payload = {}

        for key, value in payload.items():
            if key in SENSITIVE_FIELDS and value is not None:
                # Encrypt and store with _encrypted suffix
                encrypted_value = await self.encryption.encrypt_string(
                    str(value),
                    key_id=f"event-{key}",
                )
                encrypted_payload[f"{key}_encrypted"] = encrypted_value
            else:
                encrypted_payload[key] = value

        normalized["payload"] = encrypted_payload
        return normalized

    async def _flush_loop(self) -> None:
        """Background task: periodically flush batches to storage."""
        while self._running:
            try:
                # Check if we should flush
                if await self.queue.should_flush():
                    await self._flush()
                else:
                    await asyncio.sleep(0.1)
            except Exception as exc:
                logger.error(f"Error in flush loop: {exc}", exc_info=True)

    async def _flush(self) -> None:
        """Flush queue to storage."""
        # Get events to flush
        events = await self.queue.get_and_clear()

        if not events:
            return

        # Check backpressure
        queue_size = await self.queue.size()
        if queue_size > self.queue.batch_size * 0.95:
            # Queue at 95% capacity - apply backpressure
            await self._apply_backpressure()

        # Persist to storage
        t_start = time.perf_counter()
        try:
            for event in events:
                await self.storage.security_events.save_event(event)

            latency = time.perf_counter() - t_start
            batch_flush_latency_seconds.observe(latency)
            batches_flushed_total.inc()
            self.total_batches_flushed += 1

            logger.debug(
                f"Flushed {len(events)} events to storage in {latency:.3f}s"
            )
        except Exception as exc:
            logger.error(f"Failed to flush batch to storage: {exc}", exc_info=True)
            # Requeue events on failure? For now, log and continue
            # Could implement retry logic here

    async def _apply_backpressure(self) -> None:
        """Apply backpressure: pause EventBus if queue > 95%."""
        queue_size = await self.queue.size()
        if queue_size > self.queue.batch_size * 0.95:
            logger.warning(
                f"Backpressure: queue at {queue_size}/{self.queue.batch_size * 1.0:.0f}, pausing EventBus"
            )
            self._pause_event.clear()

            # Wait until queue drains to < 50%
            while await self.queue.size() > self.queue.batch_size * 0.5:
                await asyncio.sleep(0.1)

            logger.info("Queue drained, resuming EventBus")
            self._pause_event.set()

    async def get_stats(self) -> dict[str, Any]:
        """Get EventForwarder statistics."""
        return {
            "total_forwarded": self.total_forwarded,
            "total_batches_flushed": self.total_batches_flushed,
            "queue_size": await self.queue.size(),
            "queue_max": self.queue.batch_size,
            "is_paused": not self._pause_event.is_set(),
        }
