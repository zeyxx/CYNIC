"""
CYNIC actions router — proposed-actions · self-probes
"""
from __future__ import annotations

import logging
from typing import Any

from fastapi import APIRouter, Depends, HTTPException, Query

from cynic.interfaces.api.routers.utils import _append_social_signal
from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.kernel.core.event_bus import CoreEvent, Event
from cynic.kernel.core.events_schema import ActRequestedPayload, AxiomActivatedPayload
from cynic.kernel.organism.brain.learning.qlearning import LearningSignal

logger = logging.getLogger("cynic.interfaces.api.server")

router_actions = APIRouter(tags=["actions"])


@router_actions.get("/actions")
async def list_actions(
    status: str | None = Query(default=None, description="Filter by status: PENDING/ACCEPTED/REJECTED/AUTO_EXECUTED"),
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    List proposed actions from the ActionProposer queue.
    """
    org = container.organism
    proposer = org.memory.action_proposer

    if proposer is None:
        return {"actions": [], "count": 0, "stats": {}, "message": "ActionProposer not active"}

    if status is None or status == "PENDING":
        actions = proposer.pending()
    elif status == "all":
        actions = proposer.all_actions()
    else:
        actions = [a for a in proposer.all_actions() if a.status == status.upper()]

    return {
        "actions": [a.to_dict() for a in actions],
        "count": len(actions),
        "stats": proposer.stats(),
    }


@router_actions.post("/actions/{action_id}/accept")
async def accept_action(
    action_id: str,
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    Accept a proposed action — marks it ACCEPTED and signals ANTIFRAGILITY axiom.
    """
    org = container.organism
    proposer = org.memory.action_proposer
    if proposer is None:
        raise HTTPException(status_code=503, detail="ActionProposer not active")

    action = proposer.accept(action_id)
    if action is None:
        raise HTTPException(status_code=404, detail=f"Action {action_id} not found")

    bus = org.cognition.orchestrator.bus

    # ANTIFRAGILITY axiom
    if org.cognition.axiom_monitor:
        new_state = org.cognition.axiom_monitor.signal("ANTIFRAGILITY")
        if new_state == "ACTIVE":
            await bus.emit(Event.typed(
                CoreEvent.AXIOM_ACTIVATED,
                AxiomActivatedPayload(
                    axiom="ANTIFRAGILITY", 
                    maturity=org.cognition.axiom_monitor.get_maturity("ANTIFRAGILITY"), 
                    trigger="action_accept"
                ),
                source="action_accept",
            ))

    # L1 closure: fire ACT_REQUESTED
    if action.prompt:
        await bus.emit(Event.typed(
            CoreEvent.ACT_REQUESTED,
            ActRequestedPayload(
                action=action.prompt,
                action_id=action.action_id,
            ),
            source="action_accept",
        ))
        logger.info("*ears perk* Action %s → ACT_REQUESTED fired", action_id)

    # Social loop
    _append_social_signal(
        source="cynic_interaction",
        sentiment=0.5,
        topic=action.action_type or "action",
        signal_type="accept",
    )

    logger.info("*tail wag* Action %s ACCEPTED", action_id)
    return {"accepted": True, "action": action.to_dict(), "executing": bool(action.prompt)}


@router_actions.post("/actions/{action_id}/reject")
async def reject_action(
    action_id: str,
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    Reject a proposed action — marks it REJECTED.
    """
    org = container.organism
    proposer = org.memory.action_proposer
    if proposer is None:
        raise HTTPException(status_code=503, detail="ActionProposer not active")

    action = proposer.reject(action_id)
    if action is None:
        raise HTTPException(status_code=404, detail=f"Action {action_id} not found")

    if action.state_key:
        org.cognition.learning_loop.qtable.update(LearningSignal(
            state_key=action.state_key,
            action=action.verdict,
            reward=0.10,
            judgment_id=action.judgment_id,
            loop_name="ACTION_REJECTED",
        ))

    _append_social_signal(
        source="cynic_interaction",
        sentiment=-0.3,
        topic=action.action_type or "action",
        signal_type="reject",
    )

    logger.info("*head tilt* Action %s REJECTED", action_id)
    return {"rejected": True, "action": action.to_dict()}


@router_actions.get("/self-probes")
async def list_self_probes(
    status: str | None = Query(default=None, description="Filter by status: PENDING/APPLIED/DISMISSED/all"),
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    List SelfProber proposals — CYNIC's analysis of its own performance gaps.
    """
    org = container.organism
    prober = org.memory.self_prober

    if prober is None:
        return {"proposals": [], "count": 0, "stats": {}, "message": "SelfProber not active"}

    if status is None or status == "PENDING":
        proposals = prober.pending()
    elif status == "all":
        proposals = prober.all_proposals()
    else:
        proposals = [p for p in prober.all_proposals() if p.status == status.upper()]

    return {
        "proposals": [p.to_dict() for p in proposals],
        "count": len(proposals),
        "stats": prober.stats(),
    }


@router_actions.post("/self-probes/analyze")
async def trigger_self_analysis(
    pattern_type: str = Query(default="MANUAL"),
    severity: float = Query(default=0.5, ge=0.0, le=1.0),
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    Trigger a manual self-analysis run.
    """
    org = container.organism
    prober = org.memory.self_prober
    if prober is None:
        raise HTTPException(status_code=503, detail="SelfProber not active")

    new_proposals = prober.analyze(
        trigger="MANUAL",
        pattern_type=pattern_type,
        severity=severity,
    )
    return {
        "proposals": [p.to_dict() for p in new_proposals],
        "count": len(new_proposals),
        "stats": prober.stats(),
    }


@router_actions.post("/self-probes/{probe_id}/dismiss")
async def dismiss_probe(
    probe_id: str,
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """Dismiss a self-improvement proposal."""
    org = container.organism
    if org.memory.self_prober is None:
        raise HTTPException(status_code=503, detail="SelfProber not active")

    proposal = org.memory.self_prober.dismiss(probe_id)
    if proposal is None:
        raise HTTPException(status_code=404, detail=f"Probe {probe_id} not found")
    return {"dismissed": True, "proposal": proposal.to_dict()}


@router_actions.post("/self-probes/{probe_id}/apply")
async def apply_probe(
    probe_id: str,
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """Mark a self-improvement proposal as APPLIED."""
    org = container.organism
    if org.memory.self_prober is None:
        raise HTTPException(status_code=503, detail="SelfProber not active")

    proposal = org.memory.self_prober.apply(probe_id)
    if proposal is None:
        raise HTTPException(status_code=404, detail=f"Probe {probe_id} not found")
    logger.info("*tail wag* Self-probe %s APPLIED", probe_id)
    return {"applied": True, "proposal": proposal.to_dict()}
