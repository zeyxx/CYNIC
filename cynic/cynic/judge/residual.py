"""
CYNIC ResidualDetector — Composant 7/9 du Kernel

"THE_UNNAMEABLE = ce qui existe avant d'être nommé"

Residual variance = écart inexpliqué entre les votes des Dogs.
Quand le résiduel > φ⁻² (38.2%), quelque chose n'est pas capturé
par les dimensions existantes — une nouvelle dimension émerge.

Patterns détectés:
  SPIKE:       Saut soudain (ce jugement >> moyenne glissante)
  RISING:      Tendance croissante (slope > threshold sur N jugements)
  STABLE_HIGH: Constamment au-dessus de PHI_INV_2 pour N jugements

Lifecycle:
  detector = ResidualDetector()
  detector.start(get_core_bus())  # Subscribe to JUDGMENT_CREATED
  # → observe() called automatically
  # → EMERGENCE_DETECTED emitted when pattern found

Intégration: orchestrator.py observe() post-judgment (synchronous)
Phase 2: PostgreSQL persistence (candidates survive restart)
"""
from __future__ import annotations

import asyncio
import logging
import time
from collections import deque
from dataclasses import dataclass, field
from typing import Any, Deque, Dict, List, Optional

from cynic.core.phi import (
    PHI_INV, PHI_INV_2, PHI_INV_3, MAX_Q_SCORE, fibonacci,
)
from cynic.core.judgment import Judgment
from cynic.core.event_bus import (
    get_core_bus, Event, CoreEvent,
)

logger = logging.getLogger("cynic.judge.residual")

# Rolling window: F(8) = 21 judgments
HISTORY_MAXLEN: int = fibonacci(8)   # 21

# Min samples before pattern detection kicks in
MIN_SAMPLES: int = fibonacci(4)       # 3

# SPIKE: residual > mean + N × std_dev
SPIKE_SIGMA: float = 2.0

# RISING: slope threshold per judgment
RISING_SLOPE_THRESHOLD: float = PHI_INV_3   # 0.236 per step = fast rise

# STABLE_HIGH: how many consecutive high residuals
STABLE_HIGH_N: int = fibonacci(5)     # 5 consecutive

# Minimum residual to call anything noteworthy (below this = healthy)
ANOMALY_THRESHOLD: float = PHI_INV_2  # 0.382


@dataclass
class ResidualPoint:
    """One recorded residual observation."""
    judgment_id: str
    residual: float          # Normalized [0, 1]
    reality: str
    analysis: str
    unnameable: bool         # Was THE_UNNAMEABLE flag set?
    timestamp: float = field(default_factory=time.time)


@dataclass
class ResidualPattern:
    """A detected residual pattern."""
    pattern_type: str        # SPIKE | RISING | STABLE_HIGH
    severity: float          # 0–1 (normalized strength)
    evidence: Dict[str, Any] = field(default_factory=dict)
    timestamp: float = field(default_factory=time.time)


class ResidualDetector:
    """
    Watches judgment residual_variance over time.

    Integrates with the core event bus — subscribes to JUDGMENT_CREATED,
    observes each judgment, emits EMERGENCE_DETECTED when patterns found.

    Pure in-memory (Phase 1). Phase 2: PostgreSQL warm-load for candidates.
    """

    def __init__(self) -> None:
        self._history: Deque[ResidualPoint] = deque(maxlen=HISTORY_MAXLEN)
        self._patterns: List[ResidualPattern] = []
        self._observations: int = 0
        self._anomalies: int = 0
        self._patterns_detected: int = 0
        self._listener_registered: bool = False
        self._consecutive_high: int = 0   # Running count for STABLE_HIGH
        self._db_pool: Optional[Any] = None

    def set_db_pool(self, pool: Any) -> None:
        """Wire a DB pool so history observations are persisted automatically."""
        self._db_pool = pool

    # ── Public API ────────────────────────────────────────────────────────

    def start(self, bus=None) -> None:
        """
        Subscribe to JUDGMENT_CREATED events on the core bus.

        Call this ONCE during kernel startup, before judgments flow.
        """
        if self._listener_registered:
            return

        target_bus = bus or get_core_bus()
        target_bus.on(CoreEvent.JUDGMENT_CREATED, self._handle_judgment_event)
        self._listener_registered = True
        logger.info("ResidualDetector subscribed to JUDGMENT_CREATED")

    def observe(self, judgment: Judgment) -> Optional[ResidualPattern]:
        """
        Synchronously observe a judgment.

        Returns a ResidualPattern if one is detected, else None.
        Can be called directly from orchestrator post-judgment.
        """
        self._observations += 1
        residual = judgment.residual_variance  # Already in [0, 1]

        point = ResidualPoint(
            judgment_id=judgment.judgment_id,
            residual=residual,
            reality=judgment.cell.reality,
            analysis=judgment.cell.analysis,
            unnameable=judgment.unnameable_detected,
        )
        self._history.append(point)
        self._maybe_persist(point)

        if residual >= ANOMALY_THRESHOLD:
            self._anomalies += 1
            self._consecutive_high += 1
        else:
            self._consecutive_high = 0

        # Pattern detection only after MIN_SAMPLES
        if len(self._history) < MIN_SAMPLES:
            return None

        return self._detect_pattern(point, residual)

    def stats(self) -> Dict[str, Any]:
        return {
            "observations": self._observations,
            "anomalies": self._anomalies,
            "patterns_detected": self._patterns_detected,
            "history_len": len(self._history),
            "consecutive_high": self._consecutive_high,
            "anomaly_rate": round(self._anomalies / max(self._observations, 1), 3),
            "recent_patterns": [
                {"type": p.pattern_type, "severity": round(p.severity, 3)}
                for p in self._patterns[-5:]
            ],
        }

    # ── DB Persistence (Phase 2) ──────────────────────────────────────────

    async def load_from_db(self, pool: Any) -> int:
        """
        Warm-start _history from DB on boot.

        Loads up to HISTORY_MAXLEN most recent observations (oldest-first).
        Rebuilds _consecutive_high from loaded history.
        Returns count of points loaded.
        """
        try:
            async with pool.acquire() as conn:
                rows = await conn.fetch("""
                    SELECT judgment_id, residual, reality, analysis, unnameable,
                           EXTRACT(EPOCH FROM observed_at) AS timestamp
                    FROM residual_history
                    ORDER BY observed_at DESC
                    LIMIT $1
                """, HISTORY_MAXLEN)
        except Exception as exc:
            logger.warning("ResidualDetector warm-start failed: %s", exc)
            return 0

        if not rows:
            return 0

        # Replay oldest-first so deque and counters are correct
        points = list(reversed(rows))
        for row in points:
            point = ResidualPoint(
                judgment_id=row["judgment_id"],
                residual=float(row["residual"]),
                reality=row["reality"],
                analysis=row["analysis"],
                unnameable=bool(row["unnameable"]),
                timestamp=float(row["timestamp"]),
            )
            self._history.append(point)
            self._observations += 1
            if point.residual >= ANOMALY_THRESHOLD:
                self._anomalies += 1
                self._consecutive_high += 1
            else:
                self._consecutive_high = 0

        logger.info(
            "ResidualDetector warm-start: %d points loaded (consecutive_high=%d)",
            len(points), self._consecutive_high,
        )
        return len(points)

    async def _save_point_to_db(self, point: ResidualPoint) -> None:
        """Fire-and-forget: persist one ResidualPoint to DB."""
        try:
            async with self._db_pool.acquire() as conn:
                await conn.execute("""
                    INSERT INTO residual_history
                        (judgment_id, residual, reality, analysis, unnameable)
                    VALUES ($1, $2, $3, $4, $5)
                """,
                    point.judgment_id,
                    point.residual,
                    point.reality,
                    point.analysis,
                    point.unnameable,
                )
        except Exception as exc:
            logger.warning("ResidualDetector persist failed: %s", exc)

    def _maybe_persist(self, point: ResidualPoint) -> None:
        """Schedule fire-and-forget DB save if pool is set."""
        if self._db_pool is None:
            return
        try:
            loop = asyncio.get_running_loop()
            loop.create_task(self._save_point_to_db(point))
        except RuntimeError:
            pass  # No running loop (sync test context)

    # ── Pattern Detection ─────────────────────────────────────────────────

    def _detect_pattern(
        self,
        point: ResidualPoint,
        residual: float,
    ) -> Optional[ResidualPattern]:
        """Check all pattern types. Return first detected."""
        history_vals = [p.residual for p in self._history]

        pattern = (
            self._check_spike(residual, history_vals) or
            self._check_stable_high() or
            self._check_rising(history_vals)
        )

        if pattern:
            self._patterns_detected += 1
            self._patterns.append(pattern)
            if len(self._patterns) > 100:
                self._patterns.pop(0)
            logger.info(
                "ResidualDetector: pattern=%s severity=%.3f at %s·%s",
                pattern.pattern_type, pattern.severity,
                point.reality, point.analysis,
            )

        return pattern

    def _check_spike(
        self,
        current: float,
        history: List[float],
    ) -> Optional[ResidualPattern]:
        """SPIKE: current residual is significantly above rolling baseline.

        Two modes:
        - When std is meaningful (>1%): use z-score test (SPIKE_SIGMA)
        - When baseline is stable (std≈0): use absolute jump > ANOMALY_THRESHOLD
        """
        if len(history) < MIN_SAMPLES:
            return None

        prev = history[:-1]  # exclude current
        mean = sum(prev) / len(prev)

        variance = sum((v - mean) ** 2 for v in prev) / len(prev)
        std = variance ** 0.5

        MIN_STD = 0.01  # 1% — below this, baseline is essentially constant

        if std >= MIN_STD:
            # Standard z-score path
            z_score = (current - mean) / std
            if z_score >= SPIKE_SIGMA and current >= ANOMALY_THRESHOLD:
                severity = min(z_score / (SPIKE_SIGMA * 3), 1.0)
                return ResidualPattern(
                    pattern_type="SPIKE",
                    severity=severity,
                    evidence={
                        "current": round(current, 3),
                        "mean": round(mean, 3),
                        "std": round(std, 3),
                        "z_score": round(z_score, 2),
                    },
                )
        else:
            # Stable baseline: detect absolute jump above ANOMALY_THRESHOLD
            jump = current - mean
            if jump >= ANOMALY_THRESHOLD and current >= ANOMALY_THRESHOLD:
                severity = min(jump / (ANOMALY_THRESHOLD * 2), 1.0)
                return ResidualPattern(
                    pattern_type="SPIKE",
                    severity=severity,
                    evidence={
                        "current": round(current, 3),
                        "mean": round(mean, 3),
                        "std": round(std, 4),
                        "jump": round(jump, 3),
                        "mode": "absolute",
                    },
                )
        return None

    def _check_stable_high(self) -> Optional[ResidualPattern]:
        """STABLE_HIGH: N consecutive judgments above threshold."""
        if self._consecutive_high >= STABLE_HIGH_N:
            # Severity rises with consecutive count
            severity = min(self._consecutive_high / (STABLE_HIGH_N * 2), 1.0)
            return ResidualPattern(
                pattern_type="STABLE_HIGH",
                severity=severity,
                evidence={
                    "consecutive_high": self._consecutive_high,
                    "threshold": ANOMALY_THRESHOLD,
                    "required": STABLE_HIGH_N,
                },
            )
        return None

    def _check_rising(self, history: List[float]) -> Optional[ResidualPattern]:
        """RISING: Linear slope across last N points > RISING_SLOPE_THRESHOLD."""
        n = len(history)
        if n < MIN_SAMPLES * 2:  # Need more points for reliable slope
            return None

        # Simple linear regression slope
        xs = list(range(n))
        mean_x = sum(xs) / n
        mean_y = sum(history) / n
        num = sum((x - mean_x) * (y - mean_y) for x, y in zip(xs, history))
        den = sum((x - mean_x) ** 2 for x in xs)

        if den == 0:
            return None

        slope = num / den

        if slope >= RISING_SLOPE_THRESHOLD:
            severity = min(slope / (RISING_SLOPE_THRESHOLD * 3), 1.0)
            return ResidualPattern(
                pattern_type="RISING",
                severity=severity,
                evidence={
                    "slope": round(slope, 4),
                    "threshold": RISING_SLOPE_THRESHOLD,
                    "window": n,
                },
            )
        return None

    # ── Event Handler ─────────────────────────────────────────────────────

    async def _handle_judgment_event(self, event: Event) -> None:
        """Handle JUDGMENT_CREATED event — extract residual and observe."""
        payload = event.payload
        residual = payload.get("residual_variance", 0.0)
        unnameable = payload.get("unnameable_detected", False)

        # Build a minimal observation from event payload (no full Judgment object)
        point = ResidualPoint(
            judgment_id=payload.get("judgment_id", ""),
            residual=residual,
            reality=payload.get("cell", {}).get("reality", "UNKNOWN"),
            analysis=payload.get("cell", {}).get("analysis", "UNKNOWN"),
            unnameable=unnameable,
        )
        self._observations += 1
        self._history.append(point)
        self._maybe_persist(point)

        if residual >= ANOMALY_THRESHOLD:
            self._anomalies += 1
            self._consecutive_high += 1
        else:
            self._consecutive_high = 0

        if len(self._history) < MIN_SAMPLES:
            return

        history_vals = [p.residual for p in self._history]
        pattern = (
            self._check_spike(residual, history_vals) or
            self._check_stable_high() or
            self._check_rising(history_vals)
        )

        if pattern:
            self._patterns_detected += 1
            self._patterns.append(pattern)
            if len(self._patterns) > 100:
                self._patterns.pop(0)

            # Emit EMERGENCE_DETECTED
            await get_core_bus().emit(Event(
                type=CoreEvent.EMERGENCE_DETECTED,
                source="residual_detector",
                payload={
                    "pattern_type": pattern.pattern_type,
                    "severity": pattern.severity,
                    "evidence": pattern.evidence,
                    "judgment_id": point.judgment_id,
                    "reality": point.reality,
                    "analysis": point.analysis,
                    "total_anomalies": self._anomalies,
                    "total_patterns": self._patterns_detected,
                },
            ))
            logger.info(
                "EMERGENCE_DETECTED emitted: pattern=%s severity=%.3f",
                pattern.pattern_type, pattern.severity,
            )
