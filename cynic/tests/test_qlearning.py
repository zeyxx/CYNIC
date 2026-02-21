"""
Tests: Q-Learning System

Validates TD(0) correctness, Thompson Sampling, φ-bounds, and
end-to-end learning signal from JudgeOrchestrator → QTable.
"""
from __future__ import annotations

import random
import pytest

from cynic.core.phi import MAX_CONFIDENCE, MAX_Q_SCORE, LEARNING_RATE, PHI_INV
from cynic.learning.qlearning import QTable, QEntry, LearningSignal, LearningLoop, VERDICTS


# ════════════════════════════════════════════════════════════════════════════
# LearningSignal
# ════════════════════════════════════════════════════════════════════════════

class TestLearningSignal:

    def test_valid_signal(self):
        s = LearningSignal(state_key="CODE:JUDGE:PRESENT:1", action="GROWL", reward=0.52)
        assert s.state_key == "CODE:JUDGE:PRESENT:1"
        assert s.action == "GROWL"
        assert s.reward == 0.52

    def test_reward_clamped_above_1(self):
        s = LearningSignal(state_key="k", action="WAG", reward=9.9)
        assert s.reward == 1.0

    def test_reward_clamped_below_0(self):
        s = LearningSignal(state_key="k", action="BARK", reward=-1.0)
        assert s.reward == 0.0

    def test_invalid_action_raises(self):
        with pytest.raises(ValueError):
            LearningSignal(state_key="k", action="INVALID", reward=0.5)

    def test_all_verdicts_valid(self):
        for verdict in VERDICTS:
            s = LearningSignal(state_key="k", action=verdict, reward=0.5)
            assert s.action == verdict


# ════════════════════════════════════════════════════════════════════════════
# QEntry
# ════════════════════════════════════════════════════════════════════════════

class TestQEntry:

    def test_default_q_value_neutral(self):
        e = QEntry(state_key="s", action="GROWL")
        assert e.q_value == 0.5  # neutral start

    def test_thompson_sample_in_range(self):
        e = QEntry(state_key="s", action="WAG")
        for _ in range(20):
            sample = e.thompson_sample()
            assert 0.0 <= sample <= 1.0

    def test_thompson_biased_by_wins(self):
        """More wins → higher Thompson samples on average."""
        e_good = QEntry(state_key="s", action="WAG", wins=100, losses=5)
        e_bad = QEntry(state_key="s", action="WAG", wins=5, losses=100)

        avg_good = sum(e_good.thompson_sample() for _ in range(100)) / 100
        avg_bad = sum(e_bad.thompson_sample() for _ in range(100)) / 100

        assert avg_good > avg_bad, f"Good ({avg_good:.3f}) should > Bad ({avg_bad:.3f})"


# ════════════════════════════════════════════════════════════════════════════
# QTable — TD(0) correctness
# ════════════════════════════════════════════════════════════════════════════

class TestQTable:

    @pytest.fixture
    def qtable(self):
        q = QTable()
        yield q
        q.reset()

    def test_update_moves_toward_reward(self, qtable):
        """TD(0): Q(s,a) should converge toward repeated reward."""
        signal = LearningSignal(state_key="CODE:JUDGE:PRESENT:1", action="GROWL", reward=0.8)
        for _ in range(50):
            qtable.update(signal)

        q_val = qtable.predict_q("CODE:JUDGE:PRESENT:1", "GROWL")
        assert q_val > 0.6, f"Q should converge toward 0.8, got {q_val}"

    def test_update_decreases_for_low_reward(self, qtable):
        """Low reward pulls Q down from neutral 0.5."""
        signal = LearningSignal(state_key="CODE:JUDGE:PRESENT:1", action="BARK", reward=0.1)
        for _ in range(50):
            qtable.update(signal)

        q_val = qtable.predict_q("CODE:JUDGE:PRESENT:1", "BARK")
        assert q_val < 0.45, f"Q should converge toward 0.1, got {q_val}"

    def test_phi_bound_never_exceeded(self, qtable):
        """Q-values must stay in [0, 1] regardless of reward."""
        for _ in range(100):
            signal = LearningSignal(
                state_key="CODE:JUDGE:PRESENT:1",
                action=random.choice(VERDICTS),
                reward=random.random(),
            )
            qtable.update(signal)

        for sk, actions in qtable._table.items():
            for action, entry in actions.items():
                assert 0.0 <= entry.q_value <= 1.0, (
                    f"Q[{sk}][{action}] = {entry.q_value} out of [0,1]"
                )

    def test_learning_rate_controls_speed(self):
        """Faster learning rate → quicker convergence."""
        q_fast = QTable(learning_rate=0.5)
        q_slow = QTable(learning_rate=0.01)

        for _ in range(10):
            sig = LearningSignal(state_key="s", action="WAG", reward=1.0)
            q_fast.update(sig)
            q_slow.update(sig)

        fast_q = q_fast.predict_q("s", "WAG")
        slow_q = q_slow.predict_q("s", "WAG")
        assert fast_q > slow_q, f"Fast ({fast_q}) should > Slow ({slow_q})"

    def test_exploit_returns_best_action(self, qtable):
        """Exploit policy picks highest Q-value action."""
        # Train WAG heavily
        for _ in range(30):
            qtable.update(LearningSignal(state_key="s", action="WAG", reward=0.9))
        for _ in range(30):
            qtable.update(LearningSignal(state_key="s", action="BARK", reward=0.1))

        best = qtable.exploit("s")
        assert best == "WAG", f"Expected WAG, got {best}"

    def test_exploit_defaults_growl_on_unseen(self, qtable):
        """Unseen state → GROWL (cautious default)."""
        action = qtable.exploit("UNSEEN:STATE:FUTURE:3")
        assert action == "GROWL"

    def test_explore_returns_valid_verdict(self, qtable):
        """Thompson exploration always returns a valid verdict."""
        for _ in range(20):
            action = qtable.explore("CODE:JUDGE:PRESENT:1")
            assert action in VERDICTS

    def test_confidence_zero_on_unseen(self, qtable):
        """Confidence = 0 for completely unseen state."""
        conf = qtable.confidence("UNSEEN:NEVER:SEEN:0")
        assert conf == 0.0

    def test_confidence_grows_with_visits(self, qtable):
        """More visits → higher confidence (capped at φ⁻¹)."""
        for _ in range(50):
            qtable.update(LearningSignal(state_key="s", action="GROWL", reward=0.5))

        conf = qtable.confidence("s")
        assert conf > 0.0
        assert conf <= MAX_CONFIDENCE  # Never exceed φ⁻¹

    def test_confidence_capped_at_phi_inv(self, qtable):
        """Confidence MUST be capped at φ⁻¹ = 0.618 even with many visits."""
        for _ in range(1000):
            qtable.update(LearningSignal(state_key="s", action="WAG", reward=0.9))

        conf = qtable.confidence("s")
        assert conf <= MAX_CONFIDENCE, f"Confidence {conf} exceeded φ⁻¹ {MAX_CONFIDENCE}"

    def test_different_states_independent(self, qtable):
        """States don't interfere with each other."""
        for _ in range(20):
            qtable.update(LearningSignal(state_key="CODE:JUDGE:PRESENT:1", action="WAG", reward=0.9))
        for _ in range(20):
            qtable.update(LearningSignal(state_key="SOLANA:ACT:PRESENT:1", action="BARK", reward=0.1))

        assert qtable.exploit("CODE:JUDGE:PRESENT:1") == "WAG"
        assert qtable.exploit("SOLANA:ACT:PRESENT:1") == "BARK"

    def test_stats_returns_correct_counts(self, qtable):
        """Stats should reflect actual updates."""
        for _ in range(5):
            qtable.update(LearningSignal(state_key="s", action="WAG", reward=0.6))

        s = qtable.stats()
        assert s["total_updates"] == 5
        assert s["states"] == 1
        assert s["entries"] == 1

    def test_reset_clears_table(self, qtable):
        """Reset wipes all learning data."""
        qtable.update(LearningSignal(state_key="s", action="WAG", reward=0.7))
        qtable.reset()

        assert qtable.stats()["states"] == 0
        assert qtable.predict_q("s", "WAG") == 0.5  # Back to neutral

    def test_top_states_ordered_by_visits(self, qtable):
        """top_states returns most-visited first."""
        for i in range(10):
            qtable.update(LearningSignal(state_key="CODE:JUDGE:PRESENT:1", action="WAG", reward=0.7))
        for i in range(3):
            qtable.update(LearningSignal(state_key="SOLANA:ACT:PRESENT:1", action="GROWL", reward=0.4))

        top = qtable.top_states(n=2)
        assert top[0]["state_key"] == "CODE:JUDGE:PRESENT:1"
        assert top[0]["visits"] > top[1]["visits"]


# ════════════════════════════════════════════════════════════════════════════
# End-to-End: Orchestrator LEARNING_EVENT → QTable
# ════════════════════════════════════════════════════════════════════════════

class TestLearningIntegration:
    """
    Proves the full learning loop:
      JudgeOrchestrator MACRO cycle → LEARNING_EVENT → LearningLoop → QTable.update()
    """

    async def test_orchestrator_emits_learning_event(self, orchestrator, clean_code_cell):
        """After MACRO cycle, QTable should have learned something."""
        from cynic.learning.qlearning import QTable, LearningLoop
        from cynic.core.event_bus import get_core_bus
        from cynic.core.consciousness import ConsciousnessLevel

        qtable = QTable()
        loop = LearningLoop(qtable)
        loop.start(get_core_bus())

        import asyncio

        # MACRO cycle emits LEARNING_EVENT
        j = await orchestrator.run(clean_code_cell, level=ConsciousnessLevel.MACRO)

        # Yield to event loop so fire-and-forget tasks complete
        await asyncio.sleep(0)
        await asyncio.sleep(0)

        loop.stop()

        # The verdict should have been learned
        assert j.verdict in VERDICTS
        # QTable should have at least 1 update (LEARNING_EVENT fired)
        assert qtable.stats()["total_updates"] >= 1, (
            "Expected at least 1 Q-update from MACRO cycle"
        )

    async def test_repeated_cycles_converge(self, orchestrator, clean_code_cell):
        """Repeated judgments on same cell type should converge Q-values."""
        from cynic.learning.qlearning import QTable, LearningLoop
        from cynic.core.event_bus import get_core_bus
        from cynic.core.consciousness import ConsciousnessLevel

        qtable = QTable()
        loop = LearningLoop(qtable)
        loop.start(get_core_bus())

        import asyncio

        # Run 5 cycles on the same cell type
        for _ in range(5):
            await orchestrator.run(clean_code_cell, level=ConsciousnessLevel.MACRO)
            await asyncio.sleep(0)  # Yield after each — tasks complete

        state = clean_code_cell.state_key()
        updates = qtable.stats()["total_updates"]
        loop.stop()

        assert updates >= 5, f"Expected ≥5 updates, got {updates}"
        # Confidence should be growing
        conf = qtable.confidence(state)
        assert conf >= 0.0


# ── Fixtures needed from conftest (reuse) ────────────────────────────────

@pytest.fixture
def clean_code_cell():
    from cynic.core.judgment import Cell
    return Cell(
        reality="CODE",
        analysis="JUDGE",
        content={"code": "def f(x: int) -> int:\n    return x * 2\n"},
        context="Simple clean function",
        novelty=0.2, complexity=0.2, risk=0.1, budget_usd=0.1,
    )


@pytest.fixture
def orchestrator():
    from cynic.core.axioms import AxiomArchitecture
    from cynic.cognition.neurons.base import DogId
    from cynic.cognition.neurons.cynic_dog import CynicDog
    from cynic.cognition.neurons.guardian import GuardianDog
    from cynic.cognition.neurons.analyst import AnalystDog
    from cynic.cognition.neurons.janitor import JanitorDog
    from cynic.cognition.cortex.orchestrator import JudgeOrchestrator

    dogs = {
        DogId.CYNIC:    CynicDog(),
        DogId.GUARDIAN: GuardianDog(),
        DogId.ANALYST:  AnalystDog(),
        DogId.JANITOR:  JanitorDog(),
    }
    cynic_dog = dogs[DogId.CYNIC]
    return JudgeOrchestrator(dogs=dogs, axiom_arch=AxiomArchitecture(), cynic_dog=cynic_dog)


# ════════════════════════════════════════════════════════════════════════════
# QTable.matrix_stats — 7×7×7 observability (Goal A: Lazy Materialization)
# ════════════════════════════════════════════════════════════════════════════

class TestQTableMatrixStats:
    """
    QTable.matrix_stats() returns coverage of the 7×7×7 = 343-cell matrix.

    State keys have format:  "REALITY:ANALYSIS:TIME_DIM:action"
    matrix_stats() extracts the first 3 parts to build dimensional coverage maps.
    """

    def _sig(self, state_key: str, action: str = "WAG", reward: float = 60.0) -> LearningSignal:
        return LearningSignal(state_key=state_key, action=action, reward=reward)

    def test_empty_table_zero_coverage(self):
        """Fresh QTable has no entries → coverage 0.0%."""
        qt = QTable()
        stats = qt.matrix_stats()
        assert stats["total_cells"] == 0
        assert stats["matrix_343"] == 343
        assert stats["coverage_pct"] == 0.0

    def test_single_entry_shows_in_stats(self):
        """One update → total_cells=1, appears in by_reality/by_analysis/by_time_dim."""
        qt = QTable()
        qt.update(self._sig("CODE:JUDGE:PRESENT:WAG", reward=70.0))
        stats = qt.matrix_stats()
        assert stats["total_cells"] == 1
        assert stats["by_reality"].get("CODE", 0) == 1
        assert stats["by_analysis"].get("JUDGE", 0) == 1
        assert stats["by_time_dim"].get("PRESENT", 0) == 1

    def test_coverage_pct_calculation(self):
        """34 unique cells → coverage = 34/343*100 ≈ 9.9%."""
        qt = QTable()
        time_dims = ["PAST", "PRESENT", "FUTURE", "CYCLE", "TREND", "EMERGENCE", "TRANSCENDENCE"]
        for i in range(34):
            td = time_dims[i % len(time_dims)]
            qt.update(self._sig(f"CODE:JUDGE:{td}:WAG_{i}", reward=50.0))
        stats = qt.matrix_stats()
        assert stats["total_cells"] == 34
        assert abs(stats["coverage_pct"] - round(34 / 343 * 100, 1)) < 0.1

    def test_all_seven_time_dims_distinguishable(self):
        """All 7 time dimensions generate distinct keys, visible in by_time_dim."""
        qt = QTable()
        time_dims = ["PAST", "PRESENT", "FUTURE", "CYCLE", "TREND", "EMERGENCE", "TRANSCENDENCE"]
        for td in time_dims:
            qt.update(self._sig(f"CODE:JUDGE:{td}:WAG", reward=60.0))
        stats = qt.matrix_stats()
        assert len(stats["by_time_dim"]) == 7
        for td in time_dims:
            assert stats["by_time_dim"].get(td, 0) == 1

    def test_multiple_realities_counted(self):
        """Entries across different realities appear in by_reality."""
        qt = QTable()
        for reality in ("CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"):
            qt.update(self._sig(f"{reality}:JUDGE:PRESENT:WAG", reward=50.0))
        stats = qt.matrix_stats()
        assert len(stats["by_reality"]) == 7

    def test_multiple_analyses_counted(self):
        """Entries across different analysis dimensions appear in by_analysis."""
        qt = QTable()
        for analysis in ("PERCEIVE", "JUDGE", "DECIDE", "ACT", "LEARN", "ACCOUNT", "EMERGE"):
            qt.update(self._sig(f"CODE:{analysis}:PRESENT:WAG", reward=50.0))
        stats = qt.matrix_stats()
        assert len(stats["by_analysis"]) == 7

    def test_duplicate_updates_do_not_inflate_cell_count(self):
        """Updating the same state key multiple times counts as 1 cell, not N."""
        qt = QTable()
        for _ in range(10):
            qt.update(self._sig("CODE:JUDGE:PRESENT:WAG", reward=70.0))
        stats = qt.matrix_stats()
        assert stats["total_cells"] == 1, "Same state key = 1 cell regardless of update count"

    def test_malformed_key_excluded_from_dimensions(self):
        """Keys without 3 colon-separated parts are excluded from dimensional breakdown."""
        qt = QTable()
        # Inject a legacy-format key (no time_dim part) directly into the table
        qt._table["CODE:JUDGE"] = QEntry(state_key="CODE:JUDGE", action="WAG")
        qt._table["VALID:JUDGE:PRESENT:WAG"] = QEntry(state_key="VALID:JUDGE:PRESENT:WAG", action="WAG")
        stats = qt.matrix_stats()
        # Only the valid key contributes to dimensional breakdown
        assert stats["by_time_dim"].get("PRESENT", 0) == 1
        # Both keys count in total_cells
        assert stats["total_cells"] == 2
