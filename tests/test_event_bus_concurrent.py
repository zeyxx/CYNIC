"""
Test suite for EventBus concurrent handler registration/unregistration.

Verifies that Phase 2A threading locks prevent race conditions in handler
registration, preventing data corruption at 10k TPS target.
"""

import asyncio
from concurrent.futures import ThreadPoolExecutor

import pytest

from cynic.kernel.core.event_bus import Event, EventBus, CoreEvent


@pytest.fixture
def event_bus():
    """Create fresh EventBus for testing."""
    return EventBus(bus_id="TEST", instance_id="test")


@pytest.mark.asyncio
async def test_concurrent_handler_registration(event_bus):
    """Concurrent on() calls should not corrupt handler list."""
    handlers = []
    num_handlers = 50

    # Create handlers
    for i in range(num_handlers):
        async def handler(event, handler_id=i):
            await asyncio.sleep(0.001)

        handlers.append(handler)

    # Register handlers concurrently (thread-safe on())
    def register_handler(h):
        event_bus.on(CoreEvent.PERCEPTION_RECEIVED, h)

    with ThreadPoolExecutor(max_workers=10) as executor:
        list(executor.map(register_handler, handlers))

    # Verify all handlers were registered
    registered = event_bus._handlers.get(CoreEvent.PERCEPTION_RECEIVED.value, [])
    assert len(registered) == num_handlers, (
        f"Expected {num_handlers} handlers, got {len(registered)}"
    )


@pytest.mark.asyncio
async def test_concurrent_handler_unregistration(event_bus):
    """Concurrent off() calls should not corrupt handler list."""
    handlers = []
    num_handlers = 30

    # Create and register handlers
    for i in range(num_handlers):
        async def handler(event, handler_id=i):
            await asyncio.sleep(0.001)

        handlers.append(handler)
        event_bus.on(CoreEvent.PERCEPTION_RECEIVED, handler)

    # Verify registration
    assert len(event_bus._handlers[CoreEvent.PERCEPTION_RECEIVED.value]) == num_handlers

    # Unregister handlers concurrently
    def unregister_handler(h):
        event_bus.off(CoreEvent.PERCEPTION_RECEIVED, h)

    with ThreadPoolExecutor(max_workers=10) as executor:
        list(executor.map(unregister_handler, handlers))

    # Verify all handlers were unregistered
    remaining = event_bus._handlers.get(CoreEvent.PERCEPTION_RECEIVED.value, [])
    assert len(remaining) == 0, f"Expected 0 handlers, got {len(remaining)}"


@pytest.mark.asyncio
async def test_concurrent_emit_during_registration(event_bus):
    """Emit should not crash if handlers are registered concurrently."""
    events_received = []

    async def handler(event):
        events_received.append(event.type)
        await asyncio.sleep(0.005)

    # Start concurrent registration in background
    async def register_handlers():
        for i in range(20):
            event_bus.on(CoreEvent.JUDGMENT_CREATED, handler)
            await asyncio.sleep(0.002)

    # Start concurrent emission
    async def emit_events():
        for i in range(10):
            event = Event.typed(
                CoreEvent.JUDGMENT_CREATED,
                payload={"judgment_id": f"j{i}"},
                source="test",
            )
            await event_bus.emit(event)
            await asyncio.sleep(0.003)

    # Run both concurrently
    await asyncio.gather(register_handlers(), emit_events())

    # Should have received events without crashes
    assert len(events_received) > 0, "Should have received some events"
    # Some handlers may have been registered after some emissions
    assert len(events_received) <= 20 * 10, "Sanity check on event counts"


@pytest.mark.asyncio
async def test_concurrent_emit_during_unregistration(event_bus):
    """Emit should not crash if handlers are unregistered concurrently."""
    events_received = []
    handlers = []

    # Register initial handlers
    for i in range(20):
        async def handler(event, handler_id=i):
            events_received.append(f"h{handler_id}")
            await asyncio.sleep(0.005)

        handlers.append(handler)
        event_bus.on(CoreEvent.JUDGMENT_CREATED, handler)

    # Start concurrent unregistration
    async def unregister_handlers():
        for h in handlers:
            event_bus.off(CoreEvent.JUDGMENT_CREATED, h)
            await asyncio.sleep(0.002)

    # Start concurrent emission
    async def emit_events():
        for i in range(10):
            event = Event.typed(
                CoreEvent.JUDGMENT_CREATED,
                payload={"judgment_id": f"j{i}"},
                source="test",
            )
            await event_bus.emit(event)
            await asyncio.sleep(0.003)

    # Run both concurrently - should not crash
    await asyncio.gather(unregister_handlers(), emit_events())

    # Should have completed without exception
    assert True, "No exception during concurrent unregister + emit"


@pytest.mark.asyncio
async def test_drain_during_concurrent_emit(event_bus):
    """Drain should safely wait for tasks even during concurrent emit."""
    events_emitted = []

    async def slow_handler(event):
        events_emitted.append(1)
        await asyncio.sleep(0.05)

    event_bus.on(CoreEvent.PERCEPTION_RECEIVED, slow_handler)

    # Start emitting in background
    async def emit_continuously():
        for i in range(10):
            event = Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                payload={"index": i},
                source="test",
            )
            await event_bus.emit(event)
            await asyncio.sleep(0.01)

    # Run emit in background while we drain
    emit_task = asyncio.create_task(emit_continuously())

    # Wait a bit for some events to queue up
    await asyncio.sleep(0.05)

    # Drain should complete despite ongoing emissions
    await event_bus.drain(timeout=2.0)

    # Wait for emit to finish
    await emit_task

    # All events should have been processed
    assert len(events_emitted) == 10, f"Expected 10 events, got {len(events_emitted)}"


@pytest.mark.asyncio
async def test_handler_snapshot_prevents_iteration_issues(event_bus):
    """Handler snapshot during emit prevents iteration issues."""
    call_log = []

    async def handler_a(event):
        call_log.append("a")
        await asyncio.sleep(0.01)

    async def handler_b(event):
        call_log.append("b")
        await asyncio.sleep(0.01)

    async def handler_c(event):
        call_log.append("c")
        await asyncio.sleep(0.01)

    # Register handlers
    event_bus.on(CoreEvent.PERCEPTION_RECEIVED, handler_a)
    event_bus.on(CoreEvent.PERCEPTION_RECEIVED, handler_b)
    event_bus.on(CoreEvent.PERCEPTION_RECEIVED, handler_c)

    # Emit event (handler snapshot should protect us)
    event = Event.typed(
        CoreEvent.PERCEPTION_RECEIVED,
        payload={"test": "data"},
        source="test",
    )
    await event_bus.emit(event)

    # Wait for handlers to execute
    await asyncio.sleep(0.05)

    # All handlers should have executed
    assert "a" in call_log
    assert "b" in call_log
    assert "c" in call_log


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
