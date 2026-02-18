"""
CYNIC AccountAgent Tests — Step 6 (ACCOUNT)

Tests cost accumulation, budget enforcement, EScore-RUN updates.
No LLM, no DB — pure in-memory state machine.
"""
from __future__ import annotations

import pytest
from unittest.mock import MagicMock, AsyncMock, call

from cynic.judge.account import AccountAgent, _WARNING_RATIO, _DEFAULT_SESSION_BUDGET_USD
from cynic.core.event_bus import CoreEvent, Event, EventBus, reset_all_buses
from cynic.core.phi import PHI_INV_2


# ── helpers ────────────────────────────────────────────────────────────────

def _make_judgment_event(
    cost_usd: float = 0.0,
    q_score: float = 50.0,
    reality: str = "CODE",
    dog_votes: dict | None = None,
) -> Event:
    return Event(
        type=CoreEvent.JUDGMENT_CREATED,
        payload={
            "cost_usd": cost_usd,
            "q_score": q_score,
            "reality": reality,
            "dog_votes": dog_votes or {},
        },
    )


# ── Initial state ──────────────────────────────────────────────────────────

class TestAccountAgentInit:
    def test_starts_zero_cost(self):
        ag = AccountAgent()
        assert ag.total_cost_usd == 0.0

    def test_starts_full_budget(self):
        ag = AccountAgent(session_budget_usd=5.0)
        assert ag.budget_remaining_usd == 5.0

    def test_stats_keys(self):
        ag = AccountAgent()
        s = ag.stats()
        assert "total_cost_usd" in s
        assert "session_budget_usd" in s
        assert "budget_remaining_usd" in s
        assert "budget_ratio_remaining" in s
        assert "judgment_count" in s
        assert "cost_by_reality" in s
        assert "cost_by_dog" in s
        assert "warning_emitted" in s
        assert "exhausted_emitted" in s

    def test_default_budget_is_ten(self):
        ag = AccountAgent()
        assert ag._session_budget_usd == _DEFAULT_SESSION_BUDGET_USD


# ── Cost accumulation ──────────────────────────────────────────────────────

class TestCostAccumulation:
    @pytest.mark.asyncio
    async def test_cost_accumulates(self):
        reset_all_buses()
        ag = AccountAgent(session_budget_usd=100.0)
        await ag._on_judgment(_make_judgment_event(cost_usd=0.01))
        await ag._on_judgment(_make_judgment_event(cost_usd=0.02))
        assert abs(ag.total_cost_usd - 0.03) < 1e-9

    @pytest.mark.asyncio
    async def test_zero_cost_judgment(self):
        ag = AccountAgent()
        await ag._on_judgment(_make_judgment_event(cost_usd=0.0))
        assert ag.total_cost_usd == 0.0
        assert ag._judgment_count == 1

    @pytest.mark.asyncio
    async def test_cost_by_reality(self):
        ag = AccountAgent()
        await ag._on_judgment(_make_judgment_event(cost_usd=0.01, reality="CODE"))
        await ag._on_judgment(_make_judgment_event(cost_usd=0.02, reality="SOLANA"))
        assert abs(ag._cost_by_reality.get("CODE", 0) - 0.01) < 1e-9
        assert abs(ag._cost_by_reality.get("SOLANA", 0) - 0.02) < 1e-9

    @pytest.mark.asyncio
    async def test_cost_split_per_dog(self):
        ag = AccountAgent()
        await ag._on_judgment(_make_judgment_event(
            cost_usd=0.02,
            dog_votes={"SAGE": 50.0, "GUARDIAN": 40.0},
        ))
        assert abs(ag._cost_by_dog.get("SAGE", 0) - 0.01) < 1e-9
        assert abs(ag._cost_by_dog.get("GUARDIAN", 0) - 0.01) < 1e-9

    @pytest.mark.asyncio
    async def test_judgment_count_increments(self):
        ag = AccountAgent()
        for _ in range(5):
            await ag._on_judgment(_make_judgment_event())
        assert ag._judgment_count == 5

    @pytest.mark.asyncio
    async def test_budget_decreases(self):
        ag = AccountAgent(session_budget_usd=1.0)
        await ag._on_judgment(_make_judgment_event(cost_usd=0.3))
        assert abs(ag.budget_remaining_usd - 0.7) < 1e-9


# ── Budget enforcement ─────────────────────────────────────────────────────

class TestBudgetEnforcement:
    @pytest.mark.asyncio
    async def test_no_warning_at_full_budget(self):
        reset_all_buses()
        ag = AccountAgent(session_budget_usd=1.0)
        ag.start(ag._bus_ref if hasattr(ag, "_bus_ref") else EventBus("test"))
        # No spent → no warning
        assert not ag._warning_emitted

    @pytest.mark.asyncio
    async def test_warning_emitted_at_phi_inv2_remaining(self):
        """BUDGET_WARNING fires when ratio_remaining <= 0.382."""
        reset_all_buses()
        from cynic.core.event_bus import get_core_bus
        bus = get_core_bus()

        warned_events = []
        async def capture(e):
            warned_events.append(e)
        bus.on(CoreEvent.BUDGET_WARNING, capture)

        ag = AccountAgent(session_budget_usd=1.0)
        # Spend 62% → 38% remaining — just at threshold
        await ag._on_judgment(_make_judgment_event(cost_usd=0.62))

        # Wait one event loop turn for tasks
        import asyncio
        await asyncio.sleep(0)
        await asyncio.sleep(0)

        assert ag._warning_emitted
        reset_all_buses()

    @pytest.mark.asyncio
    async def test_warning_emitted_only_once(self):
        ag = AccountAgent(session_budget_usd=1.0)
        # Spend 100% in two steps
        await ag._on_judgment(_make_judgment_event(cost_usd=0.62))
        await ag._on_judgment(_make_judgment_event(cost_usd=0.38))
        # warning_emitted stays True (not re-set)
        assert ag._warning_emitted

    @pytest.mark.asyncio
    async def test_exhausted_emitted_when_spent_all(self):
        reset_all_buses()
        from cynic.core.event_bus import get_core_bus
        bus = get_core_bus()

        exhausted_events = []
        async def capture(e):
            exhausted_events.append(e)
        bus.on(CoreEvent.BUDGET_EXHAUSTED, capture)

        ag = AccountAgent(session_budget_usd=0.01)
        await ag._on_judgment(_make_judgment_event(cost_usd=0.02))

        import asyncio
        await asyncio.sleep(0)
        await asyncio.sleep(0)

        assert ag._exhausted_emitted
        reset_all_buses()

    @pytest.mark.asyncio
    async def test_no_warning_if_just_below_threshold(self):
        ag = AccountAgent(session_budget_usd=1.0)
        # Spend 61% → 39% remaining — above 38.2% warning threshold
        await ag._on_judgment(_make_judgment_event(cost_usd=0.61))
        assert not ag._warning_emitted

    @pytest.mark.asyncio
    async def test_warning_threshold_is_phi_inv2(self):
        assert abs(_WARNING_RATIO - PHI_INV_2) < 1e-6


# ── RUN score computation ──────────────────────────────────────────────────

class TestRunScore:
    def test_free_inference_gives_100(self):
        score = AccountAgent._compute_run_score(q_score=50.0, cost_usd=0.0)
        assert score == 100.0

    def test_zero_cost_gives_100_regardless_of_q(self):
        assert AccountAgent._compute_run_score(q_score=0.0, cost_usd=0.0) == 100.0
        assert AccountAgent._compute_run_score(q_score=100.0, cost_usd=0.0) == 100.0

    def test_high_cost_per_q_gives_zero(self):
        # cost_per_q = 0.02 / 1.0 = 0.02 = cap → run = 0
        score = AccountAgent._compute_run_score(q_score=1.0, cost_usd=0.02)
        assert score == 0.0

    def test_run_score_bounded(self):
        # Wildly expensive → clamped to 0
        score = AccountAgent._compute_run_score(q_score=0.1, cost_usd=1.0)
        assert score == 0.0

    def test_moderate_cost_gives_partial_score(self):
        # cost=$0.005, q=50 → cost_per_q=0.0001 → run ≈ 99.5
        score = AccountAgent._compute_run_score(q_score=50.0, cost_usd=0.005)
        assert 0.0 < score < 100.0


# ── EScore integration ─────────────────────────────────────────────────────

class TestEScoreIntegration:
    @pytest.mark.asyncio
    async def test_free_inference_sets_run_100_on_escore(self):
        """Ollama (cost=0) → RUN dimension updated to 100."""
        mock_tracker = MagicMock()
        ag = AccountAgent(escore_tracker=mock_tracker)

        await ag._on_judgment(_make_judgment_event(
            cost_usd=0.0,
            q_score=50.0,
            dog_votes={"SAGE": 50.0},
        ))

        mock_tracker.update.assert_called_once_with(
            "agent:SAGE", "RUN", 100.0, reality="CODE"
        )

    @pytest.mark.asyncio
    async def test_no_escore_no_error(self):
        """Works correctly without EScoreTracker injected."""
        ag = AccountAgent(escore_tracker=None)
        await ag._on_judgment(_make_judgment_event(
            cost_usd=0.0, dog_votes={"SAGE": 50.0}
        ))
        assert ag._judgment_count == 1

    @pytest.mark.asyncio
    async def test_escore_updated_per_dog(self):
        mock_tracker = MagicMock()
        ag = AccountAgent(escore_tracker=mock_tracker)

        await ag._on_judgment(_make_judgment_event(
            cost_usd=0.0,
            dog_votes={"SAGE": 50.0, "GUARDIAN": 40.0},
        ))

        assert mock_tracker.update.call_count == 2
        called_dogs = {c.args[0] for c in mock_tracker.update.call_args_list}
        assert "agent:SAGE" in called_dogs
        assert "agent:GUARDIAN" in called_dogs

    @pytest.mark.asyncio
    async def test_escore_update_error_is_silent(self):
        """Bad reality string → EScoreTracker raises → AgentAccount swallows."""
        mock_tracker = MagicMock()
        mock_tracker.update.side_effect = ValueError("bad reality")
        ag = AccountAgent(escore_tracker=mock_tracker)

        # Should not propagate
        await ag._on_judgment(_make_judgment_event(
            cost_usd=0.0, dog_votes={"SAGE": 50.0}
        ))
        assert ag._judgment_count == 1


# ── Lifecycle ──────────────────────────────────────────────────────────────

class TestAccountAgentLifecycle:
    def test_start_subscribes_to_bus(self):
        reset_all_buses()
        from cynic.core.event_bus import get_core_bus
        bus = get_core_bus()

        ag = AccountAgent()
        ag.start(bus)
        assert CoreEvent.JUDGMENT_CREATED in bus._handlers
        assert ag._handler in bus._handlers[CoreEvent.JUDGMENT_CREATED]

        ag.stop(bus)
        reset_all_buses()

    def test_stop_unsubscribes(self):
        reset_all_buses()
        from cynic.core.event_bus import get_core_bus
        bus = get_core_bus()

        ag = AccountAgent()
        ag.start(bus)
        ag.stop(bus)
        handlers = bus._handlers.get(CoreEvent.JUDGMENT_CREATED, [])
        assert ag._handler not in handlers
        reset_all_buses()

    def test_set_escore_tracker(self):
        ag = AccountAgent()
        mock = MagicMock()
        ag.set_escore_tracker(mock)
        assert ag._escore_tracker is mock
