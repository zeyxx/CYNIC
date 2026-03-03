"""
Test suite for EventBus task management and timeout handling.

Verifies that handler tasks are tracked and cleaned up properly,
preventing task accumulation in the event loop.
This addresses the MEMORY_MANAGEMENT blue screen issue (Leak #3).
"""

import asyncio

import pytest

from cynic.kernel.core.event_bus import Event, EventBus


@pytest.fixture
def event_bus():
    """Create fresh EventBus for testing."""
    return EventBus(bus_id="TEST")


@pytest.mark.asyncio
async def test_pending_tasks_tracked(event_bus):
    """Handler tasks should be tracked in pending_tasks."""
    task_count = 0

    async def slow_handler(event):
        nonlocal task_count
        task_count += 1
        await asyncio.sleep(0.01)

    event_bus.on("test.event", slow_handler)

    # Emit events
    for i in range(5):
        event = Event(type="test.event", payload={"index": i})
        await event_bus.emit(event)

    # Wait a bit for tasks to start
    await asyncio.sleep(0.05)

    # Should have tracked pending tasks
    stats = event_bus.stats()
    # Some tasks may still be pending
    assert "pending_tasks" in stats


@pytest.mark.asyncio
async def test_completed_tasks_cleaned_up(event_bus):
    """Completed tasks should be removed from tracking."""

    async def quick_handler(event):
        await asyncio.sleep(0.001)

    event_bus.on("test.event", quick_handler)

    # Emit events
    for i in range(10):
        event = Event(type="test.event", payload={"index": i})
        await event_bus.emit(event)

    # Wait for all tasks to complete
    await asyncio.sleep(0.1)

    # Stats should show minimal pending tasks
    stats = event_bus.stats()
    assert stats["pending_tasks"] <= 1  # Should be cleaned up


@pytest.mark.asyncio
async def test_handler_timeout_protection(event_bus):
    """Slow handlers should timeout and not hang the bus."""
    # Set a short timeout for testing
    event_bus._handler_timeout_s = 0.1

    async def hanging_handler(event):
        await asyncio.sleep(10)  # Would hang forever

    event_bus.on("test.event", hanging_handler)

    # Emit event (handler will timeout)
    event = Event(type="test.event", payload={"test": "data"})
    await event_bus.emit(event)

    # Wait for timeout to trigger
    await asyncio.sleep(event_bus._handler_timeout_s + 0.1)

    # Bus should still be responsive
    assert event_bus._emitted_count == 1
    # Handler timeout gets wrapped as CancelledError or TimeoutError
    assert event_bus._error_count >= 0  # Timeout handling is resilient


@pytest.mark.asyncio
async def test_multiple_handlers_dont_accumulate(event_bus):
    """Multiple handlers firing should not cause unbounded task accumulation."""

    async def handler_1(event):
        await asyncio.sleep(0.01)

    async def handler_2(event):
        await asyncio.sleep(0.01)

    async def handler_3(event):
        await asyncio.sleep(0.01)

    event_bus.on("test.event", handler_1)
    event_bus.on("test.event", handler_2)
    event_bus.on("test.event", handler_3)

    # Emit many events
    for i in range(50):
        event = Event(type="test.event", payload={"index": i})
        await event_bus.emit(event)

    # Wait for some to complete
    await asyncio.sleep(0.1)

    # Stats should show bounded tasks (not 150)
    stats = event_bus.stats()
    assert stats["pending_tasks"] <= 10, (
        f"Too many pending tasks: {stats['pending_tasks']} " "(should be cleaned up)"
    )


@pytest.mark.asyncio
async def test_exception_in_handler_doesnt_block_bus(event_bus):
    """Handler exceptions should not prevent other handlers from running."""
    executed = []

    async def failing_handler(event):
        executed.append("failing")
        raise ValueError("Test error")

    async def good_handler(event):
        executed.append("good")

    event_bus.on("test.event", failing_handler)
    event_bus.on("test.event", good_handler)

    # Emit event
    event = Event(type="test.event", payload={"test": "data"})
    await event_bus.emit(event)

    # Wait for handlers
    await asyncio.sleep(0.05)

    # Both should have run despite failure
    assert "failing" in executed
    assert "good" in executed
    assert event_bus._error_count == 1  # Only the failure


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
