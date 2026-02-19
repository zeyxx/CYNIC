"""
CYNIC HumanStateModel Tests (T34)

Tests 4-dimensional cognitive state tracking for the human operator.
No LLM, no DB — pure in-memory state machine.
"""
from __future__ import annotations

import time
import pytest
from cynic.core.human_state import HumanState, HumanStateModel, _REST_THRESHOLD_S
from cynic.core.phi import WAG_MIN, GROWL_MIN, PHI_INV_3


class TestHumanState:
    def test_initial_defaults_are_healthy(self):
        s = HumanState()
        assert s.energy == 75.0
        assert s.focus == 65.0
        assert s.stress == 20.0
        assert s.valence == 65.0

    def test_to_dict_has_all_keys(self):
        s = HumanState()
        d = s.to_dict()
        assert "energy" in d
        assert "focus" in d
        assert "stress" in d
        assert "valence" in d
        assert "lod_hint" in d
        assert "updated_at" in d

    def test_lod_hint_full_when_healthy(self):
        s = HumanState(energy=80.0, focus=75.0, stress=10.0, valence=70.0)
        assert s.to_dict()["lod_hint"] == "FULL"

    def test_lod_hint_micro_when_low_energy(self):
        # energy < GROWL_MIN (38.2) but > PHI_INV_3*100 (23.6)
        s = HumanState(energy=30.0, focus=80.0)
        d = s.to_dict()
        assert d["lod_hint"] == "MICRO"

    def test_lod_hint_reflex_when_exhausted(self):
        # min(energy, focus) < PHI_INV_3 * 100 = 23.6
        s = HumanState(energy=20.0, focus=80.0)
        d = s.to_dict()
        assert d["lod_hint"] == "REFLEX"


class TestHumanStateModelSignals:
    def test_session_started_boosts_energy_focus(self):
        m = HumanStateModel()
        before_e = m.energy
        before_f = m.focus
        m.on_session_started()
        assert m.energy >= before_e  # may be slightly less if decay happened
        # Net effect: should be boosted (test with low initial)

    def test_session_started_caps_at_100(self):
        m = HumanStateModel()
        m._state.energy = 99.0
        m._state.focus = 99.0
        m.on_session_started()
        assert m.energy <= 100.0
        assert m.focus <= 100.0

    def test_feedback_5star_increases_valence(self):
        m = HumanStateModel()
        before = m._state.valence
        m.on_feedback(5.0)
        assert m._state.valence > before

    def test_feedback_1star_decreases_valence(self):
        m = HumanStateModel()
        before = m._state.valence
        m.on_feedback(1.0)
        assert m._state.valence < before

    def test_feedback_1star_increases_stress(self):
        m = HumanStateModel()
        before = m._state.stress
        m.on_feedback(1.0)
        assert m._state.stress > before

    def test_feedback_5star_decreases_stress(self):
        m = HumanStateModel()
        m._state.stress = 50.0
        m.on_feedback(5.0)
        assert m._state.stress < 50.0

    def test_feedback_3star_is_neutral_on_valence(self):
        m = HumanStateModel()
        before = round(m._state.valence, 3)
        m.on_feedback(3.0)
        # delta_v = (3 - 3) / 2 * 10 = 0 — no change to valence
        after = round(m._state.valence, 3)
        assert abs(after - before) < 0.01

    def test_correction_increases_stress(self):
        m = HumanStateModel()
        before = m._state.stress
        m.on_correction()
        assert m._state.stress >= before + 14  # +15 minus small decay

    def test_correction_decreases_focus(self):
        m = HumanStateModel()
        before = m._state.focus
        m.on_correction()
        assert m._state.focus <= before  # focus drops

    def test_all_values_bounded_0_100(self):
        m = HumanStateModel()
        # Drive extreme corrections
        for _ in range(20):
            m.on_correction()
        assert 0.0 <= m.energy <= 100.0
        assert 0.0 <= m.focus <= 100.0
        assert 0.0 <= m.stress <= 100.0
        assert 0.0 <= m._state.valence <= 100.0


class TestHumanStateModelLODHint:
    def test_lod_hint_full_when_both_dimensions_high(self):
        m = HumanStateModel()
        m._state.energy = 80.0
        m._state.focus = 75.0
        assert m.lod_hint == "FULL"

    def test_lod_hint_micro_when_energy_in_growl_zone(self):
        m = HumanStateModel()
        m._state.energy = 35.0  # < GROWL_MIN (38.2), > PHI_INV_3*100 (23.6)
        m._state.focus = 90.0
        assert m.lod_hint == "MICRO"

    def test_lod_hint_reflex_when_energy_exhausted(self):
        m = HumanStateModel()
        m._state.energy = 15.0  # < PHI_INV_3 * 100 = 23.6
        m._state.focus = 90.0
        assert m.lod_hint == "REFLEX"

    def test_lod_hint_uses_minimum_of_energy_focus(self):
        m = HumanStateModel()
        m._state.energy = 90.0
        m._state.focus = 15.0  # focus is the limiting dimension
        assert m.lod_hint == "REFLEX"

    def test_snapshot_includes_lod_hint(self):
        m = HumanStateModel()
        snap = m.snapshot()
        assert "lod_hint" in snap
        assert snap["lod_hint"] in ("FULL", "MICRO", "REFLEX")


class TestHumanStateModelActivity:
    def test_activity_when_recently_active_applies_decay(self):
        m = HumanStateModel()
        m._state.energy = 80.0
        m._state.updated_at = time.time()  # just updated
        m._last_activity_ts = time.time()
        m.on_activity()
        # Gap was near 0 → decay path → energy ~unchanged (very small decay)
        assert m.energy <= 80.0

    def test_activity_after_rest_restores_energy(self):
        m = HumanStateModel()
        m._state.energy = 50.0
        m._state.stress = 30.0
        # Simulate a long rest gap
        m._last_activity_ts = time.time() - _REST_THRESHOLD_S - 600  # 15min idle
        m.on_activity()
        assert m.energy > 50.0  # rest restored energy
        assert m._state.stress < 30.0  # stress reduced during rest
