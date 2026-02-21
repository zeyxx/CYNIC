"""
CYNIC organism state endpoints — observation and introspection.

These endpoints expose read-only organism state for external monitoring.
All responses are immutable (frozen Pydantic models).

Endpoints:
  GET /api/organism/state/snapshot     → Full organism state snapshot
  GET /api/organism/consciousness      → Current consciousness level
  GET /api/organism/dogs               → All dogs and their status
  GET /api/organism/actions            → Pending proposed actions
"""
from __future__ import annotations

import logging
import time
from fastapi import APIRouter, Depends, HTTPException

from cynic.api.state import get_app_container, AppContainer
from cynic.api.models.organism_state import StateSnapshotResponse

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/api/organism", tags=["organism"])


@router.get("/state/snapshot", response_model=StateSnapshotResponse)
async def get_organism_state_snapshot(
    container: AppContainer = Depends(get_app_container),
) -> StateSnapshotResponse:
    """
    GET /api/organism/state/snapshot — Full organism state snapshot.

    Returns the complete state of the CYNIC organism including:
    - Current consciousness level (REFLEX/MICRO/MACRO/META)
    - Judgment count (recent window)
    - Number of active dogs
    - Q-Table entry count
    - Active residual detections
    - Pending proposed actions

    Response is StateSnapshotResponse (frozen, immutable).
    """
    try:
        organism = container.organism
        current_time = time.time()

        # Get consciousness level from scheduler
        consciousness_level = "REFLEX"  # default
        if organism.metabolic.scheduler is not None:
            current_lod = organism.metabolic.scheduler.current_lod
            # Map LOD to consciousness level
            if current_lod >= 3:
                consciousness_level = "META"
            elif current_lod >= 2:
                consciousness_level = "MACRO"
            elif current_lod >= 1:
                consciousness_level = "MICRO"
            else:
                consciousness_level = "REFLEX"

        # Get judgment count (use length of recent window or q-table size as proxy)
        judgment_count = len(organism.cognition.qtable._q_table) if organism.cognition.qtable else 0

        # Get dog count
        dog_count = len(organism.cognition.orchestrator.dogs) if organism.cognition.orchestrator else 0

        # Get Q-Table entries
        qtable_entries = len(organism.cognition.qtable._q_table) if organism.cognition.qtable else 0

        # Get residuals count
        residuals_count = len(organism.cognition.residual_detector._residuals) if organism.cognition.residual_detector else 0

        # Get pending actions
        pending_actions_count = len(organism.memory.action_proposer.pending()) if organism.memory.action_proposer else 0

        return StateSnapshotResponse(
            timestamp=current_time,
            consciousness_level=consciousness_level,
            judgment_count=judgment_count,
            dog_count=dog_count,
            qtable_entries=qtable_entries,
            residuals_count=residuals_count,
            pending_actions_count=pending_actions_count,
        )

    except Exception as exc:
        logger.exception("Error getting organism state snapshot: %s", exc)
        raise HTTPException(
            status_code=500,
            detail=f"Failed to get organism state: {str(exc)}",
        )
