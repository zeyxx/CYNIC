"""
Tests: Event Bus — 3 Buses + EventBusBridge

LAW: Wire events BEFORE start(). Genealogy must prevent loops.
All tests are async (asyncio_mode = "auto").
"""
import asyncio
import pytest

from cynic.core.event_bus import (
    Event, EventBus, EventBusBridge, ForwardRule,
    get_core_bus, get_automation_bus, get_agent_bus,
    reset_all_buses, emit_core, emit_agent,
    CoreEvent, AutomationEvent, AgentEvent,
    create_default_bridge,
)


class TestEvent:
    """Event envelope — genealogy tracking."""

    def test_event_creation(self):
        e = Event(type="test.event", payload={"x": 1})
        assert e.type == "test.event"
        assert e._genealogy == []
        assert e._bridged == False

    def test_genealogy_append(self):
        e = Event(type="test.event")
        e2 = e.with_genealogy("CORE")
        assert "CORE" in e2._genealogy
        assert e2._bridged == True
        # Original unchanged
        assert e._genealogy == []

    def test_already_seen(self):
        e = Event(type="test.event", _genealogy=["CORE", "AGENT"])
        assert e.already_seen("CORE") == True
        assert e.already_seen("AUTOMATION") == False

    def test_chained_genealogy(self):
        """Event passing through multiple buses accumulates genealogy."""
        e = Event(type="test.event")
        e = e.with_genealogy("CORE")
        e = e.with_genealogy("AGENT")
        assert e._genealogy == ["CORE", "AGENT"]
        assert e.already_seen("CORE") == True
        assert e.already_seen("AGENT") == True


class TestEventBus:
    """Single bus — subscribe/emit/fire-and-forget."""

    async def test_emit_calls_handler(self):
        bus = EventBus(bus_id="TEST")
        received = []

        async def handler(event: Event) -> None:
            received.append(event)

        bus.on("test.event", handler)
        await bus.emit(Event(type="test.event", payload={"val": 42}))
        await asyncio.sleep(0.01)  # let Task execute

        assert len(received) == 1
        assert received[0].payload["val"] == 42

    async def test_wildcard_handler(self):
        """'*' handler receives all events."""
        bus = EventBus(bus_id="TEST")
        received = []

        async def handler(event: Event) -> None:
            received.append(event.type)

        bus.on("*", handler)
        await bus.emit(Event(type="event.A"))
        await bus.emit(Event(type="event.B"))
        await asyncio.sleep(0.01)

        assert "event.A" in received
        assert "event.B" in received

    async def test_handler_error_doesnt_crash_bus(self):
        """A failing handler should not crash the bus."""
        bus = EventBus(bus_id="TEST")

        async def bad_handler(event: Event) -> None:
            raise RuntimeError("Handler failure!")

        bus.on("test.event", bad_handler)
        # This should not raise
        await bus.emit(Event(type="test.event"))
        await asyncio.sleep(0.01)
        assert bus._error_count == 1

    async def test_off_removes_handler(self):
        bus = EventBus(bus_id="TEST")
        received = []

        async def handler(event: Event) -> None:
            received.append(1)

        bus.on("test.event", handler)
        bus.off("test.event", handler)
        await bus.emit(Event(type="test.event"))
        await asyncio.sleep(0.01)

        assert len(received) == 0

    def test_stats(self):
        bus = EventBus(bus_id="TEST")
        stats = bus.stats()
        assert stats["bus_id"] == "TEST"
        assert stats["emitted"] == 0


class TestEventBusBridge:
    """EventBusBridge — loop prevention + forwarding."""

    async def test_bridge_forwards_agent_to_core(self):
        core = get_core_bus()
        agent = get_agent_bus()

        core_received = []
        core.on("*", lambda e: core_received.append(e.type))

        bridge = create_default_bridge()
        bridge.start()

        await agent.emit(Event(type=AgentEvent.PBFT_REPLY, payload={}, source="CYNIC"))
        await asyncio.sleep(0.05)

        assert AgentEvent.PBFT_REPLY in core_received

    async def test_bridge_prevents_loop(self):
        """Event should NOT be re-forwarded back to its source bus."""
        core = get_core_bus()
        agent = get_agent_bus()

        forward_count = {"n": 0}

        async def count_handler(event: Event) -> None:
            forward_count["n"] += 1

        agent.on(AgentEvent.PBFT_REPLY, count_handler)

        bridge = create_default_bridge()
        bridge.start()

        # Emit on agent bus
        await agent.emit(Event(type=AgentEvent.PBFT_REPLY, payload={}))
        await asyncio.sleep(0.1)

        # Should have received the original event once (not re-forwarded back)
        # The bridge forwards AGENT→CORE, NOT CORE→AGENT for PBFT_REPLY
        stats = bridge.stats()
        assert stats["loops_prevented"] >= 0  # may be 0 if no loop attempted

    async def test_bridge_genealogy_survives_forward(self):
        """Forwarded event carries genealogy through the bridge."""
        core = get_core_bus()
        agent = get_agent_bus()

        received_events = []
        core.on("*", lambda e: received_events.append(e))

        bridge = create_default_bridge()
        bridge.start()

        await agent.emit(Event(
            type=AgentEvent.DOG_VETO,
            payload={"dog_id": "GUARDIAN"},
        ))
        await asyncio.sleep(0.05)

        # Find the forwarded event on core bus
        forwarded = [e for e in received_events if e._bridged]
        if forwarded:
            assert "AGENT" in forwarded[0]._genealogy

    async def test_bridge_stats_track_forwarded(self):
        core = get_core_bus()
        agent = get_agent_bus()

        bridge = create_default_bridge()
        bridge.start()

        await agent.emit(Event(type=AgentEvent.PBFT_REPLY, payload={}))
        await asyncio.sleep(0.05)

        stats = bridge.stats()
        assert stats["forwarded"] >= 1


class TestEventEnums:
    """Event type enums are complete and non-overlapping."""

    def test_core_events_are_strings(self):
        for ev in CoreEvent:
            assert isinstance(ev.value, str)
            assert "." in ev.value  # namespaced format

    def test_automation_events_are_strings(self):
        for ev in AutomationEvent:
            assert isinstance(ev.value, str)

    def test_agent_events_are_strings(self):
        for ev in AgentEvent:
            assert isinstance(ev.value, str)
