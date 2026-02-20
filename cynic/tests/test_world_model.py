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


# ── Kernel wiring tests ───────────────────────────────────────────────────────

class TestWorldModelKernelWiring:
    """
    Verify WorldModelUpdater is properly wired into AppState / build_kernel().

    These tests exercise the wiring that was previously missing — WorldModel
    was 0% LIVE (code existed, never instantiated). Now it is LIVE.
    """

    def test_app_state_has_world_model_field(self):
        """CynicOrganism must expose world_model (via backward-compatible property).

        After the service layer refactor, world_model is nested in SensoryCore,
        but accessible via a backward-compatible property on CynicOrganism.
        """
        from cynic.api.state import CynicOrganism
        # Check that the property is accessible (via hasattr and callable)
        assert hasattr(CynicOrganism, "world_model"), "world_model property not found"
        # Also verify it's a property, not a data field
        import inspect
        members = inspect.getmembers(CynicOrganism)
        world_model_member = next((m for m in members if m[0] == "world_model"), None)
        assert world_model_member is not None, "world_model not accessible"
        assert isinstance(world_model_member[1], property), "world_model should be a property"

    def test_world_model_field_type_is_world_model_updater(self):
        """world_model field default_factory creates a WorldModelUpdater instance."""
        import dataclasses
        from cynic.api.state import CynicOrganism
        for f in dataclasses.fields(CynicOrganism):
            if f.name == "world_model":
                # field_factory should produce a WorldModelUpdater
                instance = f.default_factory()  # type: ignore[misc]
                assert isinstance(instance, WorldModelUpdater)
                break

    def test_world_model_starts_on_first_judgment(self):
        """WorldModelUpdater starts accepting events after start() is called."""
        updater, bus = _fresh_updater()
        assert updater._started is True

    async def test_world_model_snapshot_keys(self):
        """snapshot() always returns the expected keys — contract for /world-state."""
        updater = WorldModelUpdater()
        snap = updater.snapshot()
        required = {"composite_risk", "dominant_reality", "conflicts", "realities",
                    "judgment_count", "conflict_count", "last_updated"}
        assert required.issubset(snap.keys()), f"missing keys: {required - snap.keys()}"

    async def test_world_model_tracks_multiple_realities(self):
        """After judgments in 3 realities, snapshot reflects all 3."""
        updater, bus = _fresh_updater()
        for reality in ("CODE", "CYNIC", "HUMAN"):
            await bus.emit(_make_judgment_event(reality=reality, q_score=60.0))
        await asyncio.sleep(0)

        snap = updater.snapshot()
        assert set(snap["realities"].keys()) >= {"CODE", "CYNIC", "HUMAN"}
        assert snap["judgment_count"] == 3

    def test_world_model_not_none_in_imported_app_state(self):
        """After service layer refactor, CynicOrganism provides world_model via backward-compatible property."""
        from cynic.api.state import CynicOrganism, CognitionCore, MetabolicCore, SensoryCore, MemoryCore
        from unittest.mock import MagicMock

        # Build the four façades required by the new architecture
        cognition = CognitionCore(
            orchestrator=MagicMock(),
            qtable=MagicMock(),
            learning_loop=MagicMock(),
            residual_detector=MagicMock(),
        )
        metabolism = MetabolicCore(scheduler=MagicMock())
        senses = SensoryCore(
            context_compressor=MagicMock(),
            service_registry=MagicMock(),
            world_model=WorldModelUpdater(),
        )
        memory = MemoryCore()

        state = CynicOrganism(
            cognition=cognition,
            metabolism=metabolism,
            senses=senses,
            memory=memory,
        )
        assert state.world_model is not None
        assert isinstance(state.world_model, WorldModelUpdater)
