"""
CYNIC health router - core vitals: health  stats
"""
from __future__ import annotations

import asyncio
import logging
import os
import time
from typing import Any

try:
    import psutil
    PSUTIL_AVAILABLE = True
except ImportError:
    PSUTIL_AVAILABLE = False

from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import ValidationError

from cynic.interfaces.api.models import (
    HealthEventsResponse,
    HealthFullResponse,
    HealthReadyResponse,
    HealthResponse,
    RootResponse,
    StatsResponse,
)
from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.kernel.core.consciousness import get_consciousness
from cynic.kernel.core.phi import PHI

logger = logging.getLogger("cynic.interfaces.api.server")

router_health = APIRouter(tags=["health"])

_CONSCIOUSNESS_FILE = os.path.join(os.path.expanduser("~"), ".cynic", "consciousness.json")


# -- Root route: API alive status ---------------------------------------------
@router_health.get("/", response_model=RootResponse, include_in_schema=True)
async def root(request: Request) -> RootResponse:
    """
    Root endpoint - CYNIC kernel is alive.
    """
    # Collect available routes from the app
    app = request.app
    routes = []
    for route in app.routes:
        if hasattr(route, "path") and not any(
            route.path.startswith(p) for p in ["/openapi", "/docs", "/redoc", "/static"]
        ):
            routes.append(route.path)
    routes.sort()

    return RootResponse(
        status="AWAKE",
        name="CYNIC Kernel",
        PHI=PHI,  # The golden ratio
        routes=routes,
    )


# ==============================================================================
# GET /health
# ==============================================================================

@router_health.get("/health", response_model=HealthResponse)
async def health(container: AppContainer = Depends(get_app_container)) -> HealthResponse:
    """
    Kernel health - the organism's vital signs.
    """
    state = container.organism
    consciousness = get_consciousness()
    judge_stats = state.cognition.orchestrator.stats()
    learn_stats = state.cognition.qtable.stats()

    sched_stats = state.metabolism.scheduler.stats()

    # Determine status
    status = "alive"
    if not state.cognition.learning_loop._active:
        status = "degraded"

    # check SurrealDB singleton status
    _storage_status: dict[str, Any] = {}
    try:
        from cynic.kernel.core.storage.surreal import get_storage as _get_storage
        _get_storage()
        _storage_status["surreal"] = "connected"
    except RuntimeError:
        _storage_status["surreal"] = "disconnected"
    except Exception:
        _storage_status["surreal"] = "error"

    return HealthResponse(
        status=status,
        uptime_s=round(state.uptime_s, 1),
        consciousness=consciousness.model_dump(),
        dogs=[str(d) for d in state.dogs.keys()],
        learning={
            "active": state.cognition.learning_loop._active,
            "states": learn_stats["states"],
            "total_updates": learn_stats["total_updates"],
            "pending_flush": learn_stats["pending_flush"],
        },
        scheduler=sched_stats,
        llm_adapters=[
            a.adapter_id for a in __import__(
                "cynic.kernel.organism.brain.llm.adapter", 
                fromlist=["get_registry"]
            ).get_registry().get_available()
        ],
        judgments_total=judge_stats["judgments_total"],
        phi=PHI,
        storage=_storage_status,
    )


# ==============================================================================
# GET /stats
# ==============================================================================

@router_health.get("/stats", response_model=StatsResponse)
async def stats(container: AppContainer = Depends(get_app_container)) -> StatsResponse:
    """Detailed kernel metrics - everything CYNIC knows about itself."""
    state = container.organism
    
    return StatsResponse(
        judgments=state.cognition.orchestrator.stats(),
        learning=state.cognition.qtable.stats(),
        metabolism=state.metabolism.scheduler.stats(),
        timestamp=time.time(),
    )


# ==============================================================================
# GET /health/events
# ==============================================================================

@router_health.get("/health/events", response_model=HealthEventsResponse)
async def health_events(container: AppContainer = Depends(get_app_container)) -> HealthEventsResponse:
    """Event handler pipeline health + metrics."""
    try:
        state = container.organism

        # Get recent judgments from state manager
        recent_judgments = state.state.get_recent_judgments(limit=100)

        pending_count = sum(1 for j in recent_judgments if j.verdict == "PENDING")
        completed_count = sum(1 for j in recent_judgments if j.verdict != "PENDING")
        failed_count = sum(1 for j in recent_judgments if j.verdict == "BARK")

        return HealthEventsResponse(
            status="alive",
            event_handlers={
                "total_groups": len(state.dogs),
                "total_handlers": len(state.dogs) * 3,
            },
            judgment_pipeline={
                "pending_judgments": pending_count,
                "completed_judgments": completed_count,
                "failed_judgments": failed_count,
                "total_capacity": 89,
            },
            timestamp=time.time(),
        )
    except Exception as e:
        logger.error("Error in /health/events: %s", e, exc_info=True)
        return HealthEventsResponse(
            status="degraded",
            event_handlers={},
            judgment_pipeline={},
            timestamp=time.time(),
        )


# ==============================================================================
# GET /health/full
# ==============================================================================

@router_health.get("/health/full", response_model=HealthFullResponse)
async def health_full(container: AppContainer = Depends(get_app_container)) -> HealthFullResponse:
    """Comprehensive system health for Claude Code."""
    try:
        state = container.organism

        # Database
        _db_status = "unhealthy"
        try:
            from cynic.kernel.core.storage.surreal import get_storage as _get_storage
            _get_storage()
            _db_status = "healthy"
        except Exception as _e:
        logger.debug(f'Silenced: {_e}')

        # LLM
        _llm_status = "unhealthy"
        try:
            from cynic.kernel.organism.brain.llm.adapter import get_registry
            if get_registry().get_available():
                _llm_status = "healthy"
        except Exception as _e:
        logger.debug(f'Silenced: {_e}')

        # Dogs
        dogs_list = []
        for dog_id, dog in state.dogs.items():
            stats = dog.stats() if hasattr(dog, "stats") else {}
            dogs_list.append({
                "name": dog_id,
                "status": "active",
                "judgments": stats.get("judgments_total", 0),
                "errors": stats.get("errors_total", 0),
                "error_rate": 0.0,
                "avg_latency_ms": 0.0,
                "priority": 1.0,
            })

        # Resources
        res = {"note": "psutil not available"}
        if PSUTIL_AVAILABLE:
            p = psutil.Process()
            res = {
                "memory_mb": round(p.memory_info().rss / (1024 * 1024), 1),
                "cpu_percent": round(p.cpu_percent(interval=0.1), 1),
                "num_threads": p.num_threads(),
            }

        learn_stats = state.cognition.qtable.stats()

        return HealthFullResponse(
            timestamp=time.time(),
            status="healthy" if _db_status == "healthy" else "degraded",
            uptime_seconds=round(state.uptime_s, 1),
            components={
                "database": {"status": _db_status},
                "llm": {"status": _llm_status},
                "consciousness": {"status": "healthy"},
            },
            dogs={
                "active_count": len(state.dogs),
                "total_count": len(state.dogs),
                "dogs": dogs_list,
            },
            learning={
                "active": state.cognition.learning_loop._active,
                "q_table_size": learn_stats.get("states", 0),
            },
            resources=res,
        )
    except Exception as e:
        logger.error("Error in /health/full: %s", e, exc_info=True)
        raise HTTPException(status_code=500, detail=str(e))


# ==============================================================================
# GET /health/ready
# ==============================================================================

@router_health.get("/health/ready", response_model=HealthReadyResponse)
async def health_ready(
    timeout: int = 30,
    container: AppContainer = Depends(get_app_container),
) -> HealthReadyResponse:
    """Blocking health check."""
    start_time = time.time()
    poll_interval = 0.5
    max_polls = int(timeout / poll_interval)

    for _ in range(max_polls):
        state = container.organism
        db_ok = True
        try:
            from cynic.kernel.core.storage.surreal import get_storage as _get_storage
            _get_storage()
        except Exception:
            db_ok = False

        llm_ok = False
        try:
            from cynic.kernel.organism.brain.llm.adapter import get_registry
            if get_registry().get_available():
                llm_ok = True
        except Exception as _e:
        logger.debug(f'Silenced: {_e}')

        if db_ok and llm_ok:
            return HealthReadyResponse(
                status="ready",
                waited_seconds=round(time.time() - start_time, 2),
                ready_components={"database": db_ok, "llm": llm_ok},
                timestamp=time.time(),
            )
        await asyncio.sleep(poll_interval)

    raise HTTPException(status_code=503, detail="Timeout waiting for readiness")
