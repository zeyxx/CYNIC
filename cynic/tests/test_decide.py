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
from cynic.judge.decide import DecideAgent, _build_action_prompt
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


def _judgment_event(
    verdict: str,
    confidence: float,
    state_key: str = "CODE:JUDGE:PRESENT:0",
    reality: str = "CODE",
    analysis: str = "JUDGE",
    content_preview: str = "def foo(): pass",
    context: str = "test context",
) -> Event:
    """Build a synthetic JUDGMENT_CREATED event (enriched — as orchestrator sends)."""
    return Event(
        type=CoreEvent.JUDGMENT_CREATED,
        payload={
            "verdict": verdict,
            "confidence": confidence,
            "state_key": state_key,
            "judgment_id": "test-id-001",
            "q_score": 30.0,
            "reality": reality,
            "analysis": analysis,
            "content_preview": content_preview,
            "context": context,
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


# ── Tests: enriched payload (Gap 1) ─────────────────────────────────────────


@pytest.mark.asyncio
async def test_decision_payload_includes_enriched_fields(bus: EventBus, agent: DecideAgent):
    """DECISION_MADE payload includes reality, content_preview, action_prompt."""
    agent.start(bus)

    received = []

    async def capture(event: Event):
        received.append(event)

    bus.on(CoreEvent.DECISION_MADE, capture)

    await bus.emit(_judgment_event(
        "BARK", confidence=0.5,
        state_key="CODE:JUDGE:PRESENT:0",
        reality="CODE",
        content_preview="def broken(): raise ValueError",
        context="code review",
    ))
    await asyncio.sleep(0.1)

    assert len(received) == 1
    p = received[0].payload
    assert p["reality"] == "CODE"
    assert p["content_preview"] == "def broken(): raise ValueError"
    assert "action_prompt" in p
    assert len(p["action_prompt"]) > 20  # non-trivial prompt generated


@pytest.mark.asyncio
async def test_decision_payload_cynic_reality(bus: EventBus, agent: DecideAgent):
    """CYNIC reality produces a self-assessment action prompt."""
    agent.start(bus)
    received = []

    async def capture(event: Event):
        received.append(event)

    bus.on(CoreEvent.DECISION_MADE, capture)

    await bus.emit(_judgment_event(
        "GROWL", confidence=0.5,
        state_key="CYNIC:JUDGE:PRESENT:0",
        reality="CYNIC",
        analysis="JUDGE",
        content_preview="Q-table stagnating",
    ))
    await asyncio.sleep(0.1)

    assert len(received) == 1
    prompt = received[0].payload["action_prompt"]
    assert "organism" in prompt.lower() or "self" in prompt.lower() or "cynic" in prompt.lower()


@pytest.mark.asyncio
async def test_decision_backward_compat_empty_enriched_fields(bus: EventBus, agent: DecideAgent):
    """DecideAgent still works if enriched fields are absent (backward compat)."""
    agent.start(bus)
    received = []

    async def capture(event: Event):
        received.append(event)

    bus.on(CoreEvent.DECISION_MADE, capture)

    # Old-style event without enriched fields
    old_event = Event(
        type=CoreEvent.JUDGMENT_CREATED,
        payload={
            "verdict": "BARK",
            "confidence": 0.5,
            "state_key": "CODE:JUDGE:PRESENT:0",
            "judgment_id": "old-id",
            "q_score": 25.0,
        },
        source="old_orchestrator",
    )
    await bus.emit(old_event)
    await asyncio.sleep(0.1)

    assert len(received) == 1
    p = received[0].payload
    assert "action_prompt" in p  # Still generated (from empty fields, generic fallback)
    assert "reality" in p        # reality = "" (empty string, graceful)


# ── Tests: _build_action_prompt ──────────────────────────────────────────────


class TestBuildActionPrompt:

    def test_code_bark(self):
        prompt = _build_action_prompt("CODE", "JUDGE", "BARK", "critical bug here", "")
        assert "BARK" in prompt
        assert "critical bug here" in prompt
        assert "[CYNIC AUTO-ACT]" in prompt

    def test_code_growl(self):
        prompt = _build_action_prompt("CODE", "JUDGE", "GROWL", "smell here", "")
        assert "GROWL" in prompt
        assert "smell here" in prompt

    def test_cynic_bark(self):
        prompt = _build_action_prompt("CYNIC", "JUDGE", "BARK", "stagnation", "")
        assert "BARK" in prompt
        assert "organism" in prompt.lower() or "organism" in prompt

    def test_cynic_growl(self):
        prompt = _build_action_prompt("CYNIC", "EMERGE", "GROWL", "drift detected", "ctx")
        assert "degradation" in prompt.lower() or "growl" in prompt.lower()

    def test_empty_content_uses_context(self):
        prompt = _build_action_prompt("CODE", "JUDGE", "BARK", "", "fallback context")
        assert "fallback context" in prompt

    def test_unknown_reality_generic_fallback(self):
        prompt = _build_action_prompt("MARKET", "PERCEIVE", "GROWL", "price drop", "")
        assert "MARKET" in prompt
        assert "[CYNIC AUTO-ACT]" in prompt

    def test_content_truncated_at_400_chars(self):
        long_content = "x" * 500
        prompt = _build_action_prompt("CODE", "JUDGE", "BARK", long_content, "")
        # The body inside should be at most 400 chars
        assert len(prompt) < 600  # overall prompt is reasonably bounded
