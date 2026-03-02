"""
Prometheus Metrics Bridge — HTTP /metrics endpoint

Exports EventMetricsCollector data in OpenMetrics format for Prometheus scraping.
No external prometheus_client dependency — pure format generation.
"""
from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from fastapi import APIRouter, Request, Response
from cynic.kernel.core.event_bus import get_core_bus

if TYPE_CHECKING:
    pass

logger = logging.getLogger("cynic.interfaces.api.routers.metrics")

router = APIRouter(prefix="/metrics", tags=["observability"])


def _format_openmetrics(metrics_data: dict) -> str:
    """
    Format metrics dict as OpenMetrics text format.

    OpenMetrics format:
    # HELP metric_name Description
    # TYPE metric_name gauge|counter|histogram
    metric_name{labels} value
    """
    lines = []

    if not metrics_data:
        return "# No metrics available\n"

    # Event counts
    if "event_counts" in metrics_data:
        lines.append("# HELP cynic_event_count Total events recorded by type")
        lines.append("# TYPE cynic_event_count counter")
        for event_type, count in metrics_data["event_counts"].items():
            # Sanitize event_type for Prometheus label
            safe_name = event_type.replace(".", "_").replace("-", "_")
            lines.append(f'cynic_event_count{{event_type="{safe_name}"}} {count}')

    # Event rates
    if "event_rates" in metrics_data:
        lines.append("# HELP cynic_event_rate_per_min Events per minute by type")
        lines.append("# TYPE cynic_event_rate_per_min gauge")
        for event_type, rate in metrics_data["event_rates"].items():
            safe_name = event_type.replace(".", "_").replace("-", "_")
            lines.append(f'cynic_event_rate_per_min{{event_type="{safe_name}"}} {rate:.2f}')

    # Error rates
    if "error_rates" in metrics_data:
        lines.append("# HELP cynic_error_rate Error rate by event type")
        lines.append("# TYPE cynic_error_rate gauge")
        for event_type, rate in metrics_data["error_rates"].items():
            safe_name = event_type.replace(".", "_").replace("-", "_")
            lines.append(f'cynic_error_rate{{event_type="{safe_name}"}} {rate:.4f}')

    # Latency histograms (5 LOD buckets)
    if "histograms" in metrics_data:
        lines.append("# HELP cynic_event_latency_seconds Event latency distribution")
        lines.append("# TYPE cynic_event_latency_seconds histogram")
        for event_type, buckets in metrics_data["histograms"].items():
            safe_name = event_type.replace(".", "_").replace("-", "_")
            # Output histogram buckets
            for bucket_label, count in buckets.items():
                lines.append(f'cynic_event_latency_seconds_bucket{{event_type="{safe_name}",le="{bucket_label}"}} {count}')

    # Anomaly count
    if "anomaly_count" in metrics_data:
        lines.append("# HELP cynic_anomalies_detected Total anomalies detected")
        lines.append("# TYPE cynic_anomalies_detected counter")
        lines.append(f'cynic_anomalies_detected {metrics_data["anomaly_count"]}')

    # Bus stats
    if "bus_stats" in metrics_data:
        bus = metrics_data["bus_stats"]
        lines.append("# HELP cynic_bus_emitted Events emitted on bus")
        lines.append("# TYPE cynic_bus_emitted counter")
        lines.append(f'cynic_bus_emitted {bus.get("emitted", 0)}')

        lines.append("# HELP cynic_bus_pending Pending tasks on bus")
        lines.append("# TYPE cynic_bus_pending gauge")
        lines.append(f'cynic_bus_pending {bus.get("pending_tasks", 0)}')

        lines.append("# HELP cynic_bus_errors Bus handler errors")
        lines.append("# TYPE cynic_bus_errors counter")
        lines.append(f'cynic_bus_errors {bus.get("errors", 0)}')

    lines.append("# EOF")
    return "\n".join(lines) + "\n"


@router.get("", response_class=Response)
async def metrics(request: Request) -> Response:
    """
    Prometheus-compatible metrics endpoint.

    Returns OpenMetrics format with:
    - Event counts and rates by type
    - Error rates by type
    - Anomaly detection counters
    - Bus health metrics
    """
    try:
        # Try to get metrics from the global bus
        bus = get_core_bus("DEFAULT")
        metrics_data = {}

        # Bus stats (always available)
        if hasattr(bus, "stats"):
            metrics_data["bus_stats"] = bus.stats()

        # EventMetricsCollector stats (if available)
        if hasattr(bus, "_metrics_adapter") and hasattr(bus._metrics_adapter, "_collector"):
            collector = bus._metrics_adapter._collector

            # Get current rates
            try:
                rates = await collector.current_rates()
                metrics_data["event_rates"] = rates
            except Exception as e:
                logger.warning("Failed to get event rates: %s", e, exc_info=True)

            # Get metrics for all types
            try:
                all_metrics = await collector.all_metrics()
                metrics_data["event_counts"] = {
                    m.event_type: m.count_in_window
                    for m in all_metrics.values()
                }
                metrics_data["error_rates"] = {
                    m.event_type: m.error_rate
                    for m in all_metrics.values()
                }
                # Get histograms
                metrics_data["histograms"] = {
                    m.event_type: m.histogram
                    for m in all_metrics.values()
                }
            except Exception as e:
                logger.warning("Failed to get all metrics: %s", e, exc_info=True)

            # Get anomaly count
            try:
                anomalies = await collector.recent_anomalies(limit=1000)
                metrics_data["anomaly_count"] = len(anomalies)
            except Exception as e:
                logger.warning("Failed to get anomalies: %s", e, exc_info=True)

        text = _format_openmetrics(metrics_data)
        return Response(content=text, media_type="application/openmetrics-text; charset=utf-8")

    except Exception as e:
        logger.error(f"Metrics endpoint error: {e}", exc_info=True)
        return Response(
            content="# Error generating metrics\n",
            media_type="application/openmetrics-text; charset=utf-8",
            status_code=200,  # Prometheus expects 200 even on partial failures
        )
