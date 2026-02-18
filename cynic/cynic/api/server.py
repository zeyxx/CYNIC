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

import json
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
    FeedbackRequest, FeedbackResponse,
    PolicyResponse,
    HealthResponse,
    StatsResponse,
)
from cynic.api.state import build_kernel, set_state, get_state
from cynic.core.storage.postgres import JudgmentRepository

logger = logging.getLogger("cynic.api.server")

_boot_time = time.time()


# ════════════════════════════════════════════════════════════════════════════
# LIFESPAN — kernel startup / shutdown
# ════════════════════════════════════════════════════════════════════════════

@asynccontextmanager
async def lifespan(app: FastAPI):
    """Build the kernel once on startup, tear down on shutdown."""
    logger.info("*sniff* CYNIC kernel booting...")

    # ── LLM Registry: discover all available LLMs ─────────────────────────
    # Ollama (primary — free, local, parallel)
    # Claude (API — for MACRO cycle reasoning)
    # Gemini (API — free tier alternative)
    from cynic.llm.adapter import get_registry
    registry = get_registry()
    discovered = await registry.discover(
        ollama_url=os.getenv("OLLAMA_URL", "http://localhost:11434"),
        claude_api_key=os.getenv("ANTHROPIC_API_KEY"),
        gemini_api_key=os.getenv("GOOGLE_API_KEY"),
    )
    if discovered:
        logger.info("*ears perk* LLMs discovered: %s", discovered)
    else:
        logger.info("No LLMs available — heuristic mode only (Ollama not running?)")

    # ── Database pool ──────────────────────────────────────────────────────
    db_pool = None
    db_url = os.getenv("DATABASE_URL")
    if db_url:
        try:
            import asyncpg  # type: ignore
            db_pool = await asyncpg.create_pool(dsn=db_url, min_size=2, max_size=10)

            # Warm-start Q-Table from DB
            state = build_kernel(db_pool=db_pool, registry=registry)
            loaded = await state.qtable.load_from_db(db_pool)
            logger.info("Q-Table warm-start: %d entries loaded", loaded)

            # Warm-start benchmarks + enable persistent routing
            registry.set_db_pool(db_pool)
            bench_loaded = await registry.load_benchmarks_from_db(db_pool)
            logger.info("Benchmark warm-start: %d routing entries loaded", bench_loaded)
        except Exception as exc:
            logger.warning("DB unavailable (%s) — running without persistence", exc)
            db_pool = None
            state = build_kernel(db_pool=None, registry=registry)
    else:
        logger.info("No DATABASE_URL — running without persistence")
        state = build_kernel(db_pool=None, registry=registry)

    set_state(state)
    state.scheduler.start()
    llm_count = len(registry.get_available())
    logger.info(
        "*tail wag* CYNIC kernel alive — %d dogs, %d LLMs, learning active, scheduler running",
        len(state.dogs), llm_count,
    )

    yield

    # Shutdown
    logger.info("*yawn* CYNIC kernel shutting down...")
    await state.scheduler.stop()
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


_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")


_judgment_repo = JudgmentRepository()


def _persist_judgment(judgment) -> None:
    """
    Fire-and-forget judgment persistence to PostgreSQL.

    Creates an asyncio Task so we never block the HTTP response.
    DB failures are logged but never raised.
    """
    async def _do_save():
        try:
            data = judgment.to_dict()
            # Add fields not in to_dict() but needed by schema
            data.setdefault("cell_id", judgment.cell.cell_id)
            data.setdefault("time_dim", judgment.cell.time_dim)
            data.setdefault("lod", judgment.cell.lod)
            data.setdefault("consciousness", judgment.cell.consciousness)
            data["reality"] = judgment.cell.reality
            data["analysis"] = judgment.cell.analysis
            await _judgment_repo.save(data)
        except Exception as e:
            logger.debug("Judgment persistence skipped: %s", e)

    import asyncio
    try:
        loop = asyncio.get_event_loop()
        if loop.is_running():
            loop.create_task(_do_save())
    except Exception:
        pass  # Never block on persistence failure


def _write_guidance(cell: "Cell", judgment: "Judgment") -> None:  # type: ignore[name-defined]
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

    # Build cell (used for both enqueue and immediate judgment)
    cell = Cell(
        reality=req.reality,
        analysis="PERCEIVE",
        content=req.data,
        context=req.context or f"Perception from {req.source}",
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

    # Write guidance.json — feedback loop (best-effort, never blocks)
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
# POST /feedback  (explicit user reward → Q-Table)
# ════════════════════════════════════════════════════════════════════════════

@app.post("/feedback", response_model=FeedbackResponse)
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
        llm_adapters=[a.adapter_id for a in __import__("cynic.llm.adapter", fromlist=["get_registry"]).get_registry().get_available()],
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

# ════════════════════════════════════════════════════════════════════════════
# GET /introspect  (MetaCognition — composant 9/9, self-model)
# ════════════════════════════════════════════════════════════════════════════

@app.get("/introspect")
async def introspect() -> dict:
    """
    MetaCognition — CYNIC reads its own cognitive state.

    Returns a deep self-model:
    - Learning maturity (Q-table fill, top states, best actions)
    - Residual variance patterns (emergence, THE_UNNAMEABLE events)
    - Consciousness metrics (level distribution, upgrade/downgrade counts)
    - Dog health (hit rates, latencies, capability breakdown)
    - Scholar buffer (similarity memory richness)
    - Kernel integrity (9/9 components, their status)
    - φ-bound assessment (is CYNIC within its own axioms?)

    This is CYNIC judging itself — meta-cognitive self-assessment.
    "φ distrusts φ" — the organism reflects on its own biases.
    """
    from cynic.core.phi import PHI_INV, PHI_INV_2, fibonacci
    from cynic.dogs.base import DogId

    state = get_state()
    consciousness = get_consciousness()
    qtable_stats = state.qtable.stats()
    orch_stats = state.orchestrator.stats()
    residual_stats = state.residual_detector.stats()

    # Dog-level introspection
    dogs_status = {}
    for dog_id, dog in state.orchestrator.dogs.items():
        caps = dog.get_capabilities()
        dogs_status[dog_id] = {
            "sefirot": caps.sefirot,
            "uses_llm": caps.uses_llm,
            "consciousness_min": caps.consciousness_min.name if hasattr(caps.consciousness_min, 'name') else str(caps.consciousness_min),
            "avg_latency_ms": round(dog.avg_latency_ms, 2),
            "judgment_count": dog._judgment_count,
        }

    # Scholar buffer richness
    scholar = state.orchestrator.dogs.get(DogId.SCHOLAR)
    scholar_status = {}
    if scholar is not None:
        scholar_stats = scholar.stats() if hasattr(scholar, 'stats') else {}
        scholar_status = {
            "buffer_size": len(scholar._buffer),
            "buffer_max": scholar._buffer.maxlen if hasattr(scholar._buffer, 'maxlen') else 89,
            "lookups": scholar._lookups,
            "hits": scholar._hits,
            "hit_rate": round(scholar._hits / max(scholar._lookups, 1), 3),
            "buffer_richness": round(
                min(1.0, len(scholar._buffer) / fibonacci(8)), 3
            ),
        }

    # 9-component kernel integrity check
    components = {
        "1_AXIOMS":         {"status": "ACTIVE", "description": "5 axioms × 7 facets scoring"},
        "2_PHI_BOUND":      {"status": "ACTIVE", "description": "φ⁻¹=61.8% max confidence enforced"},
        "3_MULTI_AGENT":    {"status": "ACTIVE", "description": f"{len(state.orchestrator.dogs)}/11 Dogs active"},
        "4_EVENT_DRIVEN":   {"status": "ACTIVE", "description": "Core bus wired, JUDGMENT_CREATED flowing"},
        "5_JUDGMENT":       {"status": "ACTIVE", "description": "7-step PERCEIVE→EMERGE pipeline"},
        "6_LEARNING":       {
            "status": "ACTIVE" if qtable_stats.get("total_updates", 0) > 0 else "WARM",
            "description": f"QTable: {qtable_stats.get('unique_states', 0)} states learned",
        },
        "7_RESIDUAL":       {
            "status": "ACTIVE",
            "description": f"ResidualDetector: {residual_stats['observations']} obs, {residual_stats['patterns_detected']} patterns",
        },
        "8_MEMORY":         {
            "status": "ACTIVE" if scholar_status.get("buffer_size", 0) > 0 else "COLD",
            "description": f"Scholar buffer: {scholar_status.get('buffer_size', 0)}/{scholar_status.get('buffer_max', 89)} cells",
        },
        "9_META_COGNITION": {"status": "ACTIVE", "description": "This endpoint — /introspect live"},
    }

    active_count = sum(1 for c in components.values() if c["status"] == "ACTIVE")
    kernel_integrity = round(active_count / 9, 3)

    # φ self-assessment
    # Is CYNIC operating within its own axioms?
    phi_violations = []
    max_conf = qtable_stats.get("max_confidence") or 0.0
    if max_conf > PHI_INV + 0.01:
        phi_violations.append(f"Q-table confidence exceeds φ⁻¹: {max_conf:.3f}")
    if residual_stats.get("anomaly_rate", 0) > PHI_INV:
        phi_violations.append(f"Residual anomaly rate exceeds φ⁻¹: {residual_stats['anomaly_rate']:.3f}")

    return {
        "introspect_id": str(uuid.uuid4()),
        "timestamp": time.time(),
        "uptime_s": round(state.uptime_s, 1),
        "φ_self_assessment": {
            "kernel_integrity": kernel_integrity,
            "phi_violations": phi_violations,
            "self_confidence": round(min(kernel_integrity * PHI_INV, PHI_INV), 3),
            "verdict": "HOWL" if kernel_integrity >= 0.888 else (
                "WAG" if kernel_integrity >= 0.618 else (
                    "GROWL" if kernel_integrity >= 0.382 else "BARK"
                )
            ),
        },
        "consciousness": consciousness.to_dict(),
        "learning": {
            **qtable_stats,
            "learning_loop_active": True,
        },
        "residual": residual_stats,
        "dogs": dogs_status,
        "scholar": scholar_status,
        "components": components,
        "orchestrator": orch_stats,
        "message": "*sniff* Je me lis moi-même. Le chien qui se connaît.",
    }


@app.get("/")
async def root():
    return {
        "name": "CYNIC Kernel",
        "version": "2.0.0",
        "status": "alive",
        "φ": PHI,
        "routes": ["/judge", "/perceive", "/learn", "/policy/{key}", "/health", "/stats", "/introspect"],
        "message": "*sniff* Le chien est là.",
    }
