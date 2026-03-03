"""
CYNIC core router " judge  perceive  learn  feedback  policy
"""

from __future__ import annotations

import logging
import uuid

from fastapi import APIRouter, HTTPException, Depends

from cynic.interfaces.api.models import (
    FeedbackRequest,
    FeedbackResponse,
    JudgeRequest,
    JudgeResponse,
    LearnRequest,
    LearnResponse,
    PerceiveRequest,
    PerceiveResponse,
    PolicyResponse,
)
from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.kernel.core.event_bus import CoreEvent, Event
from cynic.kernel.core.events_schema import (
    PerceptionReceivedPayload,
)
from cynic.kernel.core.judgment import Cell

logger = logging.getLogger("cynic.interfaces.api.server")

router_core = APIRouter(tags=["core"])

#
# POST /judge
#


@router_core.post("/judge", response_model=JudgeResponse)
async def judge(
    req: JudgeRequest, container: AppContainer = Depends(get_app_container)
) -> JudgeResponse:
    """
    Run the full CYNIC judgment pipeline on any content.
    Event-first API " returns PENDING immediately.
    """
    state = container.organism

    # Build Cell
    history_ctx = state.senses.context_compressor.get_compressed_context(budget=400)
    enriched_context = req.context or ""
    if history_ctx:
        enriched_context = (
            f"{enriched_context}\n[Session history]\n{history_ctx}".strip()
        )

    from cynic.kernel.core.judgment import infer_time_dim

    time_dim = req.time_dim or infer_time_dim(
        str(req.content), enriched_context, req.analysis
    )

    cell = Cell(
        reality=req.reality,
        analysis=req.analysis,
        time_dim=time_dim,
        content=req.content,
        context=enriched_context,
        lod=req.lod,
        budget_usd=req.budget_usd,
    )

    judgment_id = str(uuid.uuid4())

    # Emit JUDGMENT_REQUESTED
    from cynic.kernel.core.events_schema import JudgmentRequestedPayload

    try:
        await container.organism.cognition.orchestrator.bus.emit(
            Event.typed(
                CoreEvent.JUDGMENT_REQUESTED,
                JudgmentRequestedPayload(
                    cell_id=cell.cell_id,
                    reality=cell.reality,
                    level=req.level or "",
                    fractal_depth=getattr(req, "fractal_depth", 1),
                    cell=cell.model_dump(),
                    source="api:judge",
                    judgment_id=judgment_id,
                ),
                source="api:judge",
            )
        )
    except Exception as exc:
        logger.warning("JUDGMENT_REQUESTED emission failed: %s", exc)

    # Record PENDING
    try:
        await state.memory.state.record_pending_judgment(judgment_id)
    except Exception as exc:
        logger.warning("record_pending_judgment failed: %s", exc)

    return JudgeResponse(
        judgment_id=judgment_id,
        q_score=0.0,
        verdict="PENDING",
        confidence=0.0,
        axiom_scores={},
        dog_votes={},
        consensus_reached=False,
        consensus_votes=0,
        cost_usd=0.0,
        llm_calls=0,
        duration_ms=0.0,
        level_used=req.level or "AUTO",
    )


#
# POST /perceive
#


@router_core.post("/perceive", response_model=PerceiveResponse)
async def perceive(
    req: PerceiveRequest, container: AppContainer = Depends(get_app_container)
) -> PerceiveResponse:
    """Receive raw perception and trigger judgment."""
    state = container.organism
    judgment_id = str(uuid.uuid4())

    try:
        await container.organism.cognition.orchestrator.bus.emit(
            Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                PerceptionReceivedPayload(
                    content=str(req.data),
                    source=req.source,
                    reality=req.reality,
                    context=req.context,
                    run_judgment=req.run_judgment,
                    judgment_id=judgment_id,
                ),
                source=f"api:perceive:{req.source}",
            )
        )

        if req.run_judgment:
            await state.memory.state.record_pending_judgment(judgment_id)

        return PerceiveResponse(
            cell_id=judgment_id,
            source=req.source,
            reality=req.reality,
            enqueued=True,
            message="Perception enqueued for processing",
        )
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


#
# GET /judge/{id} " Polling
#


@router_core.get("/judge/{judgment_id}")
async def get_judgment(
    judgment_id: str, container: AppContainer = Depends(get_app_container)
) -> dict:
    """Query status/result of a judgment."""
    state = container.organism

    # 1. Check if completed
    recent = await state.memory.state.get_recent_judgments(limit=50)
    for j in recent:
        # Support both real objects and mocks/dicts
        jid = getattr(j, "judgment_id", None) or (
            j.get("judgment_id") if isinstance(j, dict) else None
        )
        if jid == judgment_id:
            res = (
                j.to_dict()
                if hasattr(j, "to_dict")
                else (j if isinstance(j, dict) else vars(j))
            )
            # Convert any mappingproxy to dict for Pydantic serialization
            res = {
                k: (dict(v) if "mappingproxy" in str(type(v)) else v)
                for k, v in res.items()
            }
            res["status"] = "COMPLETED"
            return res

    # 2. Check if pending (via status key)
    status = await state.memory.state.get_judgment_status(judgment_id)
    return {"status": status, "judgment_id": judgment_id, "verdict": status}


#
# OTHER CORE ENDPOINTS
#


@router_core.post("/learn", response_model=LearnResponse)
async def learn(
    req: LearnRequest, container: AppContainer = Depends(get_app_container)
) -> LearnResponse:
    await container.organism.cognition.orchestrator.bus.emit(
        Event.typed(
            CoreEvent.LEARNING_EVENT,
            {"state_key": req.state_key, "action": req.action, "reward": req.reward},
            source="api:learn",
        )
    )
    return LearnResponse(
        state_key=req.state_key,
        action=req.action,
        q_value=0.5,
        visits=1,
        confidence=0.0,
        wins=0,
        losses=0,
    )


@router_core.post("/feedback", response_model=FeedbackResponse)
async def feedback(req: FeedbackRequest) -> FeedbackResponse:
    return FeedbackResponse(
        state_key="unknown",
        action="WAG",
        reward=float(req.rating) / 5.0,
        q_value=0.5,
        visits=1,
        message="Feedback received",
    )


@router_core.get("/policy/{state_key}", response_model=PolicyResponse)
async def get_policy(
    state_key: str, container: AppContainer = Depends(get_app_container)
) -> PolicyResponse:
    return PolicyResponse(
        state_key=state_key,
        mode="exploit",
        recommended_action="WAG",
        q_value=0.5,
        confidence=0.0,
        top_actions=[],
    )
