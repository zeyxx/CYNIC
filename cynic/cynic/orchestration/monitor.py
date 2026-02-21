"""
Health Monitoring — Continuous health checks, metrics, alerts.

CYNIC monitors itself.
"""
from __future__ import annotations

import asyncio
import logging
from dataclasses import dataclass, asdict
from datetime import datetime, timedelta
from typing import Optional

logger = logging.getLogger(__name__)


@dataclass
class MetricPoint:
    """Single metric data point."""
    name: str
    value: float
    unit: str
    timestamp: datetime
    tags: Optional[dict[str, str]] = None

    def to_dict(self):
        return {**asdict(self), "timestamp": self.timestamp.isoformat()}


@dataclass
class Alert:
    """Health alert."""
    service: str
    severity: str  # "info", "warning", "error"
    message: str
    timestamp: datetime
    resolved: bool = False

    def to_dict(self):
        return {**asdict(self), "timestamp": self.timestamp.isoformat()}


class HealthMonitor:
    """Monitors CYNIC's health continuously."""

    def __init__(self):
        self._metrics: list[MetricPoint] = []
        self._alerts: list[Alert] = []
        self._running = False

    async def record_metric(
        self,
        name: str,
        value: float,
        unit: str = "",
        tags: Optional[dict[str, str]] = None,
    ) -> None:
        """Record a metric point."""
        metric = MetricPoint(
            name=name,
            value=value,
            unit=unit,
            timestamp=datetime.now(),
            tags=tags,
        )
        self._metrics.append(metric)

        # Keep only last 24 hours
        cutoff = datetime.now() - timedelta(hours=24)
        self._metrics = [m for m in self._metrics if m.timestamp > cutoff]

    async def create_alert(
        self,
        service: str,
        severity: str,
        message: str,
    ) -> Alert:
        """Create an alert."""
        alert = Alert(
            service=service,
            severity=severity,
            message=message,
            timestamp=datetime.now(),
        )
        self._alerts.append(alert)
        logger.warning(f"[{severity.upper()}] {service}: {message}")
        return alert

    async def resolve_alert(self, alert: Alert) -> None:
        """Mark alert as resolved."""
        alert.resolved = True
        logger.info(f"Alert resolved: {alert.service}")

    async def get_metrics(
        self,
        name: Optional[str] = None,
        hours: int = 24,
    ) -> list[MetricPoint]:
        """Get metrics from last N hours."""
        cutoff = datetime.now() - timedelta(hours=hours)
        metrics = [m for m in self._metrics if m.timestamp > cutoff]

        if name:
            metrics = [m for m in metrics if m.name == name]

        return metrics

    async def get_active_alerts(self) -> list[Alert]:
        """Get unresolved alerts."""
        return [a for a in self._alerts if not a.resolved]

    async def get_service_status(self, service: str) -> dict:
        """Get aggregated status for a service."""
        metrics = await self.get_metrics(hours=1)
        alerts = await self.get_active_alerts()

        service_metrics = [m for m in metrics if m.tags and m.tags.get("service") == service]
        service_alerts = [a for a in alerts if a.service == service]

        status = "healthy"
        if service_alerts:
            status = "degraded" if any(a.severity == "warning" for a in service_alerts) else "error"

        return {
            "service": service,
            "status": status,
            "metrics": [m.to_dict() for m in service_metrics],
            "alerts": [a.to_dict() for a in service_alerts],
            "timestamp": datetime.now().isoformat(),
        }

    async def start_monitoring(self, interval_seconds: int = 30) -> None:
        """Start continuous monitoring loop."""
        self._running = True
        logger.info(f"Health monitoring started (interval: {interval_seconds}s)")

        while self._running:
            try:
                # Record metrics (these would come from actual health checks)
                await self.record_metric(
                    "cpu_usage_percent",
                    0.0,  # TODO: Get actual CPU usage
                    "%",
                    tags={"service": "cynic-kernel"},
                )
                await self.record_metric(
                    "memory_usage_bytes",
                    0,  # TODO: Get actual memory usage
                    "bytes",
                    tags={"service": "cynic-kernel"},
                )

                await asyncio.sleep(interval_seconds)
            except Exception as e:
                logger.exception("Monitoring error")
                await asyncio.sleep(interval_seconds)

    async def stop_monitoring(self) -> None:
        """Stop monitoring loop."""
        self._running = False
        logger.info("Health monitoring stopped")
