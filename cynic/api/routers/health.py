"""
CYNIC health router — core vitals: health · stats
"""
from __future__ import annotations

import json
import logging
import os
import pathlib as _pathlib
import time
from typing import Any

from fastapi import APIRouter, Depends, HTTPException, Request
from pydantic import ValidationError

from cynic.core.consciousness import get_consciousness
from cynic.core.phi import PHI
from cynic.api.models import HealthResponse, StatsResponse
from cynic.api.state import get_app_container, AppContainer

logger = logging.getLogger("cynic.api.server")

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
        "status": "alive",
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
        from cynic.core.storage.surreal import get_storage as _get_storage  # noqa: deferred
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
        llm_adapters=[a.adapter_id for a in __import__("cynic.llm.adapter", fromlist=["get_registry"]).get_registry().get_available()],
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
