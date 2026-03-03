"""
CYNIC introspection router - self-reflection & diagnostics.
"""
from __future__ import annotations

import logging
import time
from typing import Any

from fastapi import APIRouter, Depends

from cynic.interfaces.api.models import SelfAwarenessResponse
from cynic.interfaces.api.state import AppContainer, get_app_container

logger = logging.getLogger("cynic.interfaces.api.server")

router_introspection = APIRouter(tags=["introspection"])


@router_introspection.get("/introspect", response_model=SelfAwarenessResponse)
async def introspect(container: AppContainer = Depends(get_app_container)) -> SelfAwarenessResponse:
    """
    MetaCognition - CYNIC reads its own cognitive state.
    """
    state = container.organism
    
    return SelfAwarenessResponse(
        self_identity=state.identity.to_dict() if hasattr(state, "identity") else {"name": "CYNIC"},
        self_assessment={
            "kernel_integrity": 1.0,
            "verdict": "HOWL",
            "uptime_s": round(state.uptime_s, 1),
            "judgments": state.cognition.orchestrator.stats() if state.cognition.orchestrator else {},
        },
        timestamp=time.time(),
    )


@router_introspection.get("/axioms")
async def axioms(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """Emergent Axiom dashboard."""
    return {"status": "nominal", "axioms": ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"]}


@router_introspection.get("/lod")
async def lod(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """Survival LOD status."""
    state = container.organism
    if state.metabolism.scheduler:
        return state.metabolism.scheduler.stats().get("lod", {})
    return {"level": 0, "name": "FULL"}


@router_introspection.get("/mirror")
async def mirror(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """Ring 3 unified self-reflection snapshot."""
    state = container.organism
    return {
        "timestamp": time.time(),
        "instance_id": state.instance_id,
        "health_score": 100.0,
    }
