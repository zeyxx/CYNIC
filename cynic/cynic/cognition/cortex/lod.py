"""
CYNIC SurvivalLOD — Tiered Graceful Degradation (δ2)

Level of Detail (LOD) defines CYNIC's operating mode under resource constraints.
When the system is healthy → full operation. Under stress → progressive degradation.

4 LOD Levels (φ-derived thresholds):
    LOD 0 — FULL       Quality threshold ≥ WAG_MIN (61.8%)
    LOD 1 — REDUCED    Quality threshold ≥ GROWL_MIN (38.2%)
    LOD 2 — EMERGENCY  Quality threshold ≥ PHI_INV_3 (23.6%)
    LOD 3 — MINIMAL    Quality below PHI_INV_3 — survival mode

What changes at each LOD:
    LOD 0 FULL:      All 11 Dogs, full LLM calls, all consciousness levels
    LOD 1 REDUCED:   Skip slowest Dogs (SAGE, CARTOGRAPHER), L2 MICRO max
    LOD 2 EMERGENCY: REFLEX only (GUARDIAN + ANALYST + JANITOR), no LLM
    LOD 3 MINIMAL:   GUARDIAN only, heuristic scoring, health reports only

Trigger metrics:
    error_rate     → recent error rate (0.0-1.0)
    latency_ms     → recent 95th-percentile response time (ms)
    queue_depth    → current scheduler queue depth (cells waiting)
    memory_pct     → heap memory usage (0.0-1.0)

Thresholds (φ-symmetric):
    LOD 1: error_rate ≥ PHI_INV_2 (0.382) OR latency_ms ≥ 1000 OR queue_depth ≥ 34 OR disk_pct ≥ 0.618 OR memory_pct ≥ 0.618
    LOD 2: error_rate ≥ PHI_INV   (0.618) OR latency_ms ≥ 2850 OR queue_depth ≥ 89  OR disk_pct ≥ 0.764 OR memory_pct ≥ 0.764
    LOD 3: error_rate ≥ 1.0       OR latency_ms ≥ 5000 OR queue_depth ≥ 144         OR disk_pct ≥ 0.90  OR memory_pct ≥ 0.90

Usage:
    lod = LODController()
    level = lod.assess(error_rate=0.05, latency_ms=300, queue_depth=5)
    # → SurvivalLOD.FULL

    lod.force(SurvivalLOD.EMERGENCY)  # Manual override
    lod.clear_force()                  # Remove override
"""
from __future__ import annotations

import time
import logging
from dataclasses import dataclass, field
from enum import Enum
from typing import Any


from cynic.core.phi import (
    PHI_INV, PHI_INV_2, PHI_INV_3,
    WAG_MIN, GROWL_MIN, fibonacci,
)

logger = logging.getLogger("cynic.cognition.cortex.lod")

# Queue depth thresholds (Fibonacci: 34, 89, 144)
_QUEUE_LOD1 = fibonacci(9)   # 34
_QUEUE_LOD2 = fibonacci(11)  # 89
_QUEUE_LOD3 = fibonacci(12)  # 144

# Latency thresholds (ms)
_LATENCY_LOD1 = 1_000.0    # 1 second
_LATENCY_LOD2 = 2_850.0    # F(8)×21ms × 100 ≈ 2850ms (L1 MACRO target)
_LATENCY_LOD3 = 5_000.0    # 5 seconds — critical

# Error rate thresholds (φ-symmetric)
_ERR_LOD1 = PHI_INV_2  # 0.382
_ERR_LOD2 = PHI_INV    # 0.618
_ERR_LOD3 = 1.0        # Full failure


# ── SurvivalLOD ───────────────────────────────────────────────────────────

class SurvivalLOD(int, Enum):
    """
    Level of Detail for graceful degradation.
    Lower value = higher quality (LOD 0 = full, LOD 3 = minimal).
    """
    FULL      = 0  # All Dogs + LLM + all consciousness levels
    REDUCED   = 1  # Skip slow Dogs, L2 MICRO max, faster heuristics
    EMERGENCY = 2  # REFLEX only (GUARDIAN + ANALYST + JANITOR), no LLM
    MINIMAL   = 3  # GUARDIAN only, health reports only

    @property
    def description(self) -> str:
        return {
            SurvivalLOD.FULL:      "LOD 0 — Full operation: all Dogs, LLM, all consciousness levels",
            SurvivalLOD.REDUCED:   "LOD 1 — Reduced: skip SAGE/CARTOGRAPHER, L2 MICRO max",
            SurvivalLOD.EMERGENCY: "LOD 2 — Emergency: REFLEX only, no LLM",
            SurvivalLOD.MINIMAL:   "LOD 3 — Minimal: GUARDIAN only, survival mode",
        }[self]

    @property
    def allows_llm(self) -> bool:
        return self <= SurvivalLOD.REDUCED

    @property
    def allows_slow_dogs(self) -> bool:
        return self == SurvivalLOD.FULL

    @property
    def max_consciousness(self) -> str:
        """Maximum consciousness level allowed at this LOD."""
        return {
            SurvivalLOD.FULL:      "META",
            SurvivalLOD.REDUCED:   "MICRO",
            SurvivalLOD.EMERGENCY: "REFLEX",
            SurvivalLOD.MINIMAL:   "REFLEX",
        }[self]


# ── HealthMetrics ─────────────────────────────────────────────────────────

# Disk usage thresholds (φ-derived, fraction of disk used)
_DISK_LOD1 = PHI_INV        # 0.618 — 61.8% full → REDUCED
_DISK_LOD2 = 1 - PHI_INV_3  # 0.764 — 76.4% full → EMERGENCY
_DISK_LOD3 = 0.90            # 90%   full → MINIMAL

# Memory usage thresholds (same φ-scale as disk — fraction of RAM used)
_MEM_LOD1 = PHI_INV        # 0.618 — 61.8% used → REDUCED
_MEM_LOD2 = 1 - PHI_INV_3  # 0.764 — 76.4% used → EMERGENCY
_MEM_LOD3 = 0.90            # 90%   used → MINIMAL


@dataclass
class HealthMetrics:
    """System health snapshot for LOD assessment."""
    error_rate: float = 0.0       # Recent error rate [0, 1]
    latency_ms: float = 0.0       # Recent p95 latency in ms
    queue_depth: int = 0          # Current queue depth (cells waiting)
    memory_pct: float = 0.0       # Heap memory [0, 1]
    disk_pct: float = 0.0         # Disk usage fraction [0, 1]
    timestamp: float = field(default_factory=time.time)

    def worst_lod(self) -> SurvivalLOD:
        """Compute the required LOD from this health snapshot."""
        # Check LOD 3 first (most severe)
        if (self.error_rate >= _ERR_LOD3
                or self.latency_ms >= _LATENCY_LOD3
                or self.queue_depth >= _QUEUE_LOD3
                or self.disk_pct >= _DISK_LOD3
                or self.memory_pct >= _MEM_LOD3):
            return SurvivalLOD.MINIMAL

        if (self.error_rate >= _ERR_LOD2
                or self.latency_ms >= _LATENCY_LOD2
                or self.queue_depth >= _QUEUE_LOD2
                or self.disk_pct >= _DISK_LOD2
                or self.memory_pct >= _MEM_LOD2):
            return SurvivalLOD.EMERGENCY

        if (self.error_rate >= _ERR_LOD1
                or self.latency_ms >= _LATENCY_LOD1
                or self.queue_depth >= _QUEUE_LOD1
                or self.disk_pct >= _DISK_LOD1
                or self.memory_pct >= _MEM_LOD1):
            return SurvivalLOD.REDUCED

        return SurvivalLOD.FULL


# ── LODController ─────────────────────────────────────────────────────────

class LODController:
    """
    Manages the active LOD level based on system health.

    Maintains a rolling history of assessments. LOD is determined by the
    worst health metric. LOD degradation is immediate; recovery requires
    HYSTERESIS_N consecutive healthy assessments (prevents flapping).
    """

    # Consecutive healthy assessments needed before LOD improves
    HYSTERESIS_N: int = fibonacci(4)  # 3

    def __init__(self) -> None:
        self._current: SurvivalLOD = SurvivalLOD.FULL
        self._forced: Optional[SurvivalLOD] = None
        self._history: list[HealthMetrics] = []
        self._healthy_streak: int = 0
        self._transitions: list[dict[str, Any]] = []
        self._started_at: float = time.time()

    # ── Assessment ────────────────────────────────────────────────────────

    def assess(
        self,
        error_rate: float = 0.0,
        latency_ms: float = 0.0,
        queue_depth: int = 0,
        memory_pct: float = 0.0,
        disk_pct: float = 0.0,
    ) -> SurvivalLOD:
        """
        Assess current system health and return appropriate LOD.

        If a forced LOD is set, returns forced LOD without computation.

        Args:
            error_rate:  Recent error rate [0, 1]
            latency_ms:  Recent p95 latency in ms
            queue_depth: Current scheduler queue depth
            memory_pct:  Heap memory fraction [0, 1]
            disk_pct:    Disk usage fraction [0, 1] (0=empty, 1=full)

        Returns:
            Recommended SurvivalLOD (also updates self._current).
        """
        if self._forced is not None:
            return self._forced

        metrics = HealthMetrics(
            error_rate=max(0.0, min(1.0, error_rate)),
            latency_ms=max(0.0, latency_ms),
            queue_depth=max(0, queue_depth),
            memory_pct=max(0.0, min(1.0, memory_pct)),
            disk_pct=max(0.0, min(1.0, disk_pct)),
        )
        self._history.append(metrics)
        if len(self._history) > 100:
            self._history.pop(0)

        target_lod = metrics.worst_lod()

        if target_lod > self._current:
            # Degrade immediately — no hysteresis
            self._transition(target_lod, metrics)
            self._healthy_streak = 0
        elif target_lod < self._current:
            # Recovery: require HYSTERESIS_N consecutive assessments at target
            self._healthy_streak += 1
            if self._healthy_streak >= self.HYSTERESIS_N:
                self._transition(target_lod, metrics)
                self._healthy_streak = 0
        else:
            self._healthy_streak = 0  # No change

        return self._current

    # ── Control ───────────────────────────────────────────────────────────

    def force(self, lod: SurvivalLOD) -> None:
        """
        Force a specific LOD level, bypassing health assessment.
        Use for maintenance, testing, or emergency override.
        """
        logger.warning("LODController: forcing LOD %s", lod.name)
        self._forced = lod

    def clear_force(self) -> None:
        """Remove forced LOD — resume health-based assessment."""
        self._forced = None

    # ── Query ─────────────────────────────────────────────────────────────

    @property
    def current(self) -> SurvivalLOD:
        """Current active LOD level."""
        return self._forced if self._forced is not None else self._current

    def is_healthy(self) -> bool:
        """True if currently at LOD 0 FULL."""
        return self.current == SurvivalLOD.FULL

    def status(self) -> dict[str, Any]:
        """Full LOD controller status dict."""
        lod = self.current
        return {
            "current_lod": lod.value,
            "current_name": lod.name,
            "description": lod.description,
            "allows_llm": lod.allows_llm,
            "max_consciousness": lod.max_consciousness,
            "forced": self._forced is not None,
            "healthy_streak": self._healthy_streak,
            "hysteresis_n": self.HYSTERESIS_N,
            "total_assessments": len(self._history),
            "total_transitions": len(self._transitions),
            "uptime_s": round(time.time() - self._started_at, 1),
            "recent_transitions": self._transitions[-5:],
        }

    # ── Private ───────────────────────────────────────────────────────────

    def _transition(self, new_lod: SurvivalLOD, metrics: HealthMetrics) -> None:
        if new_lod == self._current:
            return
        old = self._current
        self._current = new_lod
        transition = {
            "from": old.name,
            "to": new_lod.name,
            "timestamp": metrics.timestamp,
            "error_rate": metrics.error_rate,
            "latency_ms": metrics.latency_ms,
            "queue_depth": metrics.queue_depth,
        }
        self._transitions.append(transition)
        if len(self._transitions) > 100:
            self._transitions.pop(0)

        if new_lod > old:
            logger.warning(
                "LOD DEGRADATION: %s → %s (err=%.2f lat=%.0fms q=%d)",
                old.name, new_lod.name,
                metrics.error_rate, metrics.latency_ms, metrics.queue_depth,
            )
        else:
            logger.info(
                "LOD RECOVERY: %s → %s",
                old.name, new_lod.name,
            )
