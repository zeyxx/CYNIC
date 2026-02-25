"""
CYNIC core router — judge · perceive · learn · feedback · policy
"""
from __future__ import annotations

import asyncio
import httpx
import json
import logging
import os
import time
import uuid
from typing import Any


from fastapi import APIRouter, HTTPException, Query

from cynic.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.exceptions import EventBusError
from cynic.core.events_schema import (
    AxiomActivatedPayload,
    PerceptionReceivedPayload,
    UserCorrectionPayload,
    UserFeedbackPayload,
)
from cynic.core.judgment import Cell, Judgment
from cynic.core.phi import MAX_CONFIDENCE
from cynic.learning.qlearning import LearningSignal
from cynic.organism.brain.consensus import get_consensus_engine

from cynic.api.models import (
    JudgeRequest, JudgeResponse,
    PerceiveRequest, PerceiveResponse,
    LearnRequest, LearnResponse,
    FeedbackRequest, FeedbackResponse,
    PolicyResponse,
)
from cynic.api.state import get_state
from cynic.api.routers.utils import _append_social_signal

logger = logging.getLogger("cynic.api.server")

router_core = APIRouter(tags=["core"])

_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")

# Lazy-resolved judgment repo — avoids module-level storage import
_judgment_repo: Any = None


def _get_judgment_repo() -> Any:
    """Lazily import and cache JudgmentRepository (only when DB is available)."""
    global _judgment_repo
    if _judgment_repo is None:
        from cynic.core.storage.postgres import JudgmentRepository
        _judgment_repo = JudgmentRepository()
    return _judgment_repo


def _persist_judgment(judgment: Judgment) -> None:
    """
    Fire-and-forget judgment persistence to PostgreSQL.

    Creates an asyncio Task so we never block the HTTP response.
    DB failures are logged but never raised.
    """
    async def _do_save():
        try:
            repo = _get_judgment_repo()
            data = judgment.to_dict()
            # Add fields not in to_dict() but needed by schema
            data.setdefault("cell_id", judgment.cell.cell_id)
            data.setdefault("time_dim", judgment.cell.time_dim)
            data.setdefault("lod", judgment.cell.lod)
            data.setdefault("consciousness", judgment.cell.consciousness)
            data["reality"] = judgment.cell.reality
            data["analysis"] = judgment.cell.analysis
            await repo.save(data)
        except Exception as e:
            logger.warning("Judgment persistence failed (data may be lost): %s", type(e).__name__, exc_info=True)

    import asyncio
    try:
        asyncio.get_running_loop().create_task(_do_save())
    except RuntimeError:
        # No event loop running, skip persistence
        logger.warning("No event loop available for persistence task")


def _write_guidance(cell: Cell, judgment: Judgment) -> None:  # type: ignore[name-defined]
    """
    Write last judgment as guidance.json — the feedback loop.

    JS hooks read this file at the next UserPromptSubmit to inject
    kernel recommendations into Claude Code's context.
    Best-effort: never raises, never blocks the response.
    """
    try:
        os.makedirs(os.path.dirname(_GUIDANCE_PATH), exist_ok=True)
        with open(_GUIDANCE_PATH, "w", encoding="utf-8") as fh:
            json.dump({
                "timestamp": time.time(),
                "state_key": f"{cell.reality}:{cell.analysis}:PRESENT:{cell.lod}",
                "verdict": judgment.verdict,
                "q_score": round(judgment.q_score, 3),
                "confidence": round(min(judgment.confidence, MAX_CONFIDENCE), 4),
                "reality": cell.reality,
                "dog_votes": {k: round(v, 3) for k, v in judgment.dog_votes.items()},
            }, fh)
    except json.JSONDecodeError:
        pass  # Best-effort — never propagate


# ════════════════════════════════════════════════════════════════════════════
# POST /judge
# ════════════════════════════════════════════════════════════════════════════

@router_core.post("/judge", response_model=JudgeResponse)
async def judge(req: JudgeRequest) -> JudgeResponse:
    """
    Run the full CYNIC judgment pipeline on any content.

    Level selection:
    - REFLEX  → fast (<10ms), non-LLM Dogs only, confidence 38.2%
    - MICRO   → medium (~500ms), voting Dogs, confidence 61.8%
    - MACRO   → full 7-step cycle (~2.85s), all Dogs, max confidence
    - None    → auto-selected by consciousness state + budget

    Track E: Event-first API — returns PENDING immediately, processes asynchronously.
    Clients poll GET /judge/{judgment_id} for results.
    """
    state = get_state()

    # Build Cell — enrich context with compressed session history (γ2)
    # Gives LLM dogs (SAGE) temporal continuity: "here's what we judged before"
    history_ctx = state.context_compressor.get_compressed_context(budget=400)
    enriched_context = req.context or ""
    if history_ctx:
        enriched_context = f"{enriched_context}\n[Session history]\n{history_ctx}".strip()

    # Lazy Materialization: infer time_dim if not explicitly provided (Bug 5 fix)
    from cynic.core.judgment import infer_time_dim
    time_dim = req.time_dim or infer_time_dim(req.content, enriched_context, req.analysis)

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

    # Emit JUDGMENT_REQUESTED — fire-and-forget, never block
    from cynic.core.events_schema import JudgmentRequestedPayload
    try:
        await get_core_bus().emit(Event.typed(
            CoreEvent.JUDGMENT_REQUESTED,
            JudgmentRequestedPayload(
                cell_id=cell.cell_id,
                reality=cell.reality,
                level=req.level or "",
                cell=cell.model_dump(),
                source="api:judge",
                judgment_id=judgment_id,
            ),
            source="api:judge",
        ))
    except Exception as exc:
        logger.warning("JUDGMENT_REQUESTED emission failed: %s", exc)

    # Record PENDING placeholder for polling
    try:
        from cynic.organism.conscious_state import get_conscious_state
        await get_conscious_state().record_pending_judgment(judgment_id)
    except Exception as exc:
        logger.warning("record_pending_judgment failed: %s", exc, exc_info=True)

    # Return immediately with PENDING
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


# ════════════════════════════════════════════════════════════════════════════
# POST /perceive  (JS hooks → Python kernel bridge)
# ════════════════════════════════════════════════════════════════════════════

@router_core.post("/perceive", response_model=PerceiveResponse)
async def perceive(req: PerceiveRequest) -> PerceiveResponse:
    """
    Receive raw perception from any source (JS hooks, external services, etc.).

    This is the primary bridge endpoint:
      JS thin hooks → POST /perceive → Python kernel judges it

    Track E: Event-first API — returns PENDING immediately if run_judgment=True.
    Clients poll GET /perceive/{judgment_id} for results.
    """
    state = get_state()
    cell_id = str(uuid.uuid4())
    judgment_id = str(uuid.uuid4())

    # Use content if provided (Track E), else fall back to data (legacy)
    perception_data = req.content if req.content is not None else req.data

    # Emit PERCEPTION_RECEIVED on the core bus
    await get_core_bus().emit(Event.typed(
        CoreEvent.PERCEPTION_RECEIVED,
        PerceptionReceivedPayload(
            cell_id=cell_id,
            source=req.source,
            reality=req.reality,
            data=str(perception_data)[:500],  # truncate for bus
        ),
    ))

    # Lazy Materialization: infer time_dim from perception data (Bug 5 fix)
    from cynic.core.judgment import infer_time_dim as _infer_td
    _perceive_ctx = req.context or f"Perception from {req.source}"
    time_dim = req.time_dim or _infer_td(perception_data, _perceive_ctx, "PERCEIVE")

    # Build cell (used for both enqueue and event emission)
    cell = Cell(
        reality=req.reality,
        analysis="PERCEIVE",
        time_dim=time_dim,
        content=perception_data,
        context=_perceive_ctx,
        lod=0,  # REFLEX = pattern level
        budget_usd=0.001,  # minimal budget for perception
    )
    object.__setattr__(cell, "cell_id", cell_id)

    if not req.run_judgment:
        # Submit to DogScheduler for background MICRO processing
        state.scheduler.submit(
            cell,
            level=ConsciousnessLevel.MICRO,
            budget_usd=0.03,
            source=req.source,
        )
        return PerceiveResponse(
            cell_id=cell_id,
            source=req.source,
            reality=req.reality,
            enqueued=True,
            message="Perception enqueued for background MICRO processing",
        )

    # Emit JUDGMENT_REQUESTED event — fire-and-forget, never block (Track E)
    from cynic.core.events_schema import JudgmentRequestedPayload
    try:
        await get_core_bus().emit(Event.typed(
            CoreEvent.JUDGMENT_REQUESTED,
            JudgmentRequestedPayload(
                cell_id=cell.cell_id,
                reality=cell.reality,
                level=req.level or "",
                cell=cell.model_dump(),
                source="api:perceive",
                judgment_id=judgment_id,
            ),
            source="api:perceive",
        ))
    except Exception as exc:
        logger.warning("JUDGMENT_REQUESTED emission (perceive) failed: %s", exc)

    # Record PENDING placeholder for polling (Track E)
    try:
        from cynic.organism.conscious_state import get_conscious_state
        await get_conscious_state().record_pending_judgment(judgment_id)
    except Exception as exc:
        logger.debug("record_pending_judgment skipped: %s", exc)

    # Return immediately with PENDING (Track E)
    return PerceiveResponse(
        cell_id=cell_id,
        source=req.source,
        reality=req.reality,
        judgment_id=judgment_id,
        verdict="PENDING",
        message="Perception enqueued",
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /judge/{judgment_id} (Track E: polling endpoint)
# ════════════════════════════════════════════════════════════════════════════

@router_core.get("/judge/{judgment_id}")
async def get_judgment_result(
    judgment_id: str,
    timeout_ms: int = Query(default=0, ge=0, description="Max wait in ms. 0=return immediately")
):
    """Poll for judgment result by ID (Track E+G: event-first API with resilience).

    Query params:
      - timeout_ms: Max milliseconds to wait for result (0 = return immediately)

    Returns:
      - 200 with verdict (if available or failed)
      - 408 Request Timeout (if timeout_ms exceeded and still PENDING)
      - 404 Not Found (if judgment_id never seen)
    """
    # Prefer container.organism.conscious_state (patched in tests)
    from cynic.api.state import container as _container
    conscious_state = None
    if _container is not None:
        conscious_state = getattr(getattr(_container, "organism", None), "conscious_state", None)
    # Fallback to singleton (production without full container wiring)
    if conscious_state is None:
        from cynic.organism.conscious_state import get_conscious_state
        conscious_state = get_conscious_state()

    # Poll with optional timeout
    start_time = time.time()
    timeout_s = timeout_ms / 1000.0 if timeout_ms > 0 else 0

    while True:
        result = await conscious_state.get_judgment_by_id(judgment_id)

        if result is not None:
            # Got result (PENDING, real verdict, or BARK on failure)
            if hasattr(result, "__dataclass_fields__"):
                import dataclasses
                return dataclasses.asdict(result)
            return result

        # No result yet
        if timeout_s > 0:
            elapsed = time.time() - start_time
            if elapsed >= timeout_s:
                # Timeout exceeded, still PENDING
                raise HTTPException(
                    status_code=408,
                    detail=f"Judgment {judgment_id} still PENDING after {timeout_ms}ms",
                    headers={"Retry-After": "1"}  # Suggest retry in 1 second
                )
            # Not timed out yet, wait a bit and retry
            await asyncio.sleep(min(0.1, timeout_s - elapsed))
        else:
            # No timeout, return not found
            raise HTTPException(status_code=404, detail="Judgment not found")


# ════════════════════════════════════════════════════════════════════════════
# GET /perceive/{judgment_id} (Track E: polling endpoint)
# ════════════════════════════════════════════════════════════════════════════

@router_core.get("/perceive/{judgment_id}")
async def get_perceive_result(
    judgment_id: str,
    timeout_ms: int = Query(default=0, ge=0, description="Max wait in ms. 0=return immediately")
):
    """Poll for perception result by ID (Track E+G: event-first API with resilience).

    Query params:
      - timeout_ms: Max milliseconds to wait for result (0 = return immediately)

    Returns:
      - 200 with verdict (if available or failed)
      - 408 Request Timeout (if timeout_ms exceeded and still PENDING)
      - 404 Not Found (if judgment_id never seen)
    """
    # Prefer container.organism.conscious_state (patched in tests)
    from cynic.api.state import container as _container
    conscious_state = None
    if _container is not None:
        conscious_state = getattr(getattr(_container, "organism", None), "conscious_state", None)
    # Fallback to singleton (production without full container wiring)
    if conscious_state is None:
        from cynic.organism.conscious_state import get_conscious_state
        conscious_state = get_conscious_state()

    # Poll with optional timeout
    start_time = time.time()
    timeout_s = timeout_ms / 1000.0 if timeout_ms > 0 else 0

    while True:
        result = await conscious_state.get_judgment_by_id(judgment_id)

        if result is not None:
            # Got result (PENDING, real verdict, or BARK on failure)
            if hasattr(result, "__dataclass_fields__"):
                import dataclasses
                return dataclasses.asdict(result)
            return result

        # No result yet
        if timeout_s > 0:
            elapsed = time.time() - start_time
            if elapsed >= timeout_s:
                # Timeout exceeded, still PENDING
                raise HTTPException(
                    status_code=408,
                    detail=f"Perception {judgment_id} still PENDING after {timeout_ms}ms",
                    headers={"Retry-After": "1"}
                )
            # Not timed out yet, wait a bit and retry
            await asyncio.sleep(min(0.1, timeout_s - elapsed))
        else:
            # No timeout, return not found
            raise HTTPException(status_code=404, detail="Perception not found")


# ════════════════════════════════════════════════════════════════════════════
# POST /learn
# ════════════════════════════════════════════════════════════════════════════

@router_core.post("/learn", response_model=LearnResponse)
async def learn(req: LearnRequest) -> LearnResponse:
    """
    Inject a learning signal directly into the Q-Table.

    Useful for:
    - User feedback (human says "that judgment was wrong")
    - JS system sending learning signals from its own experience
    - Testing the learning loop from outside
    """
    state = get_state()

    signal = LearningSignal(
        state_key=req.state_key,
        action=req.action,
        reward=req.reward,
        judgment_id=req.judgment_id,
        loop_name=req.loop_name,
    )
    entry = state.qtable.update(signal)
    confidence = state.qtable.confidence(req.state_key)

    return LearnResponse(
        state_key=entry.state_key,
        action=entry.action,
        q_value=round(entry.q_value, 4),
        visits=entry.visits,
        confidence=round(confidence, 4),
        wins=entry.wins,
        losses=entry.losses,
    )


# ════════════════════════════════════════════════════════════════════════════
# POST /feedback  (explicit user reward → Q-Table)
# ════════════════════════════════════════════════════════════════════════════

@router_core.post("/feedback", response_model=FeedbackResponse)
async def feedback(req: FeedbackRequest) -> FeedbackResponse:
    """
    Inject explicit user feedback into the Q-Table.

    The user rates the last kernel judgment (1=bad, 5=good).
    Rating maps to reward: (rating - 1) / 4 * 0.8 + 0.1 → [0.1, 0.9]
    φ-aligned: never reaches 0 or 1 (LAW OF DOUBT — even perfect is not certain).

    This closes the human feedback loop: user experience → Q-Table learning.
    After enough feedback, CYNIC will predict better verdicts for each context.
    """
    state = get_state()

    if state.last_judgment is None:
        raise HTTPException(
            status_code=404,
            detail="No recent judgment to rate — submit a prompt first",
        )

    last = state.last_judgment
    # φ-aligned reward: never reaches 0 or 1 (epistemic humility)
    # 1→0.1, 2→0.3, 3→0.5, 4→0.7, 5→0.9
    reward = (req.rating - 1) / 4 * 0.8 + 0.1

    signal = LearningSignal(
        state_key=last["state_key"],
        action=last["action"],
        reward=reward,
        judgment_id=last.get("judgment_id", ""),
        loop_name="USER_FEEDBACK",
    )
    entry = state.qtable.update(signal)

    # SYMBIOSIS axiom: human×machine value creation — human feedback is symbiosis in action
    try:
        new_state = state.axiom_monitor.signal("SYMBIOSIS")
        if new_state == "ACTIVE":
            await get_core_bus().emit(Event.typed(
                CoreEvent.AXIOM_ACTIVATED,
                AxiomActivatedPayload(axiom="SYMBIOSIS", maturity=state.axiom_monitor.get_maturity("SYMBIOSIS"), trigger="user_feedback"),
                source="user_feedback",
            ))
    except EventBusError:
        pass

    # USER_FEEDBACK bus event — makes human rating visible organism-wide.
    # Handlers in state.py react (EScore JUDGE update for agent:cynic).
    # This is separate from the inline SYMBIOSIS signal above — bus event lets
    # other components (SelfProber, future handlers) react without touching this code.
    try:
        await get_core_bus().emit(Event.typed(
            CoreEvent.USER_FEEDBACK,
            UserFeedbackPayload(
                rating=req.rating,
                reward=reward,
                sentiment=(req.rating - 3) / 2.0,
                state_key=last["state_key"],
                action=last["action"],
                judgment_id=last.get("judgment_id", ""),
            ),
            source="feedback_endpoint",
        ))
    except httpx.RequestError:
        pass

    # USER_CORRECTION: rating=1 = user explicitly says CYNIC was WRONG.
    # Separate from USER_FEEDBACK (covers all ratings) — this is the active correction.
    # ANTIFRAGILITY: being corrected and learning from it = growth through stress.
    if req.rating == 1:
        try:
            await get_core_bus().emit(Event.typed(
                CoreEvent.USER_CORRECTION,
                UserCorrectionPayload(
                    action=last["action"],
                    state_key=last["state_key"],
                    rating=req.rating,
                    judgment_id=last.get("judgment_id", ""),
                ),
                source="feedback_endpoint",
            ))
        except EventBusError:
            pass

    # Social loop: user rating → sentiment signal → SocialWatcher → SOCIAL×PERCEIVE
    # sentiment: rating 1→-1.0, 3→0.0, 5→+1.0; volume: rating×10
    _append_social_signal(
        source="cynic_feedback",
        sentiment=(req.rating - 3) / 2.0,
        volume=float(req.rating * 10),
        topic=last.get("action", "judgment"),
        signal_type="user_rating",
    )

    verdict_emoji = {"HOWL": "🟢", "WAG": "🟡", "GROWL": "🟠", "BARK": "🔴"}.get(last["action"], "⚪")
    msg = f"*tail wag* Feedback: rating={req.rating}/5 → reward={reward:.2f} → Q[{last['state_key']}][{last['action']}]={entry.q_value:.3f}"

    return FeedbackResponse(
        state_key=entry.state_key,
        action=entry.action,
        reward=round(reward, 3),
        q_value=round(entry.q_value, 4),
        visits=entry.visits,
        message=msg,
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /policy/{state_key}
# ════════════════════════════════════════════════════════════════════════════

@router_core.get("/policy/{state_key}", response_model=PolicyResponse)
async def policy(
    state_key: str,
    mode: str = Query(default="explore", pattern="^(exploit|explore)$"),
) -> PolicyResponse:
    """
    Query the learned policy for a given state.

    mode=exploit → greedy (best known action)
    mode=explore → Thompson Sampling (Bayesian exploration)

    Use exploit in production, explore during learning.
    """
    state = get_state()

    if mode == "exploit":
        action = state.qtable.exploit(state_key)
    else:
        action = state.qtable.explore(state_key)

    q_value = state.qtable.predict_q(state_key, action)
    confidence = state.qtable.confidence(state_key)

    # Top actions for transparency
    from cynic.learning.qlearning import VERDICTS
    top_actions = [
        {
            "action": a,
            "q_value": round(state.qtable.predict_q(state_key, a), 4),
        }
        for a in VERDICTS
    ]
    top_actions.sort(key=lambda x: x["q_value"], reverse=True)

    return PolicyResponse(
        state_key=state_key,
        mode=mode,
        recommended_action=action,
        q_value=round(q_value, 4),
        confidence=round(confidence, 4),
        top_actions=top_actions,
    )


@router_core.get("/world-state")
async def get_world_state() -> dict[str, Any]:
    """
    GET /world-state — Cross-reality state snapshot from WorldModelUpdater.

    Returns composite_risk (φ-weighted geo mean), dominant_reality,
    active conflicts (HOWL vs BARK across realities), and per-reality
    verdict+q_score. Updated after every JUDGMENT_CREATED event.
    """
    state = get_state()
    return state.world_model.snapshot()


# ════════════════════════════════════════════════════════════════════════════
# LNSP Governance Integration — startup initialization
# ════════════════════════════════════════════════════════════════════════════

# Module-level singleton — set during lifespan startup via setup_lnsp_governance()
_governance_lnsp: Any = None


async def setup_lnsp_governance(gasdf_executor: Any | None = None) -> Any:
    """Initialize LNSP Governance Integration.

    Wire GovernanceLNSP into the CYNIC event pipeline:
    - Creates LNSPManager (instance:governance / region:governance)
    - Creates GovernanceLNSP bridge
    - Registers 4 sensors (proposal, vote, execution, outcome) with Layer 1
    - Registers 1 handler (governance verdict) with Layer 4
    - Wires all layers together
    - Optionally wires GASdfExecutor for on-chain verdict execution

    Call this from the FastAPI lifespan after organism awakening.

    Args:
        gasdf_executor: Optional GASdfExecutor for on-chain execution

    Returns:
        The initialized GovernanceLNSP instance.
    """
    global _governance_lnsp

    # Initialize LNSP Governance Integration
    from cynic.protocol.lnsp.governance_integration import GovernanceLNSP
    from cynic.protocol.lnsp.manager import LNSPManager

    lnsp_manager = LNSPManager(
        instance_id="instance:governance",
        region="governance"
    )
    governance_lnsp = GovernanceLNSP(lnsp_manager, gasdf_executor=gasdf_executor)
    await governance_lnsp.setup()

    # Store for later access (optional, for testing)
    _governance_lnsp = governance_lnsp
    logger.info(
        "LNSP Governance Integration initialized — %d sensors, %d handlers wired",
        len(lnsp_manager.layer1.sensors),
        len(lnsp_manager.layer4.handlers),
    )
    return governance_lnsp
