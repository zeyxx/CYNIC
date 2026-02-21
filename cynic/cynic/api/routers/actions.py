"""
CYNIC actions router — proposed-actions · self-probes
"""
from __future__ import annotations

import logging
from typing import Any


from fastapi import APIRouter, HTTPException, Query

from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import ActRequestedPayload, AxiomActivatedPayload
from cynic.learning.qlearning import LearningSignal
from cynic.api.state import get_state
from cynic.api.routers.utils import _append_social_signal

logger = logging.getLogger("cynic.api.server")

router_actions = APIRouter(tags=["actions"])


# ════════════════════════════════════════════════════════════════════════════
# GET /actions — list proposed actions
# POST /actions/{id}/accept — approve a proposed action
# POST /actions/{id}/reject — decline a proposed action
# ════════════════════════════════════════════════════════════════════════════

@router_actions.get("/actions")
async def list_actions(
    status: Optional[str] = Query(default=None, description="Filter by status: PENDING/ACCEPTED/REJECTED/AUTO_EXECUTED"),
) -> dict[str, Any]:
    """
    List proposed actions from the ActionProposer queue.

    These are the concrete actions CYNIC wants to take after BARK/GROWL judgments.
    Sorted by priority (1=critical first), then by proposed_at.

    status=PENDING (default)    → actions awaiting human decision
    status=ACCEPTED             → approved actions
    status=REJECTED             → declined actions
    status=AUTO_EXECUTED        → automatically executed by runner
    status=all                  → full queue
    """
    state = get_state()
    proposer = state.action_proposer

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
async def accept_action(action_id: str) -> dict[str, Any]:
    """
    Accept a proposed action — marks it ACCEPTED and signals ANTIFRAGILITY axiom.

    After accepting, the human (or another component) can execute the prompt.
    CYNIC logs the acceptance and uses it to reinforce the Q-Table next cycle.
    """
    state = get_state()
    action = state.action_proposer.accept(action_id)
    if action is None:
        raise HTTPException(status_code=404, detail=f"Action {action_id} not found")

    # ANTIFRAGILITY axiom: human×machine co-decision = adaptive strength
    try:
        new_state = state.axiom_monitor.signal("ANTIFRAGILITY")
        if new_state == "ACTIVE":
            await get_core_bus().emit(Event.typed(
                CoreEvent.AXIOM_ACTIVATED,
                AxiomActivatedPayload(axiom="ANTIFRAGILITY", maturity=state.axiom_monitor.get_maturity("ANTIFRAGILITY"), trigger="action_accept"),
                source="action_accept",
            ))
    except Exception:
        pass

    # ── L1 closure: accepted → fire ACT_REQUESTED → runner executes ──────
    # This closes the Machine→Actions loop: accept = authorize execution.
    if action.prompt:
        await get_core_bus().emit(Event.typed(
            CoreEvent.ACT_REQUESTED,
            ActRequestedPayload(
                action=action.prompt,
                action_id=action.action_id,
            ),
            source="action_accept",
        ))
        logger.info("*ears perk* Action %s → ACT_REQUESTED fired (L1 auto-execute)", action_id)

    # Social loop: accept = positive human×machine interaction
    _append_social_signal(
        source="cynic_interaction",
        sentiment=0.5,
        volume=30.0,
        topic=action.action_type or "action",
        signal_type="accept",
    )

    logger.info("*tail wag* Action %s ACCEPTED by human", action_id)
    return {"accepted": True, "action": action.to_dict(), "executing": bool(action.prompt)}


@router_actions.post("/actions/{action_id}/reject")
async def reject_action(action_id: str) -> dict[str, Any]:
    """
    Reject a proposed action — marks it REJECTED.

    CYNIC learns from rejections: the next Q-Table update for this state_key
    will have a lower reward signal (indirect — via the /feedback loop).
    """
    state = get_state()
    action = state.action_proposer.reject(action_id)
    if action is None:
        raise HTTPException(status_code=404, detail=f"Action {action_id} not found")

    # ── L1 closure: rejection → negative QTable signal ───────────────────
    # Rejection = human says "this decision was wrong" — feed it back.
    if action.state_key:
        state.qtable.update(LearningSignal(
            state_key=action.state_key,
            action=action.verdict,
            reward=0.10,  # low reward (near floor) = bad decision signal
            judgment_id=action.judgment_id,
            loop_name="ACTION_REJECTED",
        ))
        logger.info(
            "*head tilt* Action %s REJECTED → Q[%s][%s] penalized",
            action_id, action.state_key, action.verdict,
        )

    # Social loop: reject = negative interaction (still valuable — CYNIC learns)
    _append_social_signal(
        source="cynic_interaction",
        sentiment=-0.3,
        volume=20.0,
        topic=action.action_type or "action",
        signal_type="reject",
    )

    logger.info("*head tilt* Action %s REJECTED by human", action_id)
    return {"rejected": True, "action": action.to_dict()}


# ════════════════════════════════════════════════════════════════════════════
# GET  /self-probes          — list self-improvement proposals (L4)
# POST /self-probes/analyze  — trigger manual analysis
# POST /self-probes/{probe_id}/dismiss
# POST /self-probes/{probe_id}/apply
# ════════════════════════════════════════════════════════════════════════════

@router_actions.get("/self-probes")
async def list_self_probes(
    status: Optional[str] = Query(default=None, description="Filter by status: PENDING/APPLIED/DISMISSED/all"),
) -> dict[str, Any]:
    """
    List SelfProber proposals — CYNIC's analysis of its own performance gaps.

    L4 CYNIC→CYNIC self-improvement loop. Proposals are generated when
    ResidualDetector detects SPIKE/RISING/STABLE_HIGH patterns and SelfProber
    analyzes QTable, EScore, and Config recommendations.

    status=PENDING (default) → proposals awaiting review
    status=all               → full history
    """
    state = get_state()
    prober = state.self_prober

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
) -> dict[str, Any]:
    """
    Trigger a manual self-analysis run.

    Useful for testing or when you want CYNIC to introspect on demand.
    Returns newly generated proposals.
    """
    state = get_state()
    prober = state.self_prober
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
async def dismiss_probe(probe_id: str) -> dict[str, Any]:
    """Dismiss a self-improvement proposal — marks it DISMISSED."""
    state = get_state()
    proposal = state.self_prober.dismiss(probe_id)
    if proposal is None:
        raise HTTPException(status_code=404, detail=f"Probe {probe_id} not found")
    return {"dismissed": True, "proposal": proposal.to_dict()}


@router_actions.post("/self-probes/{probe_id}/apply")
async def apply_probe(probe_id: str) -> dict[str, Any]:
    """Mark a self-improvement proposal as APPLIED."""
    state = get_state()
    proposal = state.self_prober.apply(probe_id)
    if proposal is None:
        raise HTTPException(status_code=404, detail=f"Probe {probe_id} not found")
    logger.info("*tail wag* Self-probe %s APPLIED", probe_id)
    return {"applied": True, "proposal": proposal.to_dict()}
