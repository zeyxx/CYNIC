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
