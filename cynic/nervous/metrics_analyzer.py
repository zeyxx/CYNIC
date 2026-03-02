"""
MetricsAnalyzer — Translate EventMetricsCollector output into SelfProposal recommendations.

Used by SelfProber to generate METRICS dimension proposals from anomaly records.
"""
from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

from cynic.kernel.core.phi import PHI

if TYPE_CHECKING:
    from cynic.nervous.event_metrics import AnomalyRecord, EventMetricsCollector

logger = logging.getLogger("cynic.nervous.metrics_analyzer")


@dataclass
class MetricsProposal:
    """One metrics-driven improvement recommendation."""
    anomaly_type: str           # RATE_SPIKE | ERROR_SPIKE | LATENCY_SPIKE
    event_type: str
    metric_value: float
    threshold_value: float
    severity: float             # [0, 1]
    target: str                 # event_type (what to improve)
    recommendation: str         # Human-readable action
    current_value: float
    suggested_value: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "anomaly_type": self.anomaly_type,
            "event_type": self.event_type,
            "metric_value": round(self.metric_value, 2),
            "threshold_value": round(self.threshold_value, 2),
            "severity": round(self.severity, 4),
            "target": self.target,
            "recommendation": self.recommendation,
            "current_value": round(self.current_value, 2),
            "suggested_value": round(self.suggested_value, 2),
        }


class MetricsAnalyzer:
    """
    Analyze EventMetricsCollector anomaly records and generate improvement proposals.

    Maps three anomaly types to actionable recommendations:
      - RATE_SPIKE: event frequency spiked; suggest circuit breaker or batching
      - ERROR_SPIKE: error rate exceeded threshold; suggest fallback or timeout
      - LATENCY_SPIKE: event took >3000ms; suggest optimization or async conversion
    """

    def __init__(self, collector: EventMetricsCollector) -> None:
        self._collector = collector

    def analyze_anomalies(
        self,
        anomalies: list[AnomalyRecord],
        severity_threshold: float = 0.0,
    ) -> list[MetricsProposal]:
        """
        Translate anomaly records into proposals.

        Args:
            anomalies: List of AnomalyRecord from EventMetricsCollector.detect_anomalies()
            severity_threshold: Only consider anomalies with severity >= threshold

        Returns:
            List of MetricsProposal recommendations
        """
        proposals: list[MetricsProposal] = []

        for anomaly in anomalies:
            if anomaly.severity < severity_threshold:
                continue

            if anomaly.anomaly_type == "RATE_SPIKE":
                proposals.append(self._analyze_rate_spike(anomaly))
            elif anomaly.anomaly_type == "ERROR_SPIKE":
                proposals.append(self._analyze_error_spike(anomaly))
            elif anomaly.anomaly_type == "LATENCY_SPIKE":
                proposals.append(self._analyze_latency_spike(anomaly))

        return proposals

    def _analyze_rate_spike(self, anomaly: AnomalyRecord) -> MetricsProposal:
        """Rate spike: event frequency exceeded PHI × baseline."""
        suggested_rate = anomaly.metric_value / PHI  # Scale back by PHI
        return MetricsProposal(
            anomaly_type="RATE_SPIKE",
            event_type=anomaly.event_type,
            metric_value=anomaly.metric_value,
            threshold_value=anomaly.threshold_value,
            severity=anomaly.severity,
            target=anomaly.event_type,
            recommendation=(
                f"Rate spike detected ({anomaly.metric_value:.1f}/min). "
                f"Consider batching events or implementing backpressure throttling."
            ),
            current_value=anomaly.metric_value,
            suggested_value=suggested_rate,
        )

    def _analyze_error_spike(self, anomaly: AnomalyRecord) -> MetricsProposal:
        """Error spike: error rate exceeded PHI_INV (61.8%)."""
        suggested_error_rate = anomaly.metric_value / 2.0  # Reduce by half
        return MetricsProposal(
            anomaly_type="ERROR_SPIKE",
            event_type=anomaly.event_type,
            metric_value=anomaly.metric_value,
            threshold_value=anomaly.threshold_value,
            severity=anomaly.severity,
            target=anomaly.event_type,
            recommendation=(
                f"Error rate spike detected ({anomaly.metric_value:.1%}). "
                f"Implement fallback handler or add circuit breaker for {anomaly.event_type}."
            ),
            current_value=anomaly.metric_value,
            suggested_value=suggested_error_rate,
        )

    def _analyze_latency_spike(self, anomaly: AnomalyRecord) -> MetricsProposal:
        """Latency spike: event duration exceeded LOD_LEVEL3 (3000ms)."""
        suggested_latency = 1000.0  # Target 1000ms (LOD_LEVEL2)
        return MetricsProposal(
            anomaly_type="LATENCY_SPIKE",
            event_type=anomaly.event_type,
            metric_value=anomaly.metric_value,
            threshold_value=anomaly.threshold_value,
            severity=anomaly.severity,
            target=anomaly.event_type,
            recommendation=(
                f"Latency spike detected ({anomaly.metric_value:.0f}ms). "
                f"Consider optimizing {anomaly.event_type} or converting to async."
            ),
            current_value=anomaly.metric_value,
            suggested_value=suggested_latency,
        )
