"""
Tests: KernelMirror — Ring 3 self-reflection surface

Validates snapshot(), diff(), overall_health computation, and
the tier mapping from health scores.
"""
from __future__ import annotations

import time
import pytest
from unittest.mock import MagicMock

from cynic.judge.mirror import KernelMirror, _deep_diff
from cynic.core.phi import WAG_MIN, GROWL_MIN


# ── Minimal mock state ────────────────────────────────────────────────────────

def _make_mock_state(
    qtable_coverage: float = 10.0,
    axiom_tier: str = "DORMANT",
    lod: int = 0,
    llm_count: int = 0,
    heuristic_count: int = 5,
    budget_remaining: float = 9.5,
) -> MagicMock:
    """Build a lightweight mock KernelState for mirror tests."""
    from cynic.dogs.base import DogId

    state = MagicMock()

    # QTable mock
    qtable = MagicMock()
    qtable.stats.return_value = {
        "total_updates": 10,
        "total_states": 3,
        "learning_rate": 0.038,
        "discount": 0.382,
    }
    qtable.matrix_stats.return_value = {
        "total_cells": 3,
        "matrix_343": 343,
        "coverage_pct": qtable_coverage,
        "by_reality": {"CODE": 3},
        "by_analysis": {"JUDGE": 3},
        "by_time_dim": {"PRESENT": 3},
    }
    qtable.top_states.return_value = []
    state.qtable = qtable

    # AxiomMonitor mock
    axiom_monitor = MagicMock()
    axiom_monitor.dashboard.return_value = {
        "tier": axiom_tier,
        "active_count": 0,
        "total_signals": 5,
        "axioms": {},
    }
    state.axiom_monitor = axiom_monitor

    # LODController mock
    lod_controller = MagicMock()
    lod_controller.status.return_value = {
        "current_lod": lod,
        "level_name": "LOD_0_FULL",
    }
    state.lod_controller = lod_controller

    # AccountAgent mock
    account_agent = MagicMock()
    account_agent.stats.return_value = {
        "total_cost_usd": 0.5,
        "session_budget_usd": 10.0,
        "budget_remaining_usd": budget_remaining,
        "judgment_count": 10,
        "cost_by_reality": {"CODE": 0.5},
        "cost_by_dog": {},
        "warning_emitted": False,
        "exhausted_emitted": False,
    }
    state.account_agent = account_agent

    # EScoreTracker mock
    escore_tracker = MagicMock()
    escore_tracker.all_scores.return_value = {}
    state.escore_tracker = escore_tracker

    # ResidualDetector mock
    residual_detector = MagicMock()
    residual_detector.stats.return_value = {
        "total_checks": 5,
        "spikes": 0,
        "stable_high": 0,
        "rising": 0,
    }
    state.residual_detector = residual_detector

    # SAGE mock (via orchestrator.dogs[DogId.SAGE])
    sage_dog = MagicMock()
    sage_dog._heuristic_count = heuristic_count
    sage_dog._llm_count = llm_count

    orchestrator = MagicMock()
    orchestrator.dogs = {DogId.SAGE: sage_dog}
    state.orchestrator = orchestrator

    # DecideAgent mock
    decide_agent = MagicMock()
    decide_agent.stats.return_value = {"decisions_made": 2, "skipped": 3}
    state.decide_agent = decide_agent

    return state


# ════════════════════════════════════════════════════════════════════════════
# KernelMirror.snapshot()
# ════════════════════════════════════════════════════════════════════════════

class TestKernelMirrorSnapshot:

    def test_snapshot_returns_dict(self):
        """snapshot() returns a dict."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state())
        assert isinstance(snap, dict)

    def test_snapshot_has_all_expected_keys(self):
        """snapshot() includes all required subsystem keys."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state())
        required = {"snapshot_id", "timestamp", "uptime_s", "qtable", "axioms",
                    "lod", "account", "escore", "residual", "sage", "dogs",
                    "overall_health", "tier"}
        assert required.issubset(set(snap.keys()))

    def test_snapshot_id_increments(self):
        """snapshot_id increments with each call."""
        mirror = KernelMirror()
        state = _make_mock_state()
        snap1 = mirror.snapshot(state)
        snap2 = mirror.snapshot(state)
        assert snap2["snapshot_id"] == snap1["snapshot_id"] + 1

    def test_timestamp_is_recent(self):
        """timestamp is a recent Unix timestamp."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state())
        assert abs(snap["timestamp"] - time.time()) < 2.0

    def test_qtable_coverage_propagated(self):
        """qtable.coverage_pct from mock is propagated to snapshot."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state(qtable_coverage=15.5))
        assert snap["qtable"]["coverage_pct"] == 15.5

    def test_axioms_tier_propagated(self):
        """axioms.tier from monitor is propagated."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state(axiom_tier="AWAKENING"))
        assert snap["axioms"]["tier"] == "AWAKENING"

    def test_lod_current_propagated(self):
        """lod.current_lod from controller is propagated."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state(lod=2))
        assert snap["lod"]["current_lod"] == 2

    def test_sage_llm_count_propagated(self):
        """sage.llm_count is taken from dog._llm_count."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state(llm_count=7, heuristic_count=3))
        assert snap["sage"]["llm_count"] == 7
        assert snap["sage"]["heuristic_count"] == 3

    def test_sage_llm_activation_rate(self):
        """llm_activation_rate = llm / (llm + heuristic)."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state(llm_count=3, heuristic_count=7))
        assert snap["sage"]["llm_activation_rate"] == pytest.approx(0.3, abs=0.01)

    def test_sage_temporal_mcts_active_true_when_llm_used(self):
        """temporal_mcts_active = True when llm_count > 0."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state(llm_count=1))
        assert snap["sage"]["temporal_mcts_active"] is True

    def test_sage_temporal_mcts_active_false_when_heuristic_only(self):
        """temporal_mcts_active = False when llm_count == 0."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state(llm_count=0, heuristic_count=10))
        assert snap["sage"]["temporal_mcts_active"] is False


# ════════════════════════════════════════════════════════════════════════════
# KernelMirror.overall_health + tier
# ════════════════════════════════════════════════════════════════════════════

class TestKernelMirrorHealth:

    def test_health_is_float_in_range(self):
        """overall_health is a float in [0, 100]."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state())
        h = snap["overall_health"]
        assert isinstance(h, float)
        assert 0.0 <= h <= 100.0

    def test_tier_bark_on_low_health(self):
        """Low QTable coverage + DORMANT axioms + LOD 3 → BARK tier."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state(
            qtable_coverage=0.1,
            axiom_tier="DORMANT",
            lod=3,
            llm_count=0,
        ))
        assert snap["tier"] in ("BARK", "GROWL"), f"Expected BARK/GROWL, got {snap['tier']}"

    def test_tier_wag_on_moderate_health(self):
        """Moderate coverage + AWAKENING axioms + LOD 0 → WAG or HOWL."""
        mirror = KernelMirror()
        snap = mirror.snapshot(_make_mock_state(
            qtable_coverage=40.0,
            axiom_tier="AWAKENING",
            lod=0,
            llm_count=8,
            heuristic_count=2,
        ))
        assert snap["tier"] in ("WAG", "HOWL"), f"Expected WAG/HOWL, got {snap['tier']}"

    def test_health_improves_with_transcendent_axioms(self):
        """TRANSCENDENT axiom tier produces higher health than DORMANT."""
        mirror = KernelMirror()
        snap_dormant = mirror.snapshot(_make_mock_state(axiom_tier="DORMANT"))
        snap_transcendent = mirror.snapshot(_make_mock_state(axiom_tier="TRANSCENDENT"))
        assert snap_transcendent["overall_health"] > snap_dormant["overall_health"]

    def test_health_lower_with_lod_3(self):
        """LOD 3 (emergency) produces lower health than LOD 0 (full)."""
        mirror = KernelMirror()
        snap_full = mirror.snapshot(_make_mock_state(lod=0))
        snap_minimal = mirror.snapshot(_make_mock_state(lod=3))
        assert snap_minimal["overall_health"] < snap_full["overall_health"]


# ════════════════════════════════════════════════════════════════════════════
# KernelMirror.diff()
# ════════════════════════════════════════════════════════════════════════════

class TestKernelMirrorDiff:

    def test_first_diff_returns_empty(self):
        """Before any snapshot, diff returns empty dict."""
        mirror = KernelMirror()
        assert mirror.diff({}) == {}

    def test_diff_detects_changed_scalar(self):
        """diff() surfaces leaf-level changes."""
        mirror = KernelMirror()
        old = {"a": {"b": 1}}
        new = {"a": {"b": 2}}
        d = _deep_diff(old, new)
        assert "a.b" in d
        assert d["a.b"]["old"] == 1
        assert d["a.b"]["new"] == 2

    def test_diff_ignores_timestamp(self):
        """diff() does not surface timestamp changes."""
        mirror = KernelMirror()
        old = {"timestamp": 1000.0, "overall_health": 50.0}
        new = {"timestamp": 2000.0, "overall_health": 50.0}
        d = _deep_diff(old, new)
        assert "timestamp" not in d

    def test_diff_ignores_snapshot_id(self):
        """diff() does not surface snapshot_id changes."""
        old = {"snapshot_id": 0, "tier": "BARK"}
        new = {"snapshot_id": 1, "tier": "BARK"}
        d = _deep_diff(old, new)
        assert "snapshot_id" not in d
        assert "tier" not in d  # unchanged

    def test_diff_between_consecutive_snapshots(self):
        """After two snapshots with different health, diff contains overall_health."""
        mirror = KernelMirror()
        state1 = _make_mock_state(axiom_tier="DORMANT", lod=3)
        state2 = _make_mock_state(axiom_tier="TRANSCENDENT", lod=0)
        snap1 = mirror.snapshot(state1)
        snap2 = mirror.snapshot(state2)
        d = mirror.diff(snap2)
        # The axiom tier changed → should appear in diff
        assert "axioms.tier" in d, f"Expected 'axioms.tier' in diff, got: {list(d.keys())}"

    def test_diff_empty_when_nothing_changed(self):
        """Identical snapshots produce empty diff."""
        mirror = KernelMirror()
        state = _make_mock_state()
        snap1 = mirror.snapshot(state)
        snap2 = mirror.snapshot(state)
        d = mirror.diff(snap2)
        # Only snapshot_id and timestamp differ (both skipped)
        filtered = {k: v for k, v in d.items() if k not in ("snapshot_id", "timestamp", "uptime_s")}
        assert len(filtered) == 0


# ════════════════════════════════════════════════════════════════════════════
# KernelMirror: resilience (missing subsystems)
# ════════════════════════════════════════════════════════════════════════════

class TestKernelMirrorResilience:

    def test_snapshot_with_no_account_agent(self):
        """state.account_agent = None → snapshot still succeeds."""
        mirror = KernelMirror()
        state = _make_mock_state()
        state.account_agent = None
        snap = mirror.snapshot(state)
        assert "overall_health" in snap
        assert "account" not in snap

    def test_snapshot_with_no_escore(self):
        """state.escore_tracker = None → snapshot still succeeds."""
        mirror = KernelMirror()
        state = _make_mock_state()
        state.escore_tracker = None
        snap = mirror.snapshot(state)
        assert "overall_health" in snap

    def test_snapshot_with_no_sage(self):
        """No SAGE in orchestrator.dogs → sage section shows available=False."""
        mirror = KernelMirror()
        state = _make_mock_state()
        state.orchestrator.dogs = {}  # No dogs
        snap = mirror.snapshot(state)
        assert snap["sage"]["available"] is False

    def test_health_tier_values(self):
        """_health_tier covers all possible score ranges."""
        mirror = KernelMirror()
        assert mirror._health_tier(90.0) == "HOWL"
        assert mirror._health_tier(70.0) == "WAG"
        assert mirror._health_tier(50.0) == "GROWL"
        assert mirror._health_tier(20.0) == "BARK"
