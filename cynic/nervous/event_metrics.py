"""
EventMetricsCollector - Rolling-window metrics over the EventBus stream.

Tracks per-event-type rates, latency histograms, and error rates
within a -derived observation window (SIGNAL_TTL_SEC = 55 s).

Anomaly types:
  RATE_SPIKE   - recent rate > baseline  PHI (1.618)
  ERROR_SPIKE  - error_rate > PHI_INV (0.618)
  LATENCY_SPIKE - duration_ms > LOD_LEVEL3 (3000 ms)

Usage:
    collector = EventMetricsCollector()
    await collector.record("core.judgment_created", duration_ms=120.0)
    metrics = await collector.get_metrics("core.judgment_created")
    anomalies = await collector.detect_anomalies()
"""

from __future__ import annotations

import asyncio
import logging
import time
from collections import defaultdict, deque
from dataclasses import dataclass
from typing import Any

from cynic.kernel.core.formulas import (
    LOG_TAIL_CAP,
    LOD_LEVEL0_LATENCY_MS,
    LOD_LEVEL1_LATENCY_MS,
    LOD_LEVEL2_LATENCY_MS,
    LOD_LEVEL3_LATENCY_MS,
    SIGNAL_TTL_SEC,
)
from cynic.kernel.core.phi import PHI, PHI_INV

logger = logging.getLogger("cynic.nervous.event_metrics")

# Histogram bucket boundaries and labels (inclusive upper bound)
_BUCKETS: list[tuple[float, str]] = [
    (LOD_LEVEL0_LATENCY_MS, f"≤{int(LOD_LEVEL0_LATENCY_MS)}ms"),  # 100ms  REFLEX
    (LOD_LEVEL1_LATENCY_MS, f"≤{int(LOD_LEVEL1_LATENCY_MS)}ms"),  # 300ms  MICRO
    (LOD_LEVEL2_LATENCY_MS, f"≤{int(LOD_LEVEL2_LATENCY_MS)}ms"),  # 1000ms MACRO
    (LOD_LEVEL3_LATENCY_MS, f"≤{int(LOD_LEVEL3_LATENCY_MS)}ms"),  # 3000ms META
    (float("inf"), f">{int(LOD_LEVEL3_LATENCY_MS)}ms"),  # >3000ms OVER
]


@dataclass
class _EventSample:
    """Single event sample in the rolling buffer."""

    timestamp_ms: float
    duration_ms: float
    is_error: bool


@dataclass
class EventTypeMetrics:
    """Computed metrics for a single event type within the observation window."""

    event_type: str
    count_in_window: int  # events within SIGNAL_TTL_SEC
    rate_per_min: float  # count_in_window  60 / SIGNAL_TTL_SEC
    error_count: int
    error_rate: float  # error_count / max(count_in_window, 1)
    histogram: dict[str, int]  # bucket_label  count (duration_ms distribution)
    last_seen_ms: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "event_type": self.event_type,
            "count_in_window": self.count_in_window,
            "rate_per_min": self.rate_per_min,
            "error_count": self.error_count,
            "error_rate": self.error_rate,
            "histogram": self.histogram,
            "last_seen_ms": self.last_seen_ms,
        }


@dataclass
class AnomalyRecord:
    """A detected anomaly from the metrics stream."""

    detected_at_ms: float
    anomaly_type: str  # "RATE_SPIKE" | "ERROR_SPIKE" | "LATENCY_SPIKE"
    event_type: str
    metric_value: float  # observed value that crossed the threshold
    threshold_value: float  # the PHI-derived threshold that was crossed
    severity: float  # [0, 1] - capped at PHI_INV (0.618)
    message: str

    def to_dict(self) -> dict[str, Any]:
        return {
            "detected_at_ms": self.detected_at_ms,
            "anomaly_type": self.anomaly_type,
            "event_type": self.event_type,
            "metric_value": self.metric_value,
            "threshold_value": self.threshold_value,
            "severity": self.severity,
            "message": self.message,
        }


class EventMetricsCollector:
    """
    Rolling-window metrics over the EventBus stream.
    Thread-safe (asyncio.Lock). No external dependencies.
    """

    def __init__(self, window_s: float = SIGNAL_TTL_SEC) -> None:
        self._window_ms = window_s * 1000.0
        self._lock = asyncio.Lock()
        # Per-type rolling samples - capped at LOG_TAIL_CAP (144) per type
        self._samples: dict[str, deque[_EventSample]] = defaultdict(
            lambda: deque(maxlen=LOG_TAIL_CAP)
        )
        # Anomaly rolling buffer - capped at LOG_TAIL_CAP (144)
        self._anomalies: deque[AnomalyRecord] = deque(maxlen=LOG_TAIL_CAP)
        self._total_recorded = 0

    async def record(
        self,
        event_type: str,
        duration_ms: float = 0.0,
        is_error: bool = False,
    ) -> None:
        """Record a single event observation."""
        async with self._lock:
            now_ms = time.time() * 1000.0
            self._samples[event_type].append(
                _EventSample(
                    timestamp_ms=now_ms, duration_ms=duration_ms, is_error=is_error
                )
            )
            self._total_recorded += 1

    async def get_metrics(self, event_type: str) -> EventTypeMetrics | None:
        """Compute metrics for a single event type within the rolling window."""
        async with self._lock:
            deque_ = self._samples.get(event_type)
            if not deque_:
                return None
            window_samples = self._in_window(list(deque_))
            return self._compute_metrics(event_type, window_samples)

    async def all_metrics(self) -> dict[str, EventTypeMetrics]:
        """Compute metrics for all tracked event types."""
        async with self._lock:
            result = {}
            for event_type, deque_ in self._samples.items():
                window_samples = self._in_window(list(deque_))
                if window_samples:
                    result[event_type] = self._compute_metrics(
                        event_type, window_samples
                    )
            return result

    async def current_rates(self) -> dict[str, float]:
        """Events-per-minute rate for each type in the current window."""
        async with self._lock:
            rates = {}
            for event_type, deque_ in self._samples.items():
                window_samples = self._in_window(list(deque_))
                count = len(window_samples)
                rates[event_type] = count * 60.0 / (self._window_ms / 1000.0)
            return rates

    async def detect_anomalies(self) -> list[AnomalyRecord]:
        """
        Scan all tracked types for PHI-threshold anomalies.
        Stores new anomalies in rolling buffer. Returns newly detected ones.
        """
        async with self._lock:
            now_ms = time.time() * 1000.0
            half_window = self._window_ms / 2.0
            newly_detected = []

            for event_type, deque_ in self._samples.items():
                all_samples = list(deque_)
                recent = [
                    s for s in all_samples if (now_ms - s.timestamp_ms) <= half_window
                ]
                older = [
                    s
                    for s in all_samples
                    if half_window < (now_ms - s.timestamp_ms) <= self._window_ms
                ]

                # RATE_SPIKE: recent half-window rate > older half-window rate  PHI
                if older and recent:
                    recent_rate = len(recent) / (half_window / 1000.0 / 60.0)  # /min
                    older_rate = len(older) / (half_window / 1000.0 / 60.0)
                    threshold = older_rate * PHI
                    if older_rate > 0 and recent_rate > threshold:
                        severity = min(
                            (recent_rate / threshold - 1.0) * PHI_INV, PHI_INV
                        )
                        record = AnomalyRecord(
                            detected_at_ms=now_ms,
                            anomaly_type="RATE_SPIKE",
                            event_type=event_type,
                            metric_value=recent_rate,
                            threshold_value=threshold,
                            severity=severity,
                            message=f"{event_type} rate {recent_rate:.1f}/min > {threshold:.1f}/min (PHI baseline)",
                        )
                        self._anomalies.append(record)
                        newly_detected.append(record)

                # ERROR_SPIKE: error_rate in window > PHI_INV (0.618)
                window_samples = self._in_window(all_samples)
                if window_samples:
                    error_rate = sum(1 for s in window_samples if s.is_error) / len(
                        window_samples
                    )
                    if error_rate > PHI_INV:
                        record = AnomalyRecord(
                            detected_at_ms=now_ms,
                            anomaly_type="ERROR_SPIKE",
                            event_type=event_type,
                            metric_value=error_rate,
                            threshold_value=PHI_INV,
                            severity=min(error_rate, PHI_INV),
                            message=f"{event_type} error rate {error_rate:.1%} > {PHI_INV:.1%} (PHI_INV)",
                        )
                        self._anomalies.append(record)
                        newly_detected.append(record)

                # LATENCY_SPIKE: any sample in window exceeds LOD_LEVEL3 (3000ms)
                latency_outliers = [
                    s for s in window_samples if s.duration_ms > LOD_LEVEL3_LATENCY_MS
                ]
                for outlier in latency_outliers[
                    :1
                ]:  # at most 1 per check to avoid flood
                    record = AnomalyRecord(
                        detected_at_ms=now_ms,
                        anomaly_type="LATENCY_SPIKE",
                        event_type=event_type,
                        metric_value=outlier.duration_ms,
                        threshold_value=LOD_LEVEL3_LATENCY_MS,
                        severity=min(
                            outlier.duration_ms / LOD_LEVEL3_LATENCY_MS * PHI_INV,
                            PHI_INV,
                        ),
                        message=f"{event_type} latency {outlier.duration_ms:.0f}ms > {LOD_LEVEL3_LATENCY_MS:.0f}ms (LOD3)",
                    )
                    self._anomalies.append(record)
                    newly_detected.append(record)

            return newly_detected

    async def recent_anomalies(self, limit: int = LOG_TAIL_CAP) -> list[AnomalyRecord]:
        """Return most recent anomaly records (newest first)."""
        async with self._lock:
            records = list(self._anomalies)
            return records[-limit:][::-1]

    async def stats(self) -> dict[str, Any]:
        """Summary statistics."""
        async with self._lock:
            return {
                "total_recorded": self._total_recorded,
                "tracked_types": len(self._samples),
                "anomaly_count": len(self._anomalies),
                "window_s": self._window_ms / 1000.0,
            }

    # -- Private helpers --------------------------------------------------------

    def _in_window(self, samples: list[_EventSample]) -> list[_EventSample]:
        """Filter samples to those within the rolling window."""
        cutoff = time.time() * 1000.0 - self._window_ms
        return [s for s in samples if s.timestamp_ms >= cutoff]

    def _bucket_label(self, duration_ms: float) -> str:
        for upper, label in _BUCKETS:
            if duration_ms <= upper:
                return label
        return _BUCKETS[-1][1]

    def _compute_metrics(
        self, event_type: str, window_samples: list[_EventSample]
    ) -> EventTypeMetrics:
        count = len(window_samples)
        error_count = sum(1 for s in window_samples if s.is_error)
        histogram: dict[str, int] = {label: 0 for _, label in _BUCKETS}
        last_seen_ms = 0.0
        for s in window_samples:
            histogram[self._bucket_label(s.duration_ms)] += 1
            if s.timestamp_ms > last_seen_ms:
                last_seen_ms = s.timestamp_ms
        return EventTypeMetrics(
            event_type=event_type,
            count_in_window=count,
            rate_per_min=count * 60.0 / (self._window_ms / 1000.0),
            error_count=error_count,
            error_rate=error_count / max(count, 1),
            histogram=histogram,
            last_seen_ms=last_seen_ms,
        )
