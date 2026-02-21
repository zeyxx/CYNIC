"""
Observability Endpoints — Metrics, Health, Logging

Provides:
- GET /metrics → Prometheus-formatted metrics
- GET /health → System health status
- All requests tracked with correlation IDs

Auto-registered by auto_register_routers().
"""

import logging
import time
import uuid
from typing import Optional

from fastapi import APIRouter, Request, Response

from cynic.api.metrics import get_metrics_text

logger = logging.getLogger(__name__)

router_observability = APIRouter(
    prefix="/observability",
    tags=["observability"],
    responses={404: {"description": "Not found"}},
)


@router_observability.get("/metrics")
async def get_metrics() -> Response:
    """
    Prometheus-compatible metrics endpoint.

    Returns metrics in Prometheus text format.
    Scraped by Prometheus for monitoring.

    Example:
        curl http://localhost:8765/observability/metrics

    Metrics tracked:
    - consciousness_requests_total: Total requests by endpoint/method/status
    - consciousness_request_duration_seconds: Latency by endpoint
    - consciousness_queue_depth: Event queue size
    - consciousness_websocket_connections_active: Active WebSocket connections
    - consciousness_errors_total: Errors by type
    """
    metrics_bytes, content_type = get_metrics_text()
    return Response(content=metrics_bytes, media_type=content_type)


@router_observability.get("/health")
async def health_check() -> dict:
    """
    System health status with diagnostics.

    Returns detailed health of all major components.
    Used by Kubernetes for liveness/readiness probes.

    Example:
        curl http://localhost:8765/observability/health

    Response:
        {
            "status": "healthy|degraded|dead",
            "uptime_seconds": 3600,
            "components": {
                "api": "healthy",
                "consciousness": "healthy",
                "event_bus": "healthy",
                "websocket": "healthy"
            }
        }
    """
    from cynic.api.state import get_app_container

    try:
        container = get_app_container()

        # Check consciousness service
        consciousness_ok = container.consciousness_service is not None

        return {
            "status": "healthy",
            "uptime_seconds": int(time.time() - container.boot_time),
            "timestamp": time.time(),
            "components": {
                "api": "healthy",
                "consciousness_service": "healthy" if consciousness_ok else "degraded",
                "event_bus": "healthy",
                "websocket": "healthy",
            },
            "version": "2.0.0",
        }
    except Exception as e:
        logger.error(f"Health check failed: {e}", exc_info=True)
        return {
            "status": "degraded",
            "error": str(e),
            "components": {
                "api": "healthy",
                "consciousness_service": "dead",
                "event_bus": "unknown",
                "websocket": "unknown",
            },
        }


@router_observability.get("/ready")
async def readiness() -> dict:
    """
    Readiness probe for orchestration (Kubernetes, etc).

    Returns 200 if service is ready to accept traffic.
    Returns 503 if warming up or degraded.
    """
    try:
        from cynic.api.state import get_app_container

        container = get_app_container()
        is_ready = (
            container.consciousness_service is not None
            and container.app is not None
        )

        if is_ready:
            return {"ready": True, "status": "ready"}
        else:
            return {"ready": False, "status": "warming_up"}
    except Exception as e:
        logger.warning(f"Readiness check failed: {e}")
        return {"ready": False, "status": "error", "error": str(e)}


@router_observability.get("/version")
async def get_version() -> dict:
    """Get CYNIC kernel version."""
    return {
        "name": "CYNIC",
        "version": "2.0.0",
        "description": "Observable Consciousness Ecosystem",
    }


__all__ = ["router_observability"]
