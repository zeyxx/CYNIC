"""
CYNIC WorldModelUpdater Tests

6 tests. No LLM, no DB.
Uses real event bus (isolated per test).
asyncio_mode = "auto" — all async tests run automatically.
"""
from __future__ import annotations

import asyncio
import pytest

from cynic.core.event_bus import CoreEvent, Event, EventBus
from cynic.core.world_model import WorldModelUpdater


# ── helpers ──────────────────────────────────────────────────────────────────

def _make_judgment_event(
    reality: str = "CODE",
    verdict: str = "WAG",
    q_score: float = 50.0,
    judgment_id: str = "",
) -> Event:
    return Event(
        type=CoreEvent.JUDGMENT_CREATED,
        payload={
            "reality": reality,
            "verdict": verdict,
            "q_score": q_score,
            "judgment_id": judgment_id,
        },
    )


def _fresh_updater() -> tuple:
    """Return an updater wired to a fresh isolated bus."""
    bus = EventBus(bus_id="TEST_WORLD_MODEL")
    updater = WorldModelUpdater()
    updater.start(bus=bus)
    return updater, bus


# ── Tests ─────────────────────────────────────────────────────────────────────

class TestWorldModelUpdater:

    def test_initial_snapshot_empty(self):
        """Fresh updater has no realities tracked."""
        updater = WorldModelUpdater()
        snap = updater.snapshot()
        assert snap["realities"] == {}
        assert snap["judgment_count"] == 0

    async def test_judgment_updates_snapshot(self):
        """Emitting JUDGMENT_CREATED updates the reality snapshot correctly."""
        updater, bus = _fresh_updater()
        event = _make_judgment_event(reality="CODE", verdict="WAG", q_score=72.0)
        await bus.emit(event)
        await asyncio.sleep(0)  # let Tasks run

        snap = updater.snapshot()
        assert "CODE" in snap["realities"]
        assert snap["realities"]["CODE"]["verdict"] == "WAG"
        assert abs(snap["realities"]["CODE"]["q_score"] - 72.0) < 1e-6

    async def test_dominant_reality_is_lowest_q(self):
        """Dominant reality is the one with the lowest q_score (highest risk)."""
        updater, bus = _fresh_updater()
        await bus.emit(_make_judgment_event(reality="CODE", verdict="HOWL", q_score=80.0))
        await bus.emit(_make_judgment_event(reality="CYNIC", verdict="BARK", q_score=30.0))
        await asyncio.sleep(0)

        snap = updater.snapshot()
        assert snap["dominant_reality"] == "CYNIC"

    async def test_conflict_detected(self):
        """HOWL in one reality vs BARK in another triggers a conflict entry."""
        updater, bus = _fresh_updater()
        await bus.emit(_make_judgment_event(reality="CODE", verdict="HOWL", q_score=90.0))
        await bus.emit(_make_judgment_event(reality="CYNIC", verdict="BARK", q_score=15.0))
        await asyncio.sleep(0)

        snap = updater.snapshot()
        assert len(snap["conflicts"]) > 0
        conflict_str = snap["conflicts"][0]
        assert "CODE" in conflict_str
        assert "CYNIC" in conflict_str

    async def test_no_conflict_when_all_same(self):
        """No conflict when all realities share the same verdict."""
        updater, bus = _fresh_updater()
        await bus.emit(_make_judgment_event(reality="CODE", verdict="WAG", q_score=65.0))
        await bus.emit(_make_judgment_event(reality="CYNIC", verdict="WAG", q_score=70.0))
        await asyncio.sleep(0)

        snap = updater.snapshot()
        assert snap["conflicts"] == []

    async def test_composite_risk_increases_with_bark(self):
        """Emitting a BARK (very low q_score) should drive composite_risk above 50."""
        updater, bus = _fresh_updater()
        await bus.emit(_make_judgment_event(reality="CODE", verdict="BARK", q_score=10.0))
        await asyncio.sleep(0)

        snap = updater.snapshot()
        # risk for q=10 is 90; composite_risk should be well above 50
        assert snap["composite_risk"] > 50.0
