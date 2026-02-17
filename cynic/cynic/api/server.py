"""
CYNIC FastAPI Server — Phase 1 API Bridge

Exposes the Python kernel over HTTP so JS hooks (and anything else) can call it.
This is the living interface between the legacy JS system and the Python organism.

Routes:
  POST /judge          → Full judgment pipeline (REFLEX/MICRO/MACRO)
  POST /perceive       → Accept raw perception, optionally run judgment
  POST /learn          → Inject learning signal directly into QTable
  GET  /policy/{key}   → Query learned policy for a state
  GET  /health         → Kernel health (consciousness, dogs, learning)
  GET  /stats          → Detailed metrics

Design principles:
  - No state in route handlers (all state in AppState singleton)
  - φ-bound all confidence values before returning
  - Errors return structured JSON (never HTML 500s)
  - Every response includes judgment_id for traceability
"""
from __future__ import annotations

import logging
import os
import time
import uuid
from contextlib import asynccontextmanager
from typing import Optional

from fastapi import FastAPI, HTTPException, Query, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from cynic.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.judgment import Cell
from cynic.core.phi import PHI, MAX_CONFIDENCE
from cynic.learning.qlearning import LearningSignal

from cynic.api.models import (
    JudgeRequest, JudgeResponse,
    PerceiveRequest, PerceiveResponse,
    LearnRequest, LearnResponse,
    PolicyResponse,
    HealthResponse,
    StatsResponse,
)
from cynic.api.state import build_kernel, set_state, get_state

logger = logging.getLogger("cynic.api.server")

_boot_time = time.time()


# ════════════════════════════════════════════════════════════════════════════
# LIFESPAN — kernel startup / shutdown
# ════════════════════════════════════════════════════════════════════════════

@asynccontextmanager
async def lifespan(app: FastAPI):
    """Build the kernel once on startup, tear down on shutdown."""
    logger.info("*sniff* CYNIC kernel booting...")

    db_pool = None
    db_url = os.getenv("DATABASE_URL")
    if db_url:
        try:
            import asyncpg  # type: ignore
            db_pool = await asyncpg.create_pool(dsn=db_url, min_size=2, max_size=10)

            # Warm-start Q-Table from DB
            state = build_kernel(db_pool=db_pool)
            loaded = await state.qtable.load_from_db(db_pool)
            logger.info("Q-Table warm-start: %d entries loaded", loaded)
        except Exception as exc:
            logger.warning("DB unavailable (%s) — running without persistence", exc)
            db_pool = None
            state = build_kernel(db_pool=None)
    else:
        logger.info("No DATABASE_URL — running without persistence")
        state = build_kernel(db_pool=None)

    set_state(state)
    logger.info("*tail wag* CYNIC kernel alive — %d dogs, learning active", len(state.dogs))

    yield

    # Shutdown
    logger.info("*yawn* CYNIC kernel shutting down...")
    state.learning_loop.stop()
    if db_pool:
        await state.qtable.flush_to_db(db_pool)
        await db_pool.close()


# ════════════════════════════════════════════════════════════════════════════
# APP
# ════════════════════════════════════════════════════════════════════════════

app = FastAPI(
    title="CYNIC Kernel API",
    description="Python kernel — φ-bounded judgment + learning",
    version="2.0.0",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],        # JS hooks call from same machine or Render
    allow_methods=["*"],
    allow_headers=["*"],
)


# ════════════════════════════════════════════════════════════════════════════
# ERROR HANDLING
# ════════════════════════════════════════════════════════════════════════════

@app.exception_handler(Exception)
async def global_exception_handler(request: Request, exc: Exception) -> JSONResponse:
    logger.error("Unhandled error on %s: %s", request.url.path, exc, exc_info=True)
    return JSONResponse(
        status_code=500,
        content={
            "error": str(exc),
            "path": str(request.url.path),
            "cynic": "φ distrusts φ — even the kernel can fail",
        },
    )


# ════════════════════════════════════════════════════════════════════════════
# POST /judge
# ════════════════════════════════════════════════════════════════════════════

@app.post("/judge", response_model=JudgeResponse)
async def judge(req: JudgeRequest) -> JudgeResponse:
    """
    Run the full CYNIC judgment pipeline on any content.

    Level selection:
    - REFLEX  → fast (<10ms), non-LLM Dogs only, confidence 38.2%
    - MICRO   → medium (~500ms), voting Dogs, confidence 61.8%
    - MACRO   → full 7-step cycle (~2.85s), all Dogs, max confidence
    - None    → auto-selected by consciousness state + budget
    """
    state = get_state()

    # Build Cell from request
    cell = Cell(
        reality=req.reality,
        analysis=req.analysis,
        content=req.content,
        context=req.context,
        lod=req.lod,
        budget_usd=req.budget_usd,
    )

    # Parse consciousness level
    level = None
    if req.level:
        level = ConsciousnessLevel[req.level]

    judgment = await state.orchestrator.run(cell, level=level, budget_usd=req.budget_usd)

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

@app.post("/perceive", response_model=PerceiveResponse)
async def perceive(req: PerceiveRequest) -> PerceiveResponse:
    """
    Receive raw perception from any source (JS hooks, external services, etc.).

    This is the primary bridge endpoint:
      JS thin hooks → POST /perceive → Python kernel judges it

    If run_judgment=True (default), runs a REFLEX judgment on the perception.
    This keeps the JS hooks thin while the Python kernel does the cognitive work.
    """
    state = get_state()
    cell_id = str(uuid.uuid4())

    # Emit PERCEPTION_RECEIVED on the core bus
    await get_core_bus().emit(Event(
        type=CoreEvent.PERCEPTION_RECEIVED,
        payload={
            "cell_id": cell_id,
            "source": req.source,
            "reality": req.reality,
            "data": str(req.data)[:500],  # truncate for bus
        },
    ))

    if not req.run_judgment:
        return PerceiveResponse(
            cell_id=cell_id,
            source=req.source,
            reality=req.reality,
            enqueued=True,
            message="Perception received, judgment skipped (run_judgment=False)",
        )

    # Build cell from perception and run REFLEX judgment
    cell = Cell(
        reality=req.reality,
        analysis="PERCEIVE",
        content=req.data,
        context=req.context or f"Perception from {req.source}",
        lod=0,  # REFLEX = pattern level
        budget_usd=0.001,  # minimal budget for perception
    )
    # Assign the pre-generated cell_id
    object.__setattr__(cell, "cell_id", cell_id)

    level_map = {"REFLEX": ConsciousnessLevel.REFLEX, "MICRO": ConsciousnessLevel.MICRO}
    level = level_map.get(req.level or "REFLEX", ConsciousnessLevel.REFLEX)

    judgment = await state.orchestrator.run(cell, level=level)

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

@app.post("/learn", response_model=LearnResponse)
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
# GET /policy/{state_key}
# ════════════════════════════════════════════════════════════════════════════

@app.get("/policy/{state_key}", response_model=PolicyResponse)
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


# ════════════════════════════════════════════════════════════════════════════
# GET /health
# ════════════════════════════════════════════════════════════════════════════

@app.get("/health", response_model=HealthResponse)
async def health() -> HealthResponse:
    """
    Kernel health — the organism's vital signs.

    status=alive    → all systems nominal
    status=degraded → partial functionality (e.g. no DB, no LLM)
    status=dead     → kernel not initialized (should never reach this route)
    """
    state = get_state()
    consciousness = get_consciousness()
    judge_stats = state.orchestrator.stats()
    learn_stats = state.qtable.stats()

    # Determine status
    status = "alive"
    if not state.learning_loop._active:
        status = "degraded"

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
        llm_adapters=[],   # populated when LLMRegistry is wired
        judgments_total=judge_stats["judgments_total"],
        phi=PHI,
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /stats
# ════════════════════════════════════════════════════════════════════════════

@app.get("/stats", response_model=StatsResponse)
async def stats() -> StatsResponse:
    """Detailed kernel metrics — everything CYNIC knows about itself."""
    state = get_state()

    return StatsResponse(
        judgments=state.orchestrator.stats(),
        learning=state.qtable.stats(),
        top_states=state.qtable.top_states(n=10),
        consciousness=get_consciousness().to_dict(),
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /  (root — for quick sanity check)
# ════════════════════════════════════════════════════════════════════════════

@app.get("/")
async def root():
    return {
        "name": "CYNIC Kernel",
        "version": "2.0.0",
        "status": "alive",
        "φ": PHI,
        "routes": ["/judge", "/perceive", "/learn", "/policy/{key}", "/health", "/stats"],
        "message": "*sniff* Le chien est là.",
    }
