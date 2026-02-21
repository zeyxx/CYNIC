"""
CYNIC core router — judge · perceive · learn · feedback · policy
"""
from __future__ import annotations

import asyncio
import json
import logging
import os
import time
import uuid
from typing import Any, TYPE_CHECKING


if TYPE_CHECKING:
    from cynic.core.storage.postgres import JudgmentRepository

from fastapi import APIRouter, Depends, HTTPException, Query

from cynic.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import (
    AxiomActivatedPayload,
    PerceptionReceivedPayload,
    UserCorrectionPayload,
    UserFeedbackPayload,
    EmergenceDetectedPayload,
)
from cynic.core.judgment import Cell, Judgment
from cynic.core.phi import MAX_CONFIDENCE
from cynic.learning.qlearning import LearningSignal

from cynic.api.models import (
    JudgeRequest, JudgeResponse,
    PerceiveRequest, PerceiveResponse,
    LearnRequest, LearnResponse,
    FeedbackRequest, FeedbackResponse,
    AccountRequest, AccountResponse,
    PolicyResponse,
)
from cynic.api.state import get_app_container, AppContainer
from cynic.api.routers.utils import _append_social_signal

logger = logging.getLogger("cynic.api.server")

router_core = APIRouter(tags=["core"])

_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")

# Lazy-resolved judgment repo — avoids module-level storage import
_judgment_repo: Any = None


def _get_judgment_repo() -> "JudgmentRepository":
    """Lazily import and cache JudgmentRepository (only when DB is available)."""
    global _judgment_repo
    if _judgment_repo is None:
        from cynic.core.storage.postgres import JudgmentRepository
        _judgment_repo = JudgmentRepository()
    return _judgment_repo


async def _persist_judgment_async(judgment: Judgment) -> None:
    """
    ASYNC persistence to PostgreSQL — MUST BE AWAITED.

    This is NOT fire-and-forget. The caller MUST await this function
    to ensure data is persisted before returning HTTP response.

    Phase 0 fix: Eliminates race condition where HTTP 200 returned
    before database persistence completes.

    Args:
        judgment: The judgment to persist

    Raises:
        ValueError: if judgment_id is missing
        Exception: if persistence fails (NOT silently caught)
    """
    if not judgment.judgment_id:
        raise ValueError("judgment_id required for persistence")

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

        # AWAIT the save — do not fire-and-forget
        await repo.save(data)
        logger.info("Judgment %s persisted successfully", judgment.judgment_id)

    except Exception as e:
        # RAISE, don't silently log
        logger.error("Judgment persistence FAILED: %s", e)
        raise


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
            logger.debug("Judgment persistence skipped: %s", e)

    import asyncio
    try:
        asyncio.get_running_loop().create_task(_do_save())
    except Exception:
        pass  # Never block on persistence failure


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
    except Exception:
        pass  # Best-effort — never propagate


# ════════════════════════════════════════════════════════════════════════════
# POST /judge
# ════════════════════════════════════════════════════════════════════════════

@router_core.post("/judge", response_model=JudgeResponse)
async def judge(req: JudgeRequest, container: AppContainer = Depends(get_app_container)) -> JudgeResponse:
    """
    Run the full CYNIC judgment pipeline on any content.

    Level selection:
    - REFLEX  → fast (<10ms), non-LLM Dogs only, confidence 38.2%
    - MICRO   → medium (~500ms), voting Dogs, confidence 61.8%
    - MACRO   → full 7-step cycle (~2.85s), all Dogs, max confidence
    - None    → auto-selected by consciousness state + budget
    """
    state = container.organism

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

    # Parse consciousness level
    level = None
    if req.level:
        level = ConsciousnessLevel[req.level]

    judgment = await state.orchestrator.run(cell, level=level, budget_usd=req.budget_usd)

    # Write guidance.json — feedback loop to JS hooks (best-effort)
    _write_guidance(cell, judgment)

    # Persist judgment to PostgreSQL (best-effort — never block on DB failures)
    if state._pool is not None:
        _persist_judgment(judgment)

    # Save for /feedback endpoint (user can rate this judgment)
    state.last_judgment = {
        "state_key": f"{cell.reality}:{cell.analysis}:PRESENT:{cell.lod}",
        "action": judgment.verdict,
        "judgment_id": judgment.judgment_id,
    }

    return JudgeResponse(
        judgment_id=judgment.judgment_id,
        q_score=round(judgment.q_score, 3),
        verdict=judgment.verdict,
        confidence=round(min(judgment.confidence, MAX_CONFIDENCE), 4),
        axiom_scores={k: round(v, 3) for k, v in judgment.axiom_scores.items()},
        dog_votes={k: round(v, 3) for k, v in judgment.dog_votes.items()},
        consensus_reached=judgment.consensus_reached,
        consensus_votes=judgment.consensus_votes,
        residual_variance=round(judgment.residual_variance or 0.0, 4),
        unnameable_detected=judgment.unnameable_detected,
        cost_usd=round(judgment.cost_usd, 6),
        llm_calls=judgment.llm_calls,
        duration_ms=round(judgment.duration_ms, 2),
        level_used=level.name if level else "AUTO",
    )


# ════════════════════════════════════════════════════════════════════════════
# POST /perceive  (JS hooks → Python kernel bridge)
# ════════════════════════════════════════════════════════════════════════════

@router_core.post("/perceive", response_model=PerceiveResponse)
async def perceive(req: PerceiveRequest, container: AppContainer = Depends(get_app_container)) -> PerceiveResponse:
    """
    Receive raw perception from any source (JS hooks, external services, etc.).

    This is the primary bridge endpoint:
      JS thin hooks → POST /perceive → Python kernel judges it

    If run_judgment=True (default), runs a REFLEX judgment on the perception.
    This keeps the JS hooks thin while the Python kernel does the cognitive work.
    """
    state = container.organism
    cell_id = str(uuid.uuid4())

    # Emit PERCEPTION_RECEIVED on the core bus
    await get_core_bus().emit(Event.typed(
        CoreEvent.PERCEPTION_RECEIVED,
        PerceptionReceivedPayload(
            cell_id=cell_id,
            source=req.source,
            reality=req.reality,
            data=str(req.data)[:500],  # truncate for bus
        ),
    ))

    # Lazy Materialization: infer time_dim from perception data (Bug 5 fix)
    from cynic.core.judgment import infer_time_dim as _infer_td
    _perceive_ctx = req.context or f"Perception from {req.source}"
    time_dim = req.time_dim or _infer_td(req.data, _perceive_ctx, "PERCEIVE")

    # Build cell (used for both enqueue and immediate judgment)
    cell = Cell(
        reality=req.reality,
        analysis="PERCEIVE",
        time_dim=time_dim,
        content=req.data,
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

    level_map = {
        "REFLEX": ConsciousnessLevel.REFLEX,
        "MICRO":  ConsciousnessLevel.MICRO,
        "MACRO":  ConsciousnessLevel.MACRO,
        "META":   ConsciousnessLevel.META,
    }
    level = level_map.get(req.level or "REFLEX", ConsciousnessLevel.REFLEX)

    judgment = await state.orchestrator.run(cell, level=level)

    # SAGE amplification: after fast REFLEX, enqueue MACRO to scheduler.
    # MACRO runs all 11 Dogs including SAGE temporal MCTS (7×Ollama).
    # JUDGMENT_CREATED handler (in state.py) overwrites guidance.json when done.
    # → Next hook call gets SAGE's wisdom. Lagged by one cycle (acceptable).
    # Only enqueue for REFLEX perception (CODE/HUMAN reality) — not self-loops.
    if level == ConsciousnessLevel.REFLEX and cell.reality in ("CODE", "HUMAN", "MARKET", "SOCIAL"):
        from cynic.core.judgment import Cell as _Cell, infer_time_dim as _itd2
        macro_cell = _Cell(
            reality=cell.reality,
            analysis="JUDGE",
            time_dim=cell.time_dim,  # Propagate inferred time_dim to MACRO follow-up
            content=cell.content,
            context=cell.context,
            budget_usd=0.05,       # enough for MACRO + Ollama temporal MCTS
            consciousness=4,       # REFLECTIVE gradient — deep analysis
        )
        state.scheduler.submit(macro_cell, level=ConsciousnessLevel.MACRO, source=f"perceive_bg:{req.source}")

    j_resp = JudgeResponse(
        judgment_id=judgment.judgment_id,
        q_score=round(judgment.q_score, 3),
        verdict=judgment.verdict,
        confidence=round(min(judgment.confidence, MAX_CONFIDENCE), 4),
        axiom_scores={k: round(v, 3) for k, v in judgment.axiom_scores.items()},
        dog_votes={k: round(v, 3) for k, v in judgment.dog_votes.items()},
        consensus_reached=judgment.consensus_reached,
        consensus_votes=judgment.consensus_votes,
        cost_usd=round(judgment.cost_usd, 6),
        llm_calls=judgment.llm_calls,
        duration_ms=round(judgment.duration_ms, 2),
        level_used=level.name,
    )

    # Write guidance.json — best-effort belt-and-suspenders backup.
    # Primary writer is now the JUDGMENT_CREATED event handler in state.py.
    _write_guidance(cell, judgment)

    # Save for /feedback endpoint
    state.last_judgment = {
        "state_key": f"{cell.reality}:{cell.analysis}:PRESENT:{cell.lod}",
        "action": judgment.verdict,
        "judgment_id": judgment.judgment_id,
    }

    return PerceiveResponse(
        cell_id=cell_id,
        source=req.source,
        reality=req.reality,
        judgment=j_resp,
        message=f"Perception judged: {judgment.verdict} (Q={judgment.q_score:.1f})",
    )


# ════════════════════════════════════════════════════════════════════════════
# POST /learn
# ════════════════════════════════════════════════════════════════════════════

@router_core.post("/learn", response_model=LearnResponse)
async def learn(req: LearnRequest, container: AppContainer = Depends(get_app_container)) -> LearnResponse:
    """
    Inject a learning signal directly into the Q-Table.

    Useful for:
    - User feedback (human says "that judgment was wrong")
    - JS system sending learning signals from its own experience
    - Testing the learning loop from outside
    """
    state = container.organism

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
async def feedback(req: FeedbackRequest, container: AppContainer = Depends(get_app_container)) -> FeedbackResponse:
    """
    Inject explicit user feedback into the Q-Table.

    The user rates the last kernel judgment (1=bad, 5=good).
    Rating maps to reward: (rating - 1) / 4 * 0.8 + 0.1 → [0.1, 0.9]
    φ-aligned: never reaches 0 or 1 (LAW OF DOUBT — even perfect is not certain).

    This closes the human feedback loop: user experience → Q-Table learning.
    After enough feedback, CYNIC will predict better verdicts for each context.
    """
    state = container.organism

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
    except Exception:
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
    except Exception:
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
        except Exception:
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
# POST /account  (Step 6: Cost accounting + EMERGE pattern detection)
# ════════════════════════════════════════════════════════════════════════════

@router_core.post("/account", response_model=AccountResponse)
async def account(req: AccountRequest, container: AppContainer = Depends(get_app_container)) -> AccountResponse:
    """
    Execute the ACCOUNT opcode (Step 6 of 7-step cycle).

    ACCOUNT is the cost ledger step:
    1. Cost tracking per judgment, reality, dog
    2. Budget enforcement (warn at 38.2%, exhaust at 0%)
    3. EScore-RUN dimension updates (efficiency per dog)
    4. Trigger EMERGE pattern detection (optional)

    ACCOUNT closes the L1 loop when combined with EMERGE (Step 7).
    """
    state = container.organism

    # Get current cost snapshot before triggering EMERGE
    cost_stats = state.account_agent.stats()

    # Trigger EMERGE pattern detection (Step 7) if requested
    emergence_detected = False
    emergence_pattern = ""
    if req.trigger_emerge and state.residual_detector:
        try:
            # ResidualDetector.observe() detects patterns and emits EMERGENCE_DETECTED
            # if a pattern is found (SPIKE, RISING, STABLE_HIGH)
            patterns = state.residual_detector.observe()
            if patterns:
                emergence_detected = True
                emergence_pattern = patterns[0].pattern_type if patterns else ""

                # Explicitly emit EMERGENCE_DETECTED for L1→L2 cross-talk
                await get_core_bus().emit(Event.typed(
                    CoreEvent.EMERGENCE_DETECTED,
                    {
                        "pattern_type": emergence_pattern,
                        "total_patterns": len(patterns),
                        "reality": "CODE",  # inferred from most recent judgment
                        "severity": patterns[0].severity if patterns else 0.5,
                        "evidence": patterns[0].evidence if patterns else {},
                    },
                    source="account_endpoint",
                ))
        except Exception as e:
            logger.debug("EMERGE pattern detection (non-fatal): %s", e)

    # Emit COST_ACCOUNTED event — closes ACCOUNT opcode
    try:
        await get_core_bus().emit(Event.typed(
            CoreEvent.COST_ACCOUNTED,
            {
                "total_cost_usd": cost_stats["total_cost_usd"],
                "judgment_count": cost_stats["judgment_count"],
                "budget_remaining_usd": cost_stats["budget_remaining_usd"],
                "emergence_detected": emergence_detected,
            },
            source="account_endpoint",
        ))
    except Exception as e:
        logger.debug("COST_ACCOUNTED event (non-fatal): %s", e)

    msg = f"*sniff* ACCOUNT: ${cost_stats['total_cost_usd']:.4f} / ${cost_stats['session_budget_usd']:.2f} spent"
    if emergence_detected:
        msg += f" · EMERGE detected: {emergence_pattern}"
    else:
        msg += " · EMERGE: no pattern"

    return AccountResponse(
        cost_usd=round(cost_stats["total_cost_usd"], 4),
        budget_remaining_usd=round(cost_stats["budget_remaining_usd"], 4),
        budget_ratio=round(cost_stats["budget_ratio_remaining"], 3),
        judgment_count=cost_stats["judgment_count"],
        warning_emitted=cost_stats["warning_emitted"],
        exhausted_emitted=cost_stats["exhausted_emitted"],
        emergence_detected=emergence_detected,
        emergence_pattern=emergence_pattern,
        message=msg,
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /policy/{state_key}
# ════════════════════════════════════════════════════════════════════════════

@router_core.get("/policy/{state_key}", response_model=PolicyResponse)
async def policy(
    state_key: str,
    mode: str = Query(default="explore", pattern="^(exploit|explore)$"),
    container: AppContainer = Depends(get_app_container),
) -> PolicyResponse:
    """
    Query the learned policy for a given state.

    mode=exploit → greedy (best known action)
    mode=explore → Thompson Sampling (Bayesian exploration)

    Use exploit in production, explore during learning.
    """
    state = container.organism

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
async def get_world_state(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    GET /world-state — Cross-reality state snapshot from WorldModelUpdater.

    Returns composite_risk (φ-weighted geo mean), dominant_reality,
    active conflicts (HOWL vs BARK across realities), and per-reality
    verdict+q_score. Updated after every JUDGMENT_CREATED event.
    """
    state = container.organism
    return state.world_model.snapshot()
