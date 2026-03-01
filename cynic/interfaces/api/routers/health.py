"""
CYNIC health router — core vitals: health · stats
"""
from __future__ import annotations

import asyncio
import json
import logging
import os
import pathlib as _pathlib
import time
from typing import Any, Optional

try:
    import psutil
    PSUTIL_AVAILABLE = True
except ImportError:
    PSUTIL_AVAILABLE = False

from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import ValidationError

from cynic.kernel.core.consciousness import get_consciousness
from cynic.kernel.core.phi import PHI
from cynic.interfaces.api.models import HealthResponse, StatsResponse
from cynic.interfaces.api.state import get_app_container, AppContainer

logger = logging.getLogger("cynic.interfaces.api.server")

router_health = APIRouter(tags=["health"])

_CONSCIOUSNESS_FILE = os.path.join(os.path.expanduser("~"), ".cynic", "consciousness.json")


# ── Root route: API alive status ────────────────────────────────────────────
@router_health.get("/", include_in_schema=True)
async def root(request: Request) -> dict:
    """
    Root endpoint — CYNIC kernel is alive.

    Returns:
        - status: "alive" if all systems nominal
        - name: "CYNIC Kernel"
        - φ: The golden ratio (for identity)
        - routes: List of available API routes
    """
    # Collect available routes from the app
    app = request.app
    routes = []
    for route in app.routes:
        if hasattr(route, "path") and not route.path.startswith("/openapi") and not route.path.startswith("/docs") and not route.path.startswith("/redoc") and not route.path.startswith("/static"):
            routes.append(route.path)
    routes.sort()

    return {
        "status": "AWAKE",
        "name": "CYNIC Kernel",
        "φ": f"{PHI:.6f}",  # The golden ratio
        "routes": routes,
    }


# ════════════════════════════════════════════════════════════════════════════
# GET /health
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/health", response_model=HealthResponse)
async def health(container: AppContainer = Depends(get_app_container)) -> HealthResponse:
    """
    Kernel health — the organism's vital signs.

    status=alive    → all systems nominal
    status=degraded → partial functionality (e.g. no DB, no LLM)
    status=dead     → kernel not initialized (should never reach this route)
    """
    state = container.organism
    consciousness = get_consciousness()
    judge_stats = state.orchestrator.stats()
    learn_stats = state.qtable.stats()

    sched_stats = state.scheduler.stats()

    # Determine status
    status = "alive"
    if not state.learning_loop._active:
        status = "degraded"

    # T02: check SurrealDB singleton status (no I/O — just checks if initialized)
    _storage_status: dict[str, Any] = {}
    try:
        from cynic.kernel.core.storage.surreal import get_storage as _get_storage  # noqa: deferred
        _get_storage()  # raises RuntimeError if not initialized
        _storage_status["surreal"] = "connected"
    except RuntimeError:
        _storage_status["surreal"] = "disconnected"
    except ValidationError:
        _storage_status["surreal"] = "error"

    return HealthResponse(
        status=status,
        uptime_s=round(state.uptime_s, 1),
        consciousness=consciousness.to_dict(),
        dogs=state.dogs,
        learning={
            "active": state.learning_loop._active,
            "states": learn_stats["states"],
            "total_updates": learn_stats["total_updates"],
            "pending_flush": learn_stats["pending_flush"],
        },
        scheduler=sched_stats,
        llm_adapters=[a.adapter_id for a in __import__("cynic.kernel.organism.brain.llm.adapter", fromlist=["get_registry"]).get_registry().get_available()],
        judgments_total=judge_stats["judgments_total"],
        phi=PHI,
        storage=_storage_status,
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /stats
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/stats", response_model=StatsResponse)
async def stats(container: AppContainer = Depends(get_app_container)) -> StatsResponse:
    """Detailed kernel metrics — everything CYNIC knows about itself."""
    state = container.organism

    return StatsResponse(
        judgments=state.orchestrator.stats(),
        learning=state.qtable.stats(),
        top_states=state.qtable.top_states(n=10),
        consciousness=get_consciousness().to_dict(),
        compressor=state.context_compressor.stats(),
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /health/events (Track G: Event pipeline metrics)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/health/events")
async def health_events(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """Event handler pipeline health + metrics (Track G resilience).

    Returns:
        - status: "alive" or "degraded"
        - event_handlers: Handler group counts
        - judgment_pipeline: Pending/completed judgment counts
        - timestamp: Response timestamp
    """
    try:
        state = container.organism

        # Collect handler stats
        handler_stats = state._handler_registry.introspect()

        # Get conscious state metrics
        conscious_state = state.conscious_state
        recent_judgments = getattr(conscious_state, "_recent_judgments", [])

        pending_count = sum(1 for j in recent_judgments if j.verdict == "PENDING")
        completed_count = sum(1 for j in recent_judgments if j.verdict != "PENDING")
        failed_count = sum(1 for j in recent_judgments if j.verdict == "BARK")

        return {
            "status": "alive",
            "event_handlers": {
                "total_groups": handler_stats.get("total_handlers", 0),
                "total_handlers": handler_stats.get("total_deps", 0),
                "groups": handler_stats.get("groups", []),
            },
            "judgment_pipeline": {
                "pending_judgments": pending_count,
                "completed_judgments": completed_count,
                "failed_judgments": failed_count,
                "total_capacity": 89,  # F(11) = 89 recent judgments
            },
            "timestamp": time.time(),
        }
    except Exception as e:
        logger.error("Error in /health/events: %s", e, exc_info=True)
        return {
            "status": "degraded",
            "message": str(e),
            "timestamp": time.time(),
        }


# ════════════════════════════════════════════════════════════════════════════
# GET /health/full (NEW: Rich health data for Claude Code)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/health/full")
async def health_full(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    Comprehensive system health endpoint for Claude Code decision-making.

    Returns:
        - timestamp: Response timestamp
        - uptime_seconds: Kernel uptime
        - components: Database, LLM, event bus, consciousness status
        - dogs: Active Dogs with per-dog metrics
        - learning: Q-table size, loop status, feedback count, convergence
        - resources: Memory, CPU, connection counts
    """
    try:
        state = container.organism
        consciousness = get_consciousness()

        # Component status
        components = {}

        # Database
        try:
            from cynic.kernel.core.storage.surreal import get_storage as _get_storage
            _get_storage()
            components["database"] = {
                "status": "healthy",
                "type": "SurrealDB",
                "details": "Connected and operational",
            }
        except RuntimeError:
            components["database"] = {
                "status": "unhealthy",
                "type": "SurrealDB",
                "details": "Not initialized",
            }
        except Exception as e:
            components["database"] = {
                "status": "degraded",
                "type": "SurrealDB",
                "details": str(e),
            }

        # LLM adapter
        try:
            from cynic.kernel.organism.brain.llm.adapter import get_registry
            registry = get_registry()
            available = registry.get_available()
            components["llm"] = {
                "status": "healthy" if available else "unhealthy",
                "available_adapters": [a.adapter_id for a in available],
                "count": len(available),
            }
        except Exception as e:
            components["llm"] = {
                "status": "degraded",
                "details": str(e),
            }

        # Event bus
        try:
            event_handlers = state._handler_registry.introspect()
            components["event_bus"] = {
                "status": "healthy",
                "total_handlers": event_handlers.get("total_handlers", 0),
                "groups": len(event_handlers.get("groups", [])),
            }
        except Exception as e:
            components["event_bus"] = {
                "status": "degraded",
                "details": str(e),
            }

        # Consciousness
        components["consciousness"] = {
            "status": "healthy",
            "details": consciousness.to_dict(),
        }

        # Dogs status
        dogs_list = []
        try:
            for dog_id, dog in state.dogs.items():
                dog_stats = dog.stats()
                dogs_list.append({
                    "name": dog_id,
                    "status": "active" if dog_stats.get("active") else "inactive",
                    "judgments": dog_stats.get("judgments", 0),
                    "errors": dog_stats.get("errors", 0),
                    "error_rate": dog_stats.get("error_rate", 0.0),
                    "avg_latency_ms": dog_stats.get("avg_latency_ms", 0.0),
                    "priority": dog_stats.get("priority", 0.0),
                })
        except Exception as e:
            logger.warning("Error collecting Dogs status: %s", e)
            dogs_list = []

        # Learning metrics
        learning_stats = {}
        try:
            learn_stats = state.qtable.stats()
            learning_stats = {
                "active": state.learning_loop._active,
                "q_table_size": learn_stats.get("states", 0),
                "total_entries": learn_stats.get("entries", 0),
                "total_updates": learn_stats.get("total_updates", 0),
                "total_visits": learn_stats.get("total_visits", 0),
                "learning_rate": learn_stats.get("learning_rate", 0.0),
                "discount_factor": learn_stats.get("discount", 0.0),
                "pending_flush": learn_stats.get("pending_flush", 0),
                "ewc_consolidated": learn_stats.get("ewc_consolidated", 0),
                "convergence_rate": round(learn_stats.get("total_visits", 0) / max(1, learn_stats.get("total_updates", 1)), 3),
            }
        except Exception as e:
            logger.warning("Error collecting learning stats: %s", e)
            learning_stats = {"error": str(e)}

        # Resources (memory, CPU)
        resources = {}
        try:
            if PSUTIL_AVAILABLE:
                process = psutil.Process()
                resources = {
                    "memory_mb": round(process.memory_info().rss / (1024 * 1024), 1),
                    "cpu_percent": round(process.cpu_percent(interval=0.1), 1),
                    "open_files": len(process.open_files()),
                    "num_threads": process.num_threads(),
                }
            else:
                resources = {"note": "psutil not available"}
        except Exception as e:
            logger.warning("Error collecting resource metrics: %s", e)
            resources = {"error": str(e)}

        return {
            "timestamp": time.time(),
            "status": "healthy" if all(c.get("status") != "unhealthy" for c in components.values()) else "degraded",
            "uptime_seconds": round(state.uptime_s, 1),
            "components": components,
            "dogs": {
                "active_count": sum(1 for d in dogs_list if d["status"] == "active"),
                "total_count": len(dogs_list),
                "dogs": dogs_list,
            },
            "learning": learning_stats,
            "resources": resources,
        }

    except Exception as e:
        logger.error("Error in /health/full: %s", e, exc_info=True)
        return {
            "status": "degraded",
            "message": str(e),
            "timestamp": time.time(),
        }


# ════════════════════════════════════════════════════════════════════════════
# GET /health/ready (NEW: Blocking endpoint with timeout)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/health/ready")
async def health_ready(
    timeout: int = 30,
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    Blocking health check that waits for all systems to be ready.

    Args:
        timeout: Max seconds to wait for readiness (default 30)

    Returns:
        - status: "ready" or degraded
        - waited_seconds: How long we waited
        - ready_components: Which systems are ready
        - timestamp: Response timestamp

    Returns 503 if systems not ready after timeout.
    """
    start_time = time.time()
    poll_interval = 0.5
    max_polls = int(timeout / poll_interval)

    for poll_count in range(max_polls):
        try:
            state = container.organism

            # Check critical components
            db_ok = True
            try:
                from cynic.kernel.core.storage.surreal import get_storage as _get_storage
                _get_storage()
            except Exception:
                db_ok = False

            llm_ok = True
            try:
                from cynic.kernel.organism.brain.llm.adapter import get_registry
                registry = get_registry()
                available = registry.get_available()
                llm_ok = len(available) > 0
            except Exception:
                llm_ok = False

            dogs_ok = len(state.dogs) > 0
            learning_ok = state.learning_loop._active
            consciousness_ok = get_consciousness() is not None

            all_ready = db_ok and llm_ok and dogs_ok and consciousness_ok
            waited_s = round(time.time() - start_time, 2)

            if all_ready:
                return {
                    "status": "ready",
                    "waited_seconds": waited_s,
                    "ready_components": {
                        "database": db_ok,
                        "llm": llm_ok,
                        "dogs": dogs_ok,
                        "consciousness": consciousness_ok,
                        "learning": learning_ok,
                    },
                    "timestamp": time.time(),
                }

            # Not ready yet, wait and retry
            if poll_count < max_polls - 1:
                await asyncio.sleep(poll_interval)

        except Exception as e:
            logger.warning("Error checking readiness: %s", e)
            await asyncio.sleep(poll_interval)

    # Timeout reached
    waited_s = round(time.time() - start_time, 2)
    raise HTTPException(
        status_code=503,
        detail={
            "status": "not_ready",
            "waited_seconds": waited_s,
            "message": "Systems not ready after timeout",
            "timestamp": time.time(),
        },
    )
