"""
CYNIC SelfProber Tests (L4)

Tests SelfProposal creation, QTable/EScore/Residual analysis, rolling cap,
lifecycle (dismiss/apply), persistence, and event handler.
No LLM, no DB, no event bus — pure in-memory unit tests.
"""
from __future__ import annotations

import asyncio
import json
import os
import time
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.judge.self_probe import (
    SelfProber,
    SelfProposal,
    _MAX_PROPOSALS,
    _LOW_Q_THRESHOLD,
    _MIN_VISITS,
)
from cynic.core.event_bus import CoreEvent, Event
from cynic.core.phi import PHI_INV_2


# ── Helpers ───────────────────────────────────────────────────────────────────

def _prober_with_tmp(tmp_path) -> SelfProber:
    path = str(tmp_path / "self_proposals.json")
    return SelfProber(proposals_path=path)


def _make_qtable(entries: list) -> MagicMock:
    """
    Make a mock QTable with _table matching expected format.
    entries: [(state_key, action, value, visits)]
    """
    table = {}
    for sk, action, value, visits in entries:
        table.setdefault(sk, {})[action] = {"value": value, "visits": visits}
    qt = MagicMock()
    qt._table = table
    return qt


def _make_escore(entries: list) -> MagicMock:
    """
    entries: [(agent_id, judge_score)]
    agent_id should be "agent:DOG_NAME"
    """
    scores = {}
    for agent_id, judge_score in entries:
        scores[agent_id] = {"JUDGE": judge_score}
    es = MagicMock()
    es._scores = scores
    return es


def _make_residual() -> MagicMock:
    rd = MagicMock()
    return rd


def _make_emergence_event(pattern_type="SPIKE", severity=0.6) -> Event:
    return Event(
        type=CoreEvent.EMERGENCE_DETECTED,
        payload={"pattern_type": pattern_type, "severity": severity},
        source="residual_detector",
    )


# ── SelfProposal dataclass ────────────────────────────────────────────────────

class TestSelfProposal:
    def _make(self) -> SelfProposal:
        return SelfProposal(
            probe_id="abc12345",
            trigger="EMERGENCE",
            pattern_type="SPIKE",
            severity=0.6,
            dimension="QTABLE",
            target="CODE:JUDGE:PRESENT:0:WAG",
            recommendation="Investigate Q-value below threshold",
            current_value=0.25,
            suggested_value=0.382,
            proposed_at=1234567890.0,
            status="PENDING",
        )

    def test_default_status_pending(self):
        p = self._make()
        assert p.status == "PENDING"

    def test_to_dict_roundtrip(self):
        p = self._make()
        d = p.to_dict()
        p2 = SelfProposal.from_dict(d)
        assert p2.probe_id == p.probe_id
        assert p2.dimension == p.dimension
        assert p2.status == p.status
        assert abs(p2.current_value - p.current_value) < 1e-6

    def test_to_dict_has_all_fields(self):
        p = self._make()
        d = p.to_dict()
        for key in [
            "probe_id", "trigger", "pattern_type", "severity",
            "dimension", "target", "recommendation",
            "current_value", "suggested_value", "proposed_at", "status",
        ]:
            assert key in d

    def test_severity_rounded_in_dict(self):
        p = self._make()
        d = p.to_dict()
        assert isinstance(d["severity"], float)


# ── SelfProber: initial state ─────────────────────────────────────────────────

class TestSelfProberInit:
    def test_initial_empty(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        assert sp.all_proposals() == []
        assert sp.pending() == []

    def test_stats_initial(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        s = sp.stats()
        assert s["proposed_total"] == 0
        assert s["queue_size"] == 0
        assert s["pending"] == 0

    def test_get_unknown_returns_none(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        assert sp.get("deadbeef") is None


# ── QTable analysis ───────────────────────────────────────────────────────────

class TestQTableAnalysis:
    def test_no_qtable_returns_empty(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        result = sp._analyze_qtable("MANUAL", "SPIKE", 0.5)
        assert result == []

    def test_qtable_no_low_entries_returns_empty(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        qt = _make_qtable([
            ("CODE:JUDGE:PRESENT:0", "WAG", 0.9, 10),
            ("CODE:JUDGE:PRESENT:0", "HOWL", 0.8, 5),
        ])
        sp.set_qtable(qt)
        result = sp._analyze_qtable("MANUAL", "SPIKE", 0.5)
        assert result == []

    def test_qtable_below_threshold_generates_proposal(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        qt = _make_qtable([
            ("CODE:JUDGE:PRESENT:0", "BARK", 0.15, 5),  # LOW + enough visits
        ])
        sp.set_qtable(qt)
        result = sp._analyze_qtable("MANUAL", "SPIKE", 0.5)
        assert len(result) == 1
        assert result[0].dimension == "QTABLE"
        assert result[0].current_value == pytest.approx(0.15)

    def test_qtable_insufficient_visits_ignored(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        qt = _make_qtable([
            ("CODE:JUDGE:PRESENT:0", "BARK", 0.10, 0),  # too few visits
        ])
        sp.set_qtable(qt)
        result = sp._analyze_qtable("MANUAL", "SPIKE", 0.5)
        assert result == []

    def test_qtable_picks_worst_entry(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        qt = _make_qtable([
            ("STATE_A", "BARK",  0.20, 5),
            ("STATE_B", "GROWL", 0.10, 5),  # worse
            ("STATE_C", "WAG",   0.30, 5),  # above threshold
        ])
        sp.set_qtable(qt)
        result = sp._analyze_qtable("MANUAL", "SPIKE", 0.5)
        assert len(result) == 1
        assert result[0].current_value == pytest.approx(0.10)

    def test_qtable_proposal_has_correct_fields(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        qt = _make_qtable([("S:A:P:0", "BARK", 0.2, 4)])
        sp.set_qtable(qt)
        result = sp._analyze_qtable("EMERGENCE", "STABLE_HIGH", 0.8)
        assert result[0].trigger == "EMERGENCE"
        assert result[0].pattern_type == "STABLE_HIGH"
        assert result[0].severity == pytest.approx(0.8)
        assert "BARK" in result[0].target


# ── EScore analysis ───────────────────────────────────────────────────────────

class TestEScoreAnalysis:
    def test_no_escore_returns_empty(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        result = sp._analyze_escore("MANUAL", "SPIKE", 0.5)
        assert result == []

    def test_all_dogs_above_threshold_returns_empty(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        es = _make_escore([("agent:GUARDIAN", 80.0), ("agent:ANALYST", 70.0)])
        sp.set_escore_tracker(es)
        result = sp._analyze_escore("MANUAL", "SPIKE", 0.5)
        assert result == []

    def test_low_escore_generates_proposal(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        es = _make_escore([("agent:GUARDIAN", 20.0)])
        sp.set_escore_tracker(es)
        result = sp._analyze_escore("MANUAL", "SPIKE", 0.5)
        assert len(result) == 1
        assert result[0].dimension == "ESCORE"
        assert result[0].target == "GUARDIAN"
        assert result[0].current_value == pytest.approx(20.0)
        assert result[0].suggested_value == pytest.approx(38.2)

    def test_non_agent_entries_ignored(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        es = _make_escore([("user:HUMAN", 10.0), ("agent:ANALYST", 80.0)])
        sp.set_escore_tracker(es)
        result = sp._analyze_escore("MANUAL", "SPIKE", 0.5)
        assert result == []

    def test_escore_max_3_proposals(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        es = _make_escore([
            (f"agent:DOG_{i}", 10.0) for i in range(6)
        ])
        sp.set_escore_tracker(es)
        result = sp._analyze_escore("MANUAL", "SPIKE", 0.5)
        assert len(result) <= 3


# ── Residual / Config analysis ────────────────────────────────────────────────

class TestResidualAnalysis:
    def test_no_residual_returns_empty(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        result = sp._analyze_residual("MANUAL", "STABLE_HIGH", 0.6)
        assert result == []

    def test_spike_pattern_returns_empty(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_residual_detector(_make_residual())
        result = sp._analyze_residual("MANUAL", "SPIKE", 0.6)
        assert result == []  # SPIKE has no config suggestion

    def test_stable_high_generates_config_proposal(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_residual_detector(_make_residual())
        result = sp._analyze_residual("EMERGENCE", "STABLE_HIGH", 0.7)
        assert len(result) == 1
        assert result[0].dimension == "CONFIG"
        assert result[0].target == "axiom_weights"
        assert result[0].pattern_type == "STABLE_HIGH"

    def test_rising_generates_threshold_proposal(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_residual_detector(_make_residual())
        result = sp._analyze_residual("EMERGENCE", "RISING", 0.5)
        assert len(result) == 1
        assert result[0].target == "residual_threshold"
        assert result[0].current_value == pytest.approx(0.382)
        assert result[0].suggested_value == pytest.approx(0.300)


# ── analyze() integration ─────────────────────────────────────────────────────

class TestAnalyze:
    def test_analyze_returns_list(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        result = sp.analyze()
        assert isinstance(result, list)

    def test_analyze_stores_proposals(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))
        result = sp.analyze()
        assert len(sp.all_proposals()) == len(result)

    def test_analyze_increments_total(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))
        sp.analyze()
        assert sp.stats()["proposed_total"] >= 1

    def test_analyze_trigger_and_pattern_set(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))
        result = sp.analyze(trigger="EMERGENCE", pattern_type="SPIKE", severity=0.7)
        assert result[0].trigger == "EMERGENCE"
        assert result[0].pattern_type == "SPIKE"
        assert result[0].severity == pytest.approx(0.7)


# ── Rolling cap ───────────────────────────────────────────────────────────────

class TestRollingCap:
    def test_cap_enforced(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        qt = _make_qtable([("S", "BARK", 0.1, 5)])
        sp.set_qtable(qt)
        for _ in range(_MAX_PROPOSALS + 10):
            sp.analyze()
        assert len(sp.all_proposals()) == _MAX_PROPOSALS

    def test_oldest_evicted(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        qt = _make_qtable([("S", "BARK", 0.1, 5)])
        sp.set_qtable(qt)
        sp.analyze()
        first_id = sp.all_proposals()[0].probe_id

        for _ in range(_MAX_PROPOSALS):
            sp.analyze()

        ids = [p.probe_id for p in sp.all_proposals()]
        assert first_id not in ids


# ── Lifecycle ─────────────────────────────────────────────────────────────────

class TestLifecycle:
    def _setup(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))
        sp.analyze()
        return sp, sp.all_proposals()[0]

    def test_dismiss_changes_status(self, tmp_path):
        sp, p = self._setup(tmp_path)
        result = sp.dismiss(p.probe_id)
        assert result is not None
        assert result.status == "DISMISSED"

    def test_apply_changes_status(self, tmp_path):
        sp, p = self._setup(tmp_path)
        result = sp.apply(p.probe_id)
        assert result is not None
        assert result.status == "APPLIED"

    def test_dismissed_not_in_pending(self, tmp_path):
        sp, p = self._setup(tmp_path)
        sp.dismiss(p.probe_id)
        assert p.probe_id not in [x.probe_id for x in sp.pending()]

    def test_applied_not_in_pending(self, tmp_path):
        sp, p = self._setup(tmp_path)
        sp.apply(p.probe_id)
        assert p.probe_id not in [x.probe_id for x in sp.pending()]

    def test_unknown_dismiss_returns_none(self, tmp_path):
        sp, _ = self._setup(tmp_path)
        assert sp.dismiss("deadbeef") is None

    def test_get_by_id(self, tmp_path):
        sp, p = self._setup(tmp_path)
        found = sp.get(p.probe_id)
        assert found is not None
        assert found.probe_id == p.probe_id


# ── Persistence ───────────────────────────────────────────────────────────────

class TestPersistence:
    def test_save_creates_file(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))
        sp.analyze()
        assert os.path.exists(str(tmp_path / "self_proposals.json"))

    def test_saved_json_is_list(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))
        sp.analyze()
        with open(str(tmp_path / "self_proposals.json")) as fh:
            data = json.load(fh)
        assert isinstance(data, list)

    def test_second_instance_loads_proposals(self, tmp_path):
        sp1 = _prober_with_tmp(tmp_path)
        sp1.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))
        sp1.analyze()
        n = len(sp1.all_proposals())

        sp2 = _prober_with_tmp(tmp_path)
        assert len(sp2.all_proposals()) == n

    def test_dismiss_persists_to_disk(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))
        sp.analyze()
        p = sp.all_proposals()[0]
        sp.dismiss(p.probe_id)

        with open(str(tmp_path / "self_proposals.json")) as fh:
            data = json.load(fh)
        assert data[0]["status"] == "DISMISSED"


# ── Event handler ─────────────────────────────────────────────────────────────

class TestEventHandler:
    def test_on_emergence_generates_proposals(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))

        event = _make_emergence_event("SPIKE", 0.7)
        with patch("cynic.judge.self_probe.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(sp._on_emergence(event))

        assert len(sp.all_proposals()) >= 1

    def test_on_emergence_emits_event_when_proposals(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_qtable(_make_qtable([("S", "BARK", 0.1, 5)]))

        event = _make_emergence_event("SPIKE", 0.7)
        with patch("cynic.judge.self_probe.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(sp._on_emergence(event))
            mock_bus.return_value.emit.assert_called_once()

    def test_on_emergence_no_proposals_no_emit(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        # No qtable, no escore, no residual → no proposals
        event = _make_emergence_event("SPIKE", 0.7)
        with patch("cynic.judge.self_probe.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(sp._on_emergence(event))
            mock_bus.return_value.emit.assert_not_called()

    def test_on_emergence_stable_high_with_residual(self, tmp_path):
        sp = _prober_with_tmp(tmp_path)
        sp.set_residual_detector(_make_residual())

        event = _make_emergence_event("STABLE_HIGH", 0.8)
        with patch("cynic.judge.self_probe.get_core_bus") as mock_bus:
            mock_bus.return_value.emit = AsyncMock()
            asyncio.get_event_loop().run_until_complete(sp._on_emergence(event))

        # Should generate at least one CONFIG proposal
        configs = [p for p in sp.all_proposals() if p.dimension == "CONFIG"]
        assert len(configs) >= 1
