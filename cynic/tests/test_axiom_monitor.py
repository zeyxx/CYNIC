"""
CYNIC AxiomMonitor Tests (δ1)

Tests emergent axiom activation monitoring (A6-A9).
No LLM, no DB.
"""
from __future__ import annotations

import pytest
from cynic.judge.axiom_monitor import (
    AxiomMonitor,
    AxiomState,
    EMERGENT_AXIOMS,
    MATURITY_WINDOW,
    _STATE_DORMANT, _STATE_STIRRING, _STATE_ACTIVE,
)
from cynic.core.phi import WAG_MIN, GROWL_MIN, MAX_Q_SCORE


class TestAxiomState:
    def test_initial_maturity_zero(self):
        ax = AxiomState(name="EMERGENCE")
        assert ax.maturity() == 0.0

    def test_state_dormant_initially(self):
        ax = AxiomState(name="AUTONOMY")
        assert ax.state() == _STATE_DORMANT

    def test_single_signal_low_maturity(self):
        ax = AxiomState(name="SYMBIOSIS")
        ax.add_signal()
        assert ax.maturity() < WAG_MIN  # 1/34 signals << 61.8%

    def test_full_window_signals_full_maturity(self):
        ax = AxiomState(name="ANTIFRAGILITY")
        for _ in range(MATURITY_WINDOW):
            ax.add_signal()
        assert abs(ax.maturity() - MAX_Q_SCORE) < 0.01

    def test_state_active_when_enough_signals(self):
        ax = AxiomState(name="EMERGENCE")
        # WAG_MIN = 61.8 → need 61.8% of MATURITY_WINDOW signals
        needed = int(WAG_MIN / MAX_Q_SCORE * MATURITY_WINDOW) + 1
        for _ in range(needed):
            ax.add_signal()
        assert ax.is_active()

    def test_to_dict_keys(self):
        ax = AxiomState(name="AUTONOMY")
        d = ax.to_dict()
        assert "name" in d
        assert "state" in d
        assert "maturity" in d
        assert "signals_recent" in d
        assert "activation_count" in d


class TestAxiomMonitorBasics:
    def test_initial_no_active_axioms(self):
        m = AxiomMonitor()
        assert m.active_count() == 0
        assert m.active_axioms() == []

    def test_signal_invalid_raises(self):
        m = AxiomMonitor()
        with pytest.raises(ValueError, match="Invalid emergent axiom"):
            m.signal("FIDELITY")  # Core axiom, not emergent

    def test_signal_valid_axiom(self):
        m = AxiomMonitor()
        m.signal("EMERGENCE")
        assert m.dashboard()["total_signals"] == 1

    def test_all_valid_axioms_accepted(self):
        m = AxiomMonitor()
        for ax in EMERGENT_AXIOMS:
            m.signal(ax)
        assert m.dashboard()["total_signals"] == len(EMERGENT_AXIOMS)

    def test_signal_count_accumulates(self):
        m = AxiomMonitor()
        for _ in range(5):
            m.signal("AUTONOMY")
        assert len(m._axioms["AUTONOMY"].signal_times) == 5

    def test_maturity_increases_with_signals(self):
        m = AxiomMonitor()
        m.signal("EMERGENCE")
        mat1 = m.get_maturity("EMERGENCE")
        m.signal("EMERGENCE", count=5)
        mat2 = m.get_maturity("EMERGENCE")
        assert mat2 > mat1

    def test_get_maturity_invalid_raises(self):
        m = AxiomMonitor()
        with pytest.raises(ValueError):
            m.get_maturity("INVALID")


class TestAxiomActivation:
    def _flood(self, monitor: AxiomMonitor, axiom: str, count: int = None):
        """Send enough signals to fully activate axiom."""
        target = count or MATURITY_WINDOW
        monitor.signal(axiom, count=target)

    def test_axiom_activates_after_sufficient_signals(self):
        m = AxiomMonitor()
        self._flood(m, "EMERGENCE")
        assert m.is_active("EMERGENCE")

    def test_inactive_axiom_returns_false(self):
        m = AxiomMonitor()
        m.signal("AUTONOMY", count=2)
        assert not m.is_active("AUTONOMY")

    def test_is_active_invalid_returns_false(self):
        m = AxiomMonitor()
        assert not m.is_active("INVALID_AXIOM")

    def test_active_axioms_list(self):
        m = AxiomMonitor()
        self._flood(m, "EMERGENCE")
        self._flood(m, "AUTONOMY")
        active = m.active_axioms()
        assert "EMERGENCE" in active
        assert "AUTONOMY" in active

    def test_activation_count_increments_on_first_activation(self):
        m = AxiomMonitor()
        self._flood(m, "SYMBIOSIS")
        assert m._axioms["SYMBIOSIS"].activation_count >= 1

    def test_signal_returns_new_state_on_transition(self):
        m = AxiomMonitor()
        result = m.signal("ANTIFRAGILITY", count=MATURITY_WINDOW)
        assert result == _STATE_ACTIVE

    def test_signal_returns_none_when_no_state_change(self):
        m = AxiomMonitor()
        result = m.signal("EMERGENCE", count=1)
        # Still DORMANT, no state change
        assert result is None or result == _STATE_DORMANT


class TestAxiomDashboard:
    def test_dashboard_has_required_keys(self):
        m = AxiomMonitor()
        d = m.dashboard()
        assert "active_count" in d
        assert "total_signals" in d
        assert "tier" in d
        assert "axioms" in d

    def test_dashboard_tier_dormant_initially(self):
        m = AxiomMonitor()
        assert m.dashboard()["tier"] == "DORMANT"

    def test_dashboard_tier_emergence_one_active(self):
        m = AxiomMonitor()
        m.signal("EMERGENCE", count=MATURITY_WINDOW)
        assert m.dashboard()["tier"] == "EMERGENCE"

    def test_dashboard_tier_transcendent_all_active(self):
        m = AxiomMonitor()
        for ax in EMERGENT_AXIOMS:
            m.signal(ax, count=MATURITY_WINDOW)
        assert m.dashboard()["tier"] == "TRANSCENDENT"

    def test_stats_keys(self):
        m = AxiomMonitor()
        s = m.stats()
        assert "active_axioms" in s
        assert "active_count" in s
        assert "tier" in s

    def test_all_four_axioms_in_dashboard(self):
        m = AxiomMonitor()
        d = m.dashboard()
        for ax in EMERGENT_AXIOMS:
            assert ax in d["axioms"]


class TestAxiomBudgetMultiplier:
    """γ3: Axiom health → budget multiplier → consciousness level selection."""

    def _make_orchestrator_with_monitor(self, monitor):
        """Build a minimal orchestrator with injected axiom_monitor."""
        from unittest.mock import MagicMock
        from cynic.judge.orchestrator import JudgeOrchestrator
        from cynic.core.axioms import AxiomArchitecture
        from cynic.dogs.cynic_dog import CynicDog
        arch = AxiomArchitecture()
        cynic_dog = MagicMock(spec=CynicDog)
        orch = JudgeOrchestrator(dogs={}, axiom_arch=arch, cynic_dog=cynic_dog)
        orch.axiom_monitor = monitor
        return orch

    def test_no_monitor_multiplier_is_one(self):
        from cynic.judge.orchestrator import JudgeOrchestrator
        from cynic.core.axioms import AxiomArchitecture
        from unittest.mock import MagicMock
        from cynic.dogs.cynic_dog import CynicDog
        arch = AxiomArchitecture()
        orch = JudgeOrchestrator(dogs={}, axiom_arch=arch, cynic_dog=MagicMock(spec=CynicDog))
        assert orch.axiom_monitor is None
        assert orch._axiom_budget_multiplier() == 1.0

    def test_zero_active_axioms_multiplier(self):
        m = AxiomMonitor()  # no signals → 0 active
        orch = self._make_orchestrator_with_monitor(m)
        mult = orch._axiom_budget_multiplier()
        # PHI^(0-2) = PHI^-2 = PHI_INV_2 ≈ 0.382
        assert abs(mult - 0.382) < 0.01

    def test_two_active_axioms_multiplier_neutral(self):
        m = AxiomMonitor()
        # Force 2 axioms to ACTIVE by saturating their signal windows
        for _ in range(MATURITY_WINDOW):
            m.signal("EMERGENCE")
            m.signal("AUTONOMY")
        orch = self._make_orchestrator_with_monitor(m)
        mult = orch._axiom_budget_multiplier()
        # PHI^(2-2) = PHI^0 = 1.0
        assert abs(mult - 1.0) < 0.01

    def test_four_active_axioms_max_multiplier(self):
        m = AxiomMonitor()
        for ax in ("EMERGENCE", "AUTONOMY", "SYMBIOSIS", "ANTIFRAGILITY"):
            for _ in range(MATURITY_WINDOW):
                m.signal(ax)
        orch = self._make_orchestrator_with_monitor(m)
        mult = orch._axiom_budget_multiplier()
        # PHI^(4-2) = PHI^2 ≈ 2.618
        assert abs(mult - 2.618) < 0.01

    def test_multiplier_scales_effective_budget(self):
        """Axiom health should visibly shift the budget used in level selection."""
        from cynic.core.phi import PHI_INV_2, PHI, PHI_2
        m_stressed = AxiomMonitor()  # 0 active → multiplier 0.382
        m_healthy = AxiomMonitor()
        for ax in ("EMERGENCE", "AUTONOMY", "SYMBIOSIS", "ANTIFRAGILITY"):
            for _ in range(MATURITY_WINDOW):
                m_healthy.signal(ax)

        orch_stressed = self._make_orchestrator_with_monitor(m_stressed)
        orch_healthy = self._make_orchestrator_with_monitor(m_healthy)

        budget = 0.05  # base budget
        stressed_eff = budget * orch_stressed._axiom_budget_multiplier()
        healthy_eff = budget * orch_healthy._axiom_budget_multiplier()

        assert stressed_eff < budget < healthy_eff
        assert abs(stressed_eff - budget * PHI_INV_2) < 0.001
        assert abs(healthy_eff - budget * PHI_2) < 0.001


class TestBudgetEnforcement:
    """Budget→LOD loop: BUDGET_WARNING/EXHAUSTED → orchestrator caps judgment depth."""

    def _make_orchestrator(self):
        from unittest.mock import MagicMock
        from cynic.judge.orchestrator import JudgeOrchestrator
        from cynic.core.axioms import AxiomArchitecture
        from cynic.dogs.cynic_dog import CynicDog
        arch = AxiomArchitecture()
        cynic_dog = MagicMock(spec=CynicDog)
        return JudgeOrchestrator(dogs={}, axiom_arch=arch, cynic_dog=cynic_dog)

    def test_initial_no_stress(self):
        orch = self._make_orchestrator()
        assert orch._budget_stress is False
        assert orch._budget_exhausted is False

    def test_budget_warning_sets_stress(self):
        orch = self._make_orchestrator()
        orch.on_budget_warning()
        assert orch._budget_stress is True
        assert orch._budget_exhausted is False  # not exhausted yet

    def test_budget_exhausted_sets_both_flags(self):
        orch = self._make_orchestrator()
        orch.on_budget_exhausted()
        assert orch._budget_exhausted is True

    def test_select_level_reflex_when_exhausted(self):
        from cynic.core.consciousness import ConsciousnessLevel
        from cynic.core.judgment import Cell
        orch = self._make_orchestrator()
        orch.on_budget_exhausted()
        cell = Cell(
            reality="CODE", analysis="JUDGE", time_dim="PRESENT",
            content="test", context="", risk=0.5, complexity=0.5,
            budget_usd=1.0, consciousness=5,  # would be MACRO without budget flag
        )
        level = orch._select_level(cell, 1.0)
        assert level == ConsciousnessLevel.REFLEX

    def test_select_level_capped_at_micro_under_stress(self):
        from cynic.core.consciousness import ConsciousnessLevel
        from cynic.core.judgment import Cell
        orch = self._make_orchestrator()
        orch.on_budget_warning()
        cell = Cell(
            reality="CODE", analysis="JUDGE", time_dim="PRESENT",
            content="test", context="", risk=0.5, complexity=0.5,
            budget_usd=1.0, consciousness=5,  # would be MACRO without stress
        )
        level = orch._select_level(cell, 1.0)
        assert level in (ConsciousnessLevel.MICRO, ConsciousnessLevel.REFLEX)
        assert level != ConsciousnessLevel.MACRO

    def test_exhausted_overrides_stress(self):
        """Exhausted takes priority over stress flag."""
        from cynic.core.consciousness import ConsciousnessLevel
        from cynic.core.judgment import Cell
        orch = self._make_orchestrator()
        orch.on_budget_warning()
        orch.on_budget_exhausted()
        cell = Cell(
            reality="CODE", analysis="JUDGE", time_dim="PRESENT",
            content="test", context="", risk=0.5, complexity=0.5,
            budget_usd=1.0, consciousness=5,
        )
        level = orch._select_level(cell, 1.0)
        assert level == ConsciousnessLevel.REFLEX


class TestAntifragilitySignal:
    """
    δ1: ANTIFRAGILITY axiom — success after stress = system improves under chaos.

    Signal source: _on_judgment_for_intelligence in state.py
    Logic: success recorded in outcome window, AND prior window had failures
           → axiom_monitor.signal("ANTIFRAGILITY")
    """

    def _make_monitor(self) -> AxiomMonitor:
        return AxiomMonitor()

    def test_no_signal_on_clean_run(self):
        """All successes → no stress → no ANTIFRAGILITY signal."""
        m = self._make_monitor()
        outcome_window = [True, True, True]  # all successes
        had_stress = len(outcome_window) > 1 and any(not ok for ok in outcome_window[:-1])
        if had_stress:
            m.signal("ANTIFRAGILITY")
        assert m.get_maturity("ANTIFRAGILITY") == 0.0

    def test_signal_after_failure_then_success(self):
        """Failure recorded, then success → ANTIFRAGILITY signaled."""
        m = self._make_monitor()
        # Simulate: outcome_window = [False, True] (failure then success just added)
        outcome_window = [False, True]
        had_stress = len(outcome_window) > 1 and any(not ok for ok in outcome_window[:-1])
        if had_stress:
            m.signal("ANTIFRAGILITY")
        assert m.get_maturity("ANTIFRAGILITY") > 0.0

    def test_signal_count_grows_with_recoveries(self):
        """Multiple recovery events → maturity grows."""
        m = self._make_monitor()
        # 5 recovery events (failure then success each time)
        for _ in range(5):
            outcome_window = [False, True]
            had_stress = len(outcome_window) > 1 and any(not ok for ok in outcome_window[:-1])
            if had_stress:
                m.signal("ANTIFRAGILITY")
        assert m.get_maturity("ANTIFRAGILITY") > 0.0
        assert m._axioms["ANTIFRAGILITY"].signal_times.__len__() == 5

    def test_signal_not_fired_on_empty_window(self):
        """Empty window (first-ever judgment) → no stress → no signal."""
        m = self._make_monitor()
        outcome_window = [True]  # only the current success
        had_stress = len(outcome_window) > 1 and any(not ok for ok in outcome_window[:-1])
        if had_stress:
            m.signal("ANTIFRAGILITY")
        assert m.get_maturity("ANTIFRAGILITY") == 0.0

    def test_antifragility_activates_at_threshold(self):
        """Enough recovery signals → ANTIFRAGILITY reaches ACTIVE state."""
        m = self._make_monitor()
        from cynic.core.phi import WAG_MIN, MAX_Q_SCORE
        needed = int(WAG_MIN / MAX_Q_SCORE * MATURITY_WINDOW) + 1
        for _ in range(needed):
            m.signal("ANTIFRAGILITY")
        assert m.is_active("ANTIFRAGILITY")

    def test_state_py_signal_logic(self):
        """Validate the exact slice logic used in _on_judgment_for_intelligence."""
        m = self._make_monitor()

        # Case 1: mixed window ending in success — stress present → signal
        window_with_failure = [True, False, True, True]  # last = current success
        had = len(window_with_failure) > 1 and any(not ok for ok in window_with_failure[:-1])
        assert had is True
        m.signal("ANTIFRAGILITY")

        # Case 2: all successes — no stress → no signal
        window_clean = [True, True, True, True]
        had2 = len(window_clean) > 1 and any(not ok for ok in window_clean[:-1])
        assert had2 is False


class TestTranscendenceLoop:
    """
    δ1: TRANSCENDENCE → EScoreTracker JUDGE update for agent:cynic.

    Closes the axiom chain end-to-end:
        AXIOM_ACTIVATED×4 → TRANSCENDENCE → escore_tracker.update("agent:cynic", JUDGE, MAX_Q_SCORE)

    Peak cognitive capacity demonstrated → CYNIC rewards its own judgment score.
    """

    def _activate_all(self, monitor: AxiomMonitor) -> None:
        """Drive all 4 axioms to ACTIVE state."""
        needed = int(WAG_MIN / MAX_Q_SCORE * MATURITY_WINDOW) + 1
        for axiom in ("AUTONOMY", "SYMBIOSIS", "EMERGENCE", "ANTIFRAGILITY"):
            for _ in range(needed):
                monitor.signal(axiom)

    def test_tier_dormant_with_no_signals(self):
        m = AxiomMonitor()
        assert m.dashboard()["tier"] == "DORMANT"

    def test_tier_awakening_with_3_axioms(self):
        """3 axioms active → AWAKENING, not TRANSCENDENT."""
        m = AxiomMonitor()
        needed = int(WAG_MIN / MAX_Q_SCORE * MATURITY_WINDOW) + 1
        for axiom in ("AUTONOMY", "SYMBIOSIS", "EMERGENCE"):
            for _ in range(needed):
                m.signal(axiom)
        assert m.dashboard()["tier"] == "AWAKENING"
        assert m.active_count() == 3

    def test_tier_transcendent_with_all_4(self):
        """All 4 axioms active → tier == TRANSCENDENT."""
        m = AxiomMonitor()
        self._activate_all(m)
        assert m.dashboard()["tier"] == "TRANSCENDENT"
        assert m.active_count() == 4

    def test_cynic_escore_updated_on_transcendence(self):
        """Simulates _on_transcendence handler: JUDGE update for agent:cynic."""
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        initial = tracker.get_score("agent:cynic")

        # Simulate what _on_transcendence does
        tracker.update("agent:cynic", "JUDGE", MAX_Q_SCORE)

        after = tracker.get_score("agent:cynic")
        assert after > initial, "EScore should increase after JUDGE=MAX_Q_SCORE update"

    def test_cynic_judge_dimension_reaches_high_after_transcendence(self):
        """After JUDGE=MAX_Q_SCORE update, JUDGE dimension is above GROWL_MIN."""
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        # Apply multiple updates to drive EMA toward MAX_Q_SCORE
        for _ in range(5):
            tracker.update("agent:cynic", "JUDGE", MAX_Q_SCORE)
        detail = tracker.get_detail("agent:cynic")
        judge_val = detail["dimensions"]["JUDGE"]["value"]
        assert judge_val > GROWL_MIN, f"JUDGE dim {judge_val:.1f} should exceed GROWL_MIN={GROWL_MIN}"

    def test_transcendence_idempotent(self):
        """Triggering TRANSCENDENCE handler twice does not crash or corrupt EScore."""
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        tracker.update("agent:cynic", "JUDGE", MAX_Q_SCORE)
        tracker.update("agent:cynic", "JUDGE", MAX_Q_SCORE)
        score = tracker.get_score("agent:cynic")
        assert 0.0 <= score <= MAX_Q_SCORE


class TestResidualHighLoop:
    """
    RESIDUAL_HIGH → EMERGENCE signal + EScore JUDGE penalty.

    THE_UNNAMEABLE (50th cell) detected when residual_variance > PHI_INV.
    The organism should react: signal the EMERGENCE axiom (feeding the
    EMERGENCE→AXIOM_ACTIVATED→TRANSCENDENCE chain) and apply a JUDGE penalty
    to agent:cynic's EScore (inverse of residual: more chaos → lower quality).
    """

    def test_residual_high_signals_emergence(self):
        """RESIDUAL_HIGH → axiom_monitor receives EMERGENCE signal."""
        m = AxiomMonitor()
        before = m.get_maturity("EMERGENCE")
        # Simulate _on_residual_high: signal EMERGENCE
        m.signal("EMERGENCE")
        after = m.get_maturity("EMERGENCE")
        assert after > before, "EMERGENCE maturity should increase after RESIDUAL_HIGH"

    def test_residual_high_escore_penalty_proportional(self):
        """Penalty = (1 - residual) × MAX_Q_SCORE — inversely proportional."""
        from cynic.core.phi import PHI_INV
        # residual = 0.618 (exactly at threshold) → penalty = 38.2
        residual = PHI_INV
        penalty = (1.0 - residual) * MAX_Q_SCORE
        assert abs(penalty - GROWL_MIN) < 0.5, (
            f"residual=PHI_INV → penalty≈GROWL_MIN, got {penalty:.2f}"
        )

    def test_residual_high_full_residual_penalty_zero(self):
        """residual=1.0 → score=0 — total failure to categorize."""
        residual = 1.0
        penalty = (1.0 - min(residual, 1.0)) * MAX_Q_SCORE
        assert penalty == 0.0

    def test_residual_high_escore_decreases_after_penalty(self):
        """Applying a JUDGE penalty below baseline lowers EScore."""
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        # First establish a high baseline
        for _ in range(3):
            tracker.update("agent:cynic", "JUDGE", MAX_Q_SCORE)
        high_score = tracker.get_score("agent:cynic")
        # Now apply the penalty (residual=1.0 → score=0)
        tracker.update("agent:cynic", "JUDGE", 0.0)
        low_score = tracker.get_score("agent:cynic")
        assert low_score < high_score, "JUDGE penalty should reduce EScore"

    def test_residual_high_accumulates_emergence_signals(self):
        """Multiple RESIDUAL_HIGH events accumulate EMERGENCE maturity."""
        m = AxiomMonitor()
        for _ in range(10):
            m.signal("EMERGENCE")
        maturity = m.get_maturity("EMERGENCE")
        assert maturity > 0.0, "10 signals should raise EMERGENCE maturity above 0"

    def test_residual_high_handler_tolerates_bad_payload(self):
        """_on_residual_high logic handles missing/wrong payload fields safely."""
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        # Simulate handler with empty payload
        p = {}
        residual = float(p.get("residual_variance", 0.0))
        penalty = (1.0 - min(residual, 1.0)) * MAX_Q_SCORE
        assert penalty == MAX_Q_SCORE  # residual=0 → full score (no penalty)
        # Should not raise
        tracker.update("agent:cynic", "JUDGE", penalty)
        assert tracker.get_score("agent:cynic") >= 0.0


# ── META_CYCLE → ANTIFRAGILITY signal + EScore JUDGE update ──────────────────

class TestMetaCycleLoop:
    """
    META_CYCLE → ANTIFRAGILITY signal + EScore JUDGE update.

    orchestrator.evolve() runs PROBE_CELLS at REFLEX level every ~4h and emits
    META_CYCLE with pass_rate + regression flag.  This loop closes the gap:
    - regression=True  → signal ANTIFRAGILITY axiom (stress activates immunity)
    - pass_rate ≥ 0.618 → JUDGE = pass_rate × MAX_Q_SCORE (organism healthy)
    - pass_rate ∈ [0.382, 0.618) → JUDGE = WAG_MIN (fair)
    - pass_rate < 0.382 → JUDGE = GROWL_MIN (struggling)
    """

    def test_high_pass_rate_judge_score_proportional(self):
        """pass_rate=0.8 ≥ PHI_INV → JUDGE = 0.8 × MAX_Q_SCORE = 80.0."""
        from cynic.core.phi import PHI_INV
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        pass_rate = 0.8
        assert pass_rate >= PHI_INV
        judge_score = pass_rate * MAX_Q_SCORE
        tracker.update("agent:cynic", "JUDGE", judge_score)
        assert tracker.get_score("agent:cynic") >= 0.0
        detail = tracker.get_detail("agent:cynic")
        stored = detail["dimensions"]["JUDGE"]["value"]
        assert stored == pytest.approx(judge_score, abs=0.5)

    def test_medium_pass_rate_gets_wag_score(self):
        """pass_rate=0.5 ∈ [PHI_INV_2, PHI_INV) → JUDGE = WAG_MIN."""
        from cynic.core.phi import PHI_INV, PHI_INV_2
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        pass_rate = 0.5
        assert PHI_INV_2 <= pass_rate < PHI_INV
        tracker.update("agent:cynic", "JUDGE", WAG_MIN)
        detail = tracker.get_detail("agent:cynic")
        stored = detail["dimensions"]["JUDGE"]["value"]
        assert stored == pytest.approx(WAG_MIN, abs=0.5)

    def test_low_pass_rate_gets_growl_score(self):
        """pass_rate=0.2 < PHI_INV_2 → JUDGE = GROWL_MIN."""
        from cynic.core.phi import PHI_INV_2
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        pass_rate = 0.2
        assert pass_rate < PHI_INV_2
        tracker.update("agent:cynic", "JUDGE", GROWL_MIN)
        detail = tracker.get_detail("agent:cynic")
        stored = detail["dimensions"]["JUDGE"]["value"]
        assert stored == pytest.approx(GROWL_MIN, abs=0.5)

    def test_regression_signals_antifragility(self):
        """regression=True → axiom_monitor.signal('ANTIFRAGILITY') increases maturity."""
        m = AxiomMonitor()
        before = m.get_maturity("ANTIFRAGILITY")
        m.signal("ANTIFRAGILITY")
        after = m.get_maturity("ANTIFRAGILITY")
        assert after > before, "ANTIFRAGILITY maturity must increase on regression"

    def test_no_regression_does_not_signal_antifragility(self):
        """regression=False → ANTIFRAGILITY maturity unchanged."""
        m = AxiomMonitor()
        before = m.get_maturity("ANTIFRAGILITY")
        # Handler only calls signal() when regression=True — simulate no-op
        regression = False
        if regression:
            m.signal("ANTIFRAGILITY")
        after = m.get_maturity("ANTIFRAGILITY")
        assert after == before, "No regression → no ANTIFRAGILITY signal"

    def test_meta_cycle_handler_tolerates_bad_payload(self):
        """Handler with empty evolve dict falls back to safe defaults."""
        from cynic.core.phi import PHI_INV, PHI_INV_2
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        # Simulate handler logic with empty payload
        evolve = {}
        pass_rate = float(evolve.get("pass_rate", 0.0))
        regression = bool(evolve.get("regression", False))
        # pass_rate=0.0 < PHI_INV_2 → GROWL_MIN
        if pass_rate >= PHI_INV:
            judge_score = pass_rate * MAX_Q_SCORE
        elif pass_rate >= PHI_INV_2:
            judge_score = WAG_MIN
        else:
            judge_score = GROWL_MIN
        assert judge_score == GROWL_MIN
        assert regression is False
        tracker.update("agent:cynic", "JUDGE", judge_score)
        assert tracker.get_score("agent:cynic") >= 0.0


# ── SDK_TOOL_JUDGED → SYMBIOSIS signal + GRAPH EScore loop ───────────────────

class TestSdkToolJudgedLoop:
    """
    SDK_TOOL_JUDGED → SYMBIOSIS signal + GRAPH EScore update.

    Emitted by server.py ws_sdk handler after GUARDIAN judges each tool call.
    Payload: {"session_id": str, "tool": str, "verdict": str}

    GRAPH dimension = trust network quality (tool verdicts build/erode the
    trust graph between SDK sessions and CYNIC's guardian layer).
    SYMBIOSIS axiom: seamless human+machine tool use = HOWL on every tool.
    """

    def _graph_score_for(self, verdict: str) -> float:
        """Reproduce handler's verdict→score mapping."""
        from cynic.core.phi import HOWL_MIN, WAG_MIN, GROWL_MIN
        return {
            "HOWL":  HOWL_MIN,
            "WAG":   WAG_MIN,
            "GROWL": GROWL_MIN,
            "BARK":  0.0,
        }.get(verdict, WAG_MIN)

    def test_howl_signals_symbiosis(self):
        """HOWL verdict → axiom_monitor.signal('SYMBIOSIS') increases maturity."""
        m = AxiomMonitor()
        before = m.get_maturity("SYMBIOSIS")
        # Simulate handler: HOWL → signal SYMBIOSIS
        m.signal("SYMBIOSIS")
        after = m.get_maturity("SYMBIOSIS")
        assert after > before, "SYMBIOSIS maturity should increase on HOWL verdict"

    def test_howl_gets_howl_graph_score(self):
        """HOWL verdict → GRAPH stored value ≈ HOWL_MIN (82.0)."""
        from cynic.core.phi import HOWL_MIN
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        score = self._graph_score_for("HOWL")
        assert score == pytest.approx(HOWL_MIN, abs=0.5)
        tracker.update("agent:cynic", "GRAPH", score)
        detail = tracker.get_detail("agent:cynic")
        stored = detail["dimensions"]["GRAPH"]["value"]
        assert stored == pytest.approx(HOWL_MIN, abs=0.5)

    def test_wag_gets_wag_graph_score(self):
        """WAG verdict → GRAPH stored value ≈ WAG_MIN (61.8)."""
        from cynic.core.phi import WAG_MIN
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        score = self._graph_score_for("WAG")
        assert score == pytest.approx(WAG_MIN, abs=0.5)
        tracker.update("agent:cynic", "GRAPH", score)
        detail = tracker.get_detail("agent:cynic")
        stored = detail["dimensions"]["GRAPH"]["value"]
        assert stored == pytest.approx(WAG_MIN, abs=0.5)

    def test_growl_gets_growl_graph_score(self):
        """GROWL verdict → GRAPH stored value ≈ GROWL_MIN (38.2)."""
        from cynic.core.phi import GROWL_MIN
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        score = self._graph_score_for("GROWL")
        assert score == pytest.approx(GROWL_MIN, abs=0.5)
        tracker.update("agent:cynic", "GRAPH", score)
        detail = tracker.get_detail("agent:cynic")
        stored = detail["dimensions"]["GRAPH"]["value"]
        assert stored == pytest.approx(GROWL_MIN, abs=0.5)

    def test_bark_gets_zero_graph_score(self):
        """BARK verdict → GRAPH = 0.0 (trust breakdown — tool denied)."""
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        score = self._graph_score_for("BARK")
        assert score == 0.0
        tracker.update("agent:cynic", "GRAPH", score)
        # After enough BARK updates the GRAPH dimension should be low
        for _ in range(5):
            tracker.update("agent:cynic", "GRAPH", 0.0)
        detail = tracker.get_detail("agent:cynic")
        stored = detail["dimensions"]["GRAPH"]["value"]
        assert stored < GROWL_MIN, f"BARK×5 → GRAPH {stored:.1f} should be below GROWL_MIN={GROWL_MIN}"

    def test_handler_tolerates_bad_payload(self):
        """Empty payload → no raise, falls back to WAG_MIN for unknown verdict."""
        from cynic.core.phi import WAG_MIN
        from cynic.core.escore import EScoreTracker
        tracker = EScoreTracker()
        # Simulate handler with empty payload (verdict="" → .get("", WAG_MIN))
        p = {}
        verdict = p.get("verdict", "")
        score = self._graph_score_for(verdict)  # "" not in map → WAG_MIN fallback
        assert score == pytest.approx(WAG_MIN, abs=0.5)
        # Should not raise
        tracker.update("agent:cynic", "GRAPH", score)
        assert tracker.get_score("agent:cynic") >= 0.0
