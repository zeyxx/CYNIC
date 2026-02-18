"""
CYNIC DecideAgent Tests — unit tests for judge/decide.py

Tests subscribe/unsubscribe, verdict filtering, confidence threshold,
and stats tracking. No DB required — QTable works fully in-memory.
"""
from __future__ import annotations

import asyncio
import pytest
import pytest_asyncio

from cynic.core.event_bus import CoreEvent, Event, EventBus, reset_all_buses
from cynic.judge.decide import DecideAgent
from cynic.learning.qlearning import QTable


# ── Fixtures ──────────────────────────────────────────────────────────────


@pytest.fixture(autouse=True)
def reset_buses():
    """Ensure a clean bus for every test."""
    reset_all_buses()
    yield
    reset_all_buses()


@pytest.fixture
def bus() -> EventBus:
    from cynic.core.event_bus import get_core_bus
    return get_core_bus()


@pytest.fixture
def qtable() -> QTable:
    return QTable()


@pytest.fixture
def agent(qtable: QTable) -> DecideAgent:
    return DecideAgent(qtable=qtable)


def _judgment_event(verdict: str, confidence: float, state_key: str = "CODE:JUDGE:PRESENT:0") -> Event:
    """Build a synthetic JUDGMENT_CREATED event."""
    return Event(
        type=CoreEvent.JUDGMENT_CREATED,
        payload={
            "verdict": verdict,
            "confidence": confidence,
            "state_key": state_key,
            "judgment_id": "test-id-001",
            "q_score": 30.0,
        },
        source="test",
    )


# ── Tests ──────────────────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_decide_agent_subscribes_on_start(bus: EventBus, agent: DecideAgent):
    """start() registers a handler on JUDGMENT_CREATED."""
    assert CoreEvent.JUDGMENT_CREATED not in bus._handlers or len(bus._handlers.get(CoreEvent.JUDGMENT_CREATED, [])) == 0
    agent.start(bus)
    assert len(bus._handlers.get(CoreEvent.JUDGMENT_CREATED, [])) == 1


@pytest.mark.asyncio
async def test_decide_skips_wag_verdict(bus: EventBus, agent: DecideAgent):
    """WAG verdict does NOT trigger a DECISION_MADE event."""
    agent.start(bus)

    decisions_received = []

    async def capture(event: Event):
        decisions_received.append(event)

    bus.on(CoreEvent.DECISION_MADE, capture)

    await bus.emit(_judgment_event("WAG", confidence=0.9))
    # Allow tasks to run
    await asyncio.sleep(0.05)

    assert len(decisions_received) == 0
    assert agent.stats()["skipped"] == 1
    assert agent.stats()["decisions_made"] == 0


@pytest.mark.asyncio
async def test_decide_triggers_on_bark_with_sufficient_confidence(bus: EventBus, agent: DecideAgent):
    """BARK + confidence >= 0.382 -> DECISION_MADE emitted."""
    agent.start(bus)

    decisions_received = []

    async def capture(event: Event):
        decisions_received.append(event)

    bus.on(CoreEvent.DECISION_MADE, capture)

    await bus.emit(_judgment_event("BARK", confidence=0.5, state_key="CODE:JUDGE:PRESENT:1"))
    await asyncio.sleep(0.1)

    assert len(decisions_received) == 1
    d = decisions_received[0]
    assert d.payload["trigger"] == "auto_decide"
    assert d.payload["judgment_id"] == "test-id-001"
    assert "recommended_action" in d.payload
    assert agent.stats()["decisions_made"] == 1


@pytest.mark.asyncio
async def test_decide_skips_bark_with_low_confidence(bus: EventBus, agent: DecideAgent):
    """BARK + confidence < 0.382 -> no DECISION_MADE."""
    agent.start(bus)

    decisions_received = []

    async def capture(event: Event):
        decisions_received.append(event)

    bus.on(CoreEvent.DECISION_MADE, capture)

    await bus.emit(_judgment_event("BARK", confidence=0.1))
    await asyncio.sleep(0.05)

    assert len(decisions_received) == 0
    assert agent.stats()["skipped"] == 1
    assert agent.stats()["decisions_made"] == 0


@pytest.mark.asyncio
async def test_decide_stats_track_counts(bus: EventBus, agent: DecideAgent):
    """stats() accurately reflects decisions_made and skipped counts."""
    agent.start(bus)

    # GROWL with sufficient confidence -> decision
    await bus.emit(_judgment_event("GROWL", confidence=0.5, state_key="k1"))
    # WAG -> skip
    await bus.emit(_judgment_event("WAG", confidence=0.9, state_key="k2"))
    # BARK with low confidence -> skip
    await bus.emit(_judgment_event("BARK", confidence=0.2, state_key="k3"))
    # BARK with sufficient confidence -> decision
    await bus.emit(_judgment_event("BARK", confidence=0.4, state_key="k4"))

    await asyncio.sleep(0.15)

    stats = agent.stats()
    assert stats["decisions_made"] == 2
    assert stats["skipped"] == 2
