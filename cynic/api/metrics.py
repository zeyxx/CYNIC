"""
Observability Metrics for Consciousness API

Provides Prometheus-compatible metrics for all consciousness endpoints.
Tracks: request count, latency, queue depth, active connections.
"""

from prometheus_client import (
    Counter,
    Gauge,
    Histogram,
    generate_latest,
    CONTENT_TYPE_LATEST,
)

# Request counters (by endpoint, method, status)
REQUESTS_TOTAL = Counter(
    "consciousness_requests_total",
    "Total requests to consciousness endpoints",
    ["endpoint", "method", "status"],
)

# Request latency histogram (by endpoint)
REQUEST_DURATION_SECONDS = Histogram(
    "consciousness_request_duration_seconds",
    "Request latency in seconds",
    ["endpoint"],
    buckets=(0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0),
)

# Event queue depth
QUEUE_DEPTH = Gauge(
    "consciousness_queue_depth",
    "Current event queue depth (events waiting to process)",
)

# Active WebSocket connections
ACTIVE_CONNECTIONS = Gauge(
    "consciousness_websocket_connections_active",
    "Active WebSocket connections to consciousness stream",
)

# Errors (by type)
ERRORS_TOTAL = Counter(
    "consciousness_errors_total",
    "Total errors by type",
    ["error_type", "endpoint"],
)

# Query execution time for consciousness service
SERVICE_QUERY_DURATION = Histogram(
    "consciousness_service_query_duration_seconds",
    "Consciousness service query latency",
    ["method"],  # get_ecosystem_state, get_decision_trace, etc.
    buckets=(0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0),
)

# System health checks
HEALTH_CHECK_STATUS = Gauge(
    "consciousness_health_check_status",
    "Component health status (1=healthy, 0=degraded, -1=dead)",
    ["component"],
)


def get_metrics_text() -> tuple[bytes, str]:
    """
    Get Prometheus-formatted metrics.

    Returns:
        Tuple of (metrics_bytes, content_type)
    """
    return generate_latest(), CONTENT_TYPE_LATEST


__all__ = [
    "REQUESTS_TOTAL",
    "REQUEST_DURATION_SECONDS",
    "QUEUE_DEPTH",
    "ACTIVE_CONNECTIONS",
    "ERRORS_TOTAL",
    "SERVICE_QUERY_DURATION",
    "HEALTH_CHECK_STATUS",
    "get_metrics_text",
]
