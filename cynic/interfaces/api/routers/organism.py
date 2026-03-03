"""
CYNIC organism state endpoints - observation and introspection.
"""
from __future__ import annotations

import logging
import time

from fastapi import APIRouter, Depends, HTTPException

from cynic.interfaces.api.models import (
    ConsciousnessResponse,
    DogsResponse,
    DogStatus,
    StateSnapshotResponse,
    AccountResponse as AccountStatusResponse,
)
from cynic.interfaces.api.state import AppContainer, get_app_container

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/api/organism", tags=["organism"])


@router.get("/state/snapshot", response_model=StateSnapshotResponse)
async def get_organism_state_snapshot(
    container: AppContainer = Depends(get_app_container),
) -> StateSnapshotResponse:
    """Full organism state snapshot."""
    try:
        organism = container.organism
        current_time = time.time()

        # Get consciousness level from scheduler
        consciousness_level = "REFLEX"  # default
        if organism.metabolism.scheduler is not None:
            current_lod = organism.metabolism.scheduler.current_lod
            if current_lod >= 3:
                consciousness_level = "META"
            elif current_lod >= 2:
                consciousness_level = "MACRO"
            elif current_lod >= 1:
                consciousness_level = "MICRO"
            else:
                consciousness_level = "REFLEX"

        judgment_count = len(organism.state.get_recent_judgments(limit=100))
        dog_count = len(organism.dogs)
        qtable_entries = organism.cognition.qtable.stats()["states"] if organism.cognition.qtable else 0
        pending_actions_count = 0 # Placeholder

        return StateSnapshotResponse(
            timestamp=current_time,
            consciousness_level=consciousness_level,
            judgment_count=judgment_count,
            dog_count=dog_count,
            qtable_entries=qtable_entries,
            residuals_count=0,
            pending_actions_count=pending_actions_count,
        )

    except Exception as exc:
        logger.exception("Error getting organism state snapshot: %s", exc)
            raise HTTPException(status_code=500, detail=str(exc))


@router.get("/consciousness", response_model=ConsciousnessResponse)
async def get_organism_consciousness(
    container: AppContainer = Depends(get_app_container),
) -> ConsciousnessResponse:
    """Current consciousness level."""
    organism = container.organism
    level = "REFLEX"
    if organism.metabolism.scheduler:
        lod = organism.metabolism.scheduler.current_lod
        level = ["REFLEX", "MICRO", "MACRO", "META"][min(lod, 3)]
    
    return ConsciousnessResponse(level=level)


@router.get("/dogs", response_model=DogsResponse)
async def get_organism_dogs(
    container: AppContainer = Depends(get_app_container),
) -> DogsResponse:
    """All dogs and their status."""
    organism = container.organism
    dogs_res = {}
    for dog_id, dog in organism.dogs.items():
        stats = dog.stats() if hasattr(dog, "stats") else {}
        dogs_res[dog_id] = DogStatus(
            q_score=stats.get("avg_q_score", 50.0),
            verdict=stats.get("last_verdict", "WAG"),
            confidence=stats.get("avg_confidence", 0.0),
            activity="active"
        )
    return DogsResponse(dogs=dogs_res, count=len(dogs_res))


@router.get("/account", response_model=AccountStatusResponse)
async def get_organism_account(
    container: AppContainer = Depends(get_app_container),
) -> AccountStatusResponse:
    """Account and budget status."""
    organism = container.organism
    # Placeholder mapping
    return AccountStatusResponse(
        limit_usd=10.0,
        spent_usd=0.0,
        remaining_usd=10.0,
        usage_pct=0.0
    )
