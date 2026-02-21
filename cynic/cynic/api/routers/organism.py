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
from cynic.api.models.organism_state import (
    StateSnapshotResponse,
    ConsciousnessResponse,
    DogsResponse,
    DogStatus,
    ActionsResponse,
    ProposedAction,
    AccountStatusResponse,
)

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


@router.get("/consciousness", response_model=ConsciousnessResponse)
async def get_organism_consciousness(
    container: AppContainer = Depends(get_app_container),
) -> ConsciousnessResponse:
    """
    GET /api/organism/consciousness — Current consciousness level.

    Returns the current consciousness level (REFLEX|MICRO|MACRO|META)
    inferred from the organism's metabolic scheduler.

    Response is ConsciousnessResponse (frozen, immutable).
    """
    try:
        organism = container.organism

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

        return ConsciousnessResponse(level=consciousness_level)

    except Exception as exc:
        logger.exception("Error getting consciousness level: %s", exc)
        raise HTTPException(
            status_code=500,
            detail=f"Failed to get consciousness: {str(exc)}",
        )


@router.get("/dogs", response_model=DogsResponse)
async def get_organism_dogs(
    container: AppContainer = Depends(get_app_container),
) -> DogsResponse:
    """
    GET /api/organism/dogs — All dogs and their status.

    Returns the current status of all dogs in the organism's orchestrator,
    including their last verdict, Q-score, confidence, and activity.

    Response is DogsResponse (frozen, immutable).
    """
    try:
        organism = container.organism

        # Get dogs from orchestrator
        dogs_response: dict[str, DogStatus] = {}
        if organism.cognition.orchestrator and organism.cognition.orchestrator.dogs:
            for dog_id, dog in organism.cognition.orchestrator.dogs.items():
                # Extract values with defaults, ensuring types are correct
                q_score = getattr(dog, "q_score", 50.0)
                if not isinstance(q_score, (int, float)):
                    q_score = 50.0

                verdict = getattr(dog, "verdict", "WAG")
                if not isinstance(verdict, str):
                    verdict = "WAG"

                confidence = getattr(dog, "confidence", None)
                if confidence is not None and not isinstance(confidence, (int, float)):
                    confidence = None

                activity = getattr(dog, "activity", None)
                if activity is not None and not isinstance(activity, str):
                    activity = None

                dogs_response[dog_id] = DogStatus(
                    q_score=q_score,
                    verdict=verdict,
                    confidence=confidence,
                    activity=activity,
                )

        return DogsResponse(dogs=dogs_response, count=len(dogs_response))

    except Exception as exc:
        logger.exception("Error getting dogs: %s", exc)
        raise HTTPException(
            status_code=500,
            detail=f"Failed to get dogs: {str(exc)}",
        )


@router.get("/actions", response_model=ActionsResponse)
async def get_organism_actions(
    container: AppContainer = Depends(get_app_container),
) -> ActionsResponse:
    """
    GET /api/organism/actions — Pending proposed actions.

    Returns the list of pending proposed actions from the organism's
    memory action proposer, including action ID, type, priority, and description.

    Response is ActionsResponse (frozen, immutable).
    """
    try:
        organism = container.organism

        # Get pending actions
        actions: list[ProposedAction] = []
        if organism.memory.action_proposer:
            pending_list = organism.memory.action_proposer.pending()
            if pending_list:
                for action in pending_list:
                    actions.append(ProposedAction(
                        action_id=action.get("action_id", "unknown"),
                        action_type=action.get("action_type", "MONITOR"),
                        priority=action.get("priority", 3),
                        description=action.get("description"),
                    ))

        return ActionsResponse(actions=actions, count=len(actions))

    except Exception as exc:
        logger.exception("Error getting actions: %s", exc)
        raise HTTPException(
            status_code=500,
            detail=f"Failed to get actions: {str(exc)}",
        )


@router.get("/account", response_model=AccountStatusResponse)
async def get_organism_account(
    container: AppContainer = Depends(get_app_container),
) -> AccountStatusResponse:
    """
    GET /api/organism/account — Account and budget status.

    Returns the organism's account metrics including:
    - Current session budget (balance_usd)
    - Total cumulative spend (spent_usd)
    - Budget remaining (budget_remaining_usd)
    - Learning rate [0, φ⁻¹=0.618]
    - Reputation score [0, 100]

    Response is AccountStatusResponse (frozen, immutable).
    """
    try:
        organism = container.organism
        current_time = time.time()

        # Get account stats from the account_agent
        account_stats = {}
        if organism.metabolic.account_agent:
            account_stats = organism.metabolic.account_agent.stats()

        # Extract fields with defaults
        balance_usd = float(account_stats.get("session_budget_usd", 10.0))
        spent_usd = float(account_stats.get("total_cost_usd", 0.0))
        budget_remaining_usd = float(account_stats.get("budget_remaining_usd", balance_usd - spent_usd))

        # Learn rate: fixed φ⁻¹ = 0.618 for now (can be dynamic later)
        learn_rate = 0.618

        # Reputation: compute from E-Score tracker if available
        # For now, default to 50.0 (neutral) if not available
        reputation = 50.0
        if organism.cognition.escore_tracker:
            # Get organism's own E-Score
            try:
                escore = await organism.cognition.escore_tracker.get("agent:cynic")
                if escore and hasattr(escore, "total"):
                    reputation = float(escore.total)
            except Exception:
                pass  # E-Score not available, use default

        return AccountStatusResponse(
            timestamp=current_time,
            balance_usd=balance_usd,
            spent_usd=spent_usd,
            budget_remaining_usd=budget_remaining_usd,
            learn_rate=learn_rate,
            reputation=reputation,
        )

    except Exception as exc:
        logger.exception("Error getting account status: %s", exc)
        raise HTTPException(
            status_code=500,
            detail=f"Failed to get account status: {str(exc)}",
        )
