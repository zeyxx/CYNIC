"""
CYNIC SurvivalLOD Tests (δ2)

Tests LOD graceful degradation controller.
No LLM, no DB — pure in-memory.
"""
from __future__ import annotations

import pytest
from cynic.judge.lod import (
    SurvivalLOD,
    HealthMetrics,
    LODController,
    _QUEUE_LOD1, _QUEUE_LOD2, _QUEUE_LOD3,
    _LATENCY_LOD1, _LATENCY_LOD2, _LATENCY_LOD3,
    _ERR_LOD1, _ERR_LOD2, _ERR_LOD3,
    _DISK_LOD1, _DISK_LOD2, _DISK_LOD3,
    _MEM_LOD1, _MEM_LOD2, _MEM_LOD3,
)


# ── SurvivalLOD enum ──────────────────────────────────────────────────────────

class TestSurvivalLOD:
    def test_full_is_zero(self):
        assert SurvivalLOD.FULL == 0

    def test_minimal_is_three(self):
        assert SurvivalLOD.MINIMAL == 3

    def test_ordering(self):
        assert SurvivalLOD.FULL < SurvivalLOD.REDUCED < SurvivalLOD.EMERGENCY < SurvivalLOD.MINIMAL

    def test_allows_llm_full(self):
        assert SurvivalLOD.FULL.allows_llm is True

    def test_allows_llm_reduced(self):
        assert SurvivalLOD.REDUCED.allows_llm is True

    def test_no_llm_emergency(self):
        assert SurvivalLOD.EMERGENCY.allows_llm is False

    def test_no_llm_minimal(self):
        assert SurvivalLOD.MINIMAL.allows_llm is False

    def test_allows_slow_dogs_only_full(self):
        assert SurvivalLOD.FULL.allows_slow_dogs is True
        assert SurvivalLOD.REDUCED.allows_slow_dogs is False
        assert SurvivalLOD.EMERGENCY.allows_slow_dogs is False

    def test_max_consciousness_full(self):
        assert SurvivalLOD.FULL.max_consciousness == "META"

    def test_max_consciousness_reduced(self):
        assert SurvivalLOD.REDUCED.max_consciousness == "MICRO"

    def test_max_consciousness_emergency(self):
        assert SurvivalLOD.EMERGENCY.max_consciousness == "REFLEX"

    def test_max_consciousness_minimal(self):
        assert SurvivalLOD.MINIMAL.max_consciousness == "REFLEX"

    def test_description_not_empty(self):
        for lod in SurvivalLOD:
            assert len(lod.description) > 10


# ── HealthMetrics ─────────────────────────────────────────────────────────────

class TestHealthMetrics:
    def test_healthy_metrics_full_lod(self):
        m = HealthMetrics(error_rate=0.0, latency_ms=100, queue_depth=5)
        assert m.worst_lod() == SurvivalLOD.FULL

    def test_error_rate_lod1_threshold(self):
        m = HealthMetrics(error_rate=_ERR_LOD1)
        assert m.worst_lod() == SurvivalLOD.REDUCED

    def test_error_rate_lod2_threshold(self):
        m = HealthMetrics(error_rate=_ERR_LOD2)
        assert m.worst_lod() == SurvivalLOD.EMERGENCY

    def test_error_rate_lod3_threshold(self):
        m = HealthMetrics(error_rate=_ERR_LOD3)
        assert m.worst_lod() == SurvivalLOD.MINIMAL

    def test_latency_lod1_threshold(self):
        m = HealthMetrics(latency_ms=_LATENCY_LOD1)
        assert m.worst_lod() == SurvivalLOD.REDUCED

    def test_latency_lod2_threshold(self):
        m = HealthMetrics(latency_ms=_LATENCY_LOD2)
        assert m.worst_lod() == SurvivalLOD.EMERGENCY

    def test_latency_lod3_threshold(self):
        m = HealthMetrics(latency_ms=_LATENCY_LOD3)
        assert m.worst_lod() == SurvivalLOD.MINIMAL

    def test_queue_lod1_threshold(self):
        m = HealthMetrics(queue_depth=_QUEUE_LOD1)
        assert m.worst_lod() == SurvivalLOD.REDUCED

    def test_queue_lod2_threshold(self):
        m = HealthMetrics(queue_depth=_QUEUE_LOD2)
        assert m.worst_lod() == SurvivalLOD.EMERGENCY

    def test_queue_lod3_threshold(self):
        m = HealthMetrics(queue_depth=_QUEUE_LOD3)
        assert m.worst_lod() == SurvivalLOD.MINIMAL

    def test_worst_metric_dominates(self):
        """Queue at LOD3 dominates even when error_rate is healthy."""
        m = HealthMetrics(error_rate=0.0, latency_ms=100, queue_depth=_QUEUE_LOD3)
        assert m.worst_lod() == SurvivalLOD.MINIMAL

    def test_below_all_thresholds_is_full(self):
        m = HealthMetrics(error_rate=0.1, latency_ms=500, queue_depth=10)
        assert m.worst_lod() == SurvivalLOD.FULL

    # ── Disk pressure ──────────────────────────────────────────────────────

    def test_disk_lod1_threshold(self):
        m = HealthMetrics(disk_pct=_DISK_LOD1)
        assert m.worst_lod() == SurvivalLOD.REDUCED

    def test_disk_lod2_threshold(self):
        m = HealthMetrics(disk_pct=_DISK_LOD2)
        assert m.worst_lod() == SurvivalLOD.EMERGENCY

    def test_disk_lod3_threshold(self):
        m = HealthMetrics(disk_pct=_DISK_LOD3)
        assert m.worst_lod() == SurvivalLOD.MINIMAL

    def test_disk_below_threshold_is_full(self):
        m = HealthMetrics(disk_pct=0.5)
        assert m.worst_lod() == SurvivalLOD.FULL

    # ── Memory pressure ─────────────────────────────────────────────────────

    def test_memory_lod1_threshold(self):
        m = HealthMetrics(memory_pct=_MEM_LOD1)
        assert m.worst_lod() == SurvivalLOD.REDUCED

    def test_memory_lod2_threshold(self):
        m = HealthMetrics(memory_pct=_MEM_LOD2)
        assert m.worst_lod() == SurvivalLOD.EMERGENCY

    def test_memory_lod3_threshold(self):
        m = HealthMetrics(memory_pct=_MEM_LOD3)
        assert m.worst_lod() == SurvivalLOD.MINIMAL

    def test_memory_below_threshold_is_full(self):
        m = HealthMetrics(memory_pct=0.5)
        assert m.worst_lod() == SurvivalLOD.FULL

    def test_memory_dominates_when_worst(self):
        """Memory at LOD3 dominates even when all other metrics are healthy."""
        m = HealthMetrics(error_rate=0.0, latency_ms=100, queue_depth=5,
                          disk_pct=0.5, memory_pct=_MEM_LOD3)
        assert m.worst_lod() == SurvivalLOD.MINIMAL

    def test_disk_and_memory_independent(self):
        """LOD1 disk does not mask LOD2 memory."""
        m = HealthMetrics(disk_pct=_DISK_LOD1, memory_pct=_MEM_LOD2)
        assert m.worst_lod() == SurvivalLOD.EMERGENCY


# ── LODController ─────────────────────────────────────────────────────────────

class TestLODControllerDefaults:
    def test_initial_lod_full(self):
        ctrl = LODController()
        assert ctrl.current == SurvivalLOD.FULL

    def test_initial_is_healthy(self):
        ctrl = LODController()
        assert ctrl.is_healthy() is True

    def test_status_keys(self):
        ctrl = LODController()
        s = ctrl.status()
        assert "current_lod" in s
        assert "current_name" in s
        assert "allows_llm" in s
        assert "max_consciousness" in s
        assert "forced" in s
        assert "healthy_streak" in s
        assert "hysteresis_n" in s
        assert "total_assessments" in s
        assert "total_transitions" in s
        assert "uptime_s" in s

    def test_status_forced_false_initially(self):
        ctrl = LODController()
        assert ctrl.status()["forced"] is False

    def test_hysteresis_n_is_three(self):
        """F(4) = 3 for hysteresis."""
        ctrl = LODController()
        assert ctrl.HYSTERESIS_N == 3


class TestLODControllerAssess:
    def test_healthy_assessment_stays_full(self):
        ctrl = LODController()
        result = ctrl.assess(error_rate=0.0, latency_ms=100, queue_depth=5)
        assert result == SurvivalLOD.FULL

    def test_immediate_degradation_on_high_error(self):
        ctrl = LODController()
        result = ctrl.assess(error_rate=_ERR_LOD1)
        assert result == SurvivalLOD.REDUCED

    def test_immediate_degradation_to_minimal(self):
        ctrl = LODController()
        result = ctrl.assess(error_rate=1.0)
        assert result == SurvivalLOD.MINIMAL

    def test_degradation_immediate_no_hysteresis(self):
        """One bad assessment → immediate LOD change (no delay)."""
        ctrl = LODController()
        ctrl.assess(error_rate=_ERR_LOD2)  # LOD 2 = EMERGENCY
        assert ctrl.current == SurvivalLOD.EMERGENCY

    def test_recovery_requires_hysteresis(self):
        """After degradation, 1 healthy assessment is NOT enough to recover."""
        ctrl = LODController()
        ctrl.assess(error_rate=_ERR_LOD1)  # Degrade to LOD 1
        assert ctrl.current == SurvivalLOD.REDUCED
        ctrl.assess()  # 1 healthy — not enough
        assert ctrl.current == SurvivalLOD.REDUCED

    def test_recovery_after_hysteresis_n(self):
        """After HYSTERESIS_N healthy assessments, LOD improves."""
        ctrl = LODController()
        ctrl.assess(error_rate=_ERR_LOD1)
        assert ctrl.current == SurvivalLOD.REDUCED
        for _ in range(ctrl.HYSTERESIS_N):
            ctrl.assess()  # All healthy
        assert ctrl.current == SurvivalLOD.FULL

    def test_streak_resets_on_no_change(self):
        """If health is stable (same LOD), healthy_streak resets."""
        ctrl = LODController()
        ctrl.assess()
        ctrl.assess()
        assert ctrl.status()["healthy_streak"] == 0

    def test_assessment_count_accumulates(self):
        ctrl = LODController()
        for _ in range(5):
            ctrl.assess()
        assert ctrl.status()["total_assessments"] == 5

    def test_error_rate_clamped_below_zero(self):
        """Negative error_rate clamped to 0."""
        ctrl = LODController()
        result = ctrl.assess(error_rate=-1.0)
        assert result == SurvivalLOD.FULL

    def test_error_rate_clamped_above_one(self):
        """error_rate > 1 clamped to 1 → LOD 3."""
        ctrl = LODController()
        result = ctrl.assess(error_rate=99.0)
        assert result == SurvivalLOD.MINIMAL


class TestLODControllerForce:
    def test_force_overrides_assessment(self):
        ctrl = LODController()
        ctrl.force(SurvivalLOD.EMERGENCY)
        result = ctrl.assess(error_rate=0.0)  # Would normally be FULL
        assert result == SurvivalLOD.EMERGENCY

    def test_force_changes_current(self):
        ctrl = LODController()
        ctrl.force(SurvivalLOD.MINIMAL)
        assert ctrl.current == SurvivalLOD.MINIMAL

    def test_forced_status_true(self):
        ctrl = LODController()
        ctrl.force(SurvivalLOD.REDUCED)
        assert ctrl.status()["forced"] is True

    def test_clear_force_resumes_assessment(self):
        ctrl = LODController()
        ctrl.force(SurvivalLOD.EMERGENCY)
        ctrl.clear_force()
        result = ctrl.assess(error_rate=0.0)
        assert result == SurvivalLOD.FULL

    def test_forced_status_false_after_clear(self):
        ctrl = LODController()
        ctrl.force(SurvivalLOD.EMERGENCY)
        ctrl.clear_force()
        assert ctrl.status()["forced"] is False

    def test_force_to_full_while_degraded(self):
        """Can force FULL even while health would suggest degradation."""
        ctrl = LODController()
        ctrl.force(SurvivalLOD.FULL)
        result = ctrl.assess(error_rate=1.0)  # Would be MINIMAL, but forced
        assert result == SurvivalLOD.FULL


class TestLODTransitions:
    def test_transition_recorded(self):
        ctrl = LODController()
        ctrl.assess(error_rate=_ERR_LOD1)
        assert ctrl.status()["total_transitions"] == 1

    def test_no_transition_if_same_lod(self):
        ctrl = LODController()
        ctrl.assess()
        ctrl.assess()
        assert ctrl.status()["total_transitions"] == 0

    def test_recent_transitions_in_status(self):
        ctrl = LODController()
        ctrl.assess(error_rate=_ERR_LOD1)
        transitions = ctrl.status()["recent_transitions"]
        assert len(transitions) == 1
        assert transitions[0]["from"] == "FULL"
        assert transitions[0]["to"] == "REDUCED"


class TestHealthCacheRegressions:
    """
    Validate the _health_cache pattern that prevents the accumulated-metrics reset bug.

    Bug: if each sensor calls assess() with only its own dimension, a subsequent
    assess(memory_pct=0.5) resets disk_pct=0.0 → LOD incorrectly recovers.
    Fix: pass ALL cached dimensions on every assess() call.
    """

    def test_disk_not_reset_by_memory_assess(self):
        """
        Simulate _health_cache: disk=0.94 then memory=0.5.
        LOD must stay MINIMAL (disk 90%+) not recover to FULL.
        """
        ctrl = LODController()
        # Step 1 — disk watcher fires: disk critical
        cache = {"error_rate": 0.0, "latency_ms": 0.0, "queue_depth": 0,
                 "memory_pct": 0.0, "disk_pct": _DISK_LOD3}
        ctrl.assess(**cache)
        assert ctrl.current == SurvivalLOD.MINIMAL

        # Step 2 — memory watcher fires: memory healthy (simulates cache update)
        cache["memory_pct"] = 0.5
        ctrl.assess(**cache)  # disk_pct still 0.94 in cache
        assert ctrl.current == SurvivalLOD.MINIMAL  # Must NOT recover

    def test_memory_not_reset_by_disk_assess(self):
        """Memory CRITICAL must not be overridden by a healthy disk assess."""
        ctrl = LODController()
        cache = {"error_rate": 0.0, "latency_ms": 0.0, "queue_depth": 0,
                 "memory_pct": _MEM_LOD3, "disk_pct": 0.0}
        ctrl.assess(**cache)
        assert ctrl.current == SurvivalLOD.MINIMAL

        cache["disk_pct"] = 0.5
        ctrl.assess(**cache)  # memory_pct still critical in cache
        assert ctrl.current == SurvivalLOD.MINIMAL

    def test_combined_sensors_accumulate_correctly(self):
        """
        Disk at LOD1, memory at LOD2 → combined worst = EMERGENCY.
        Subsequent healthy judgment assess() must not reset disk/memory to 0.
        """
        ctrl = LODController()
        cache = {"error_rate": 0.0, "latency_ms": 0.0, "queue_depth": 0,
                 "memory_pct": _MEM_LOD2, "disk_pct": _DISK_LOD1}
        ctrl.assess(**cache)
        assert ctrl.current == SurvivalLOD.EMERGENCY

        # Judgment fires with low error rate — cache preserves disk+memory
        cache["error_rate"] = 0.01
        cache["latency_ms"] = 200.0
        ctrl.assess(**cache)
        assert ctrl.current == SurvivalLOD.EMERGENCY  # Still EMERGENCY, not recovered
