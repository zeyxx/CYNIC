"""
CYNIC DiskClearance Tests — T01: DISK_CLEARED / MEMORY_CLEARED → LOD recovery

6 tests covering the LOD recovery path when disk/memory pressure clears.

Bug addressed: DiskWatcher/MemoryWatcher returned None (no event) when pressure
went back to OK, leaving _health_cache["disk_pct"] / ["memory_pct"] at their
last elevated values → LOD stayed degraded even when resources were available.

Fix: DISK_CLEARED / MEMORY_CLEARED events emitted; state.py resets cache field.
These tests verify the cache reset pattern and LOD recovery logic directly.
"""
from __future__ import annotations

import pytest
from cynic.judge.lod import (
    SurvivalLOD,
    LODController,
    _DISK_LOD1, _DISK_LOD2,
    _MEM_LOD1,
)
from cynic.core.event_bus import CoreEvent


# ── helpers ───────────────────────────────────────────────────────────────────

def _cache(disk_pct: float = 0.0, memory_pct: float = 0.0) -> dict:
    return {
        "error_rate": 0.0, "latency_ms": 0.0, "queue_depth": 0,
        "memory_pct": memory_pct, "disk_pct": disk_pct,
    }


# ── Tests ─────────────────────────────────────────────────────────────────────

class TestDiskClearance:
    """
    Verifies DISK_CLEARED / MEMORY_CLEARED close the LOD recovery loop.
    """

    def test_disk_elevated_prevents_full_lod(self):
        """disk_pct above LOD 1 threshold → LOD is REDUCED."""
        lod = LODController()
        result = lod.assess(**_cache(disk_pct=0.70))
        assert result == SurvivalLOD.REDUCED

    def test_disk_cleared_resets_to_full_after_hysteresis(self):
        """After DISK_CLEARED resets disk_pct, LOD recovers to FULL in HYSTERESIS_N steps."""
        lod = LODController()
        lod.assess(**_cache(disk_pct=0.70))
        # Simulate _on_disk_cleared: disk_pct resets to actual safe value
        safe = _cache(disk_pct=0.30)
        for _ in range(LODController.HYSTERESIS_N):
            result = lod.assess(**safe)
        assert result == SurvivalLOD.FULL

    def test_disk_cleared_single_assess_not_enough_due_to_hysteresis(self):
        """A single assessment after disk clear is not enough — hysteresis requires N=3."""
        lod = LODController()
        lod.assess(**_cache(disk_pct=0.70))
        result = lod.assess(**_cache(disk_pct=0.30))
        assert result == SurvivalLOD.REDUCED  # one assessment insufficient

    def test_memory_elevated_prevents_full_lod(self):
        """memory_pct above LOD 1 threshold → LOD is REDUCED."""
        lod = LODController()
        result = lod.assess(**_cache(memory_pct=0.70))
        assert result == SurvivalLOD.REDUCED

    def test_memory_cleared_resets_to_full_after_hysteresis(self):
        """After MEMORY_CLEARED resets memory_pct, LOD recovers to FULL."""
        lod = LODController()
        lod.assess(**_cache(memory_pct=0.70))
        safe = _cache(memory_pct=0.25)
        for _ in range(LODController.HYSTERESIS_N):
            result = lod.assess(**safe)
        assert result == SurvivalLOD.FULL

    def test_independent_clearance_while_other_elevated_stays_degraded(self):
        """Clearing disk while memory still elevated keeps LOD degraded."""
        lod = LODController()
        lod.assess(**_cache(disk_pct=0.70, memory_pct=0.70))
        # Only disk clears — memory still elevated
        for _ in range(LODController.HYSTERESIS_N + 1):
            result = lod.assess(**_cache(disk_pct=0.25, memory_pct=0.70))
        assert result == SurvivalLOD.REDUCED


class TestDiskClearedEvent:
    """Verify DISK_CLEARED and MEMORY_CLEARED are defined in CoreEvent."""

    def test_disk_cleared_event_exists(self):
        assert hasattr(CoreEvent, "DISK_CLEARED")
        assert CoreEvent.DISK_CLEARED == "storage.disk_cleared"

    def test_memory_cleared_event_exists(self):
        assert hasattr(CoreEvent, "MEMORY_CLEARED")
        assert CoreEvent.MEMORY_CLEARED == "system.memory_cleared"
