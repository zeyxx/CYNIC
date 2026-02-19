"""
CYNIC FastAPI Server — Phase 1 API Bridge

Exposes the Python kernel over HTTP so JS hooks (and anything else) can call it.
This is the living interface between the legacy JS system and the Python organism.

Routes:
  POST /judge                     → Full judgment pipeline (REFLEX/MICRO/MACRO)
  POST /perceive                  → Accept raw perception, optionally run judgment
  POST /learn                     → Inject learning signal directly into QTable
  GET  /actions                   → List proposed actions (PENDING by default)
  POST /actions/{id}/accept       → Accept a proposed action
  POST /actions/{id}/reject       → Reject a proposed action
  GET  /policy/{key}              → Query learned policy for a state
  GET  /health                    → Kernel health (consciousness, dogs, learning)
  GET  /stats                     → Detailed metrics
  GET  /dashboard                 → Live kernel dashboard (WebSocket /ws/stream consumer)

Design principles:
  - No state in route handlers (all state in AppState singleton)
  - φ-bound all confidence values before returning
  - Errors return structured JSON (never HTML 500s)
  - Every response includes judgment_id for traceability
"""
from __future__ import annotations

import asyncio
import json
import logging
import os
import time
import uuid
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from fastapi.staticfiles import StaticFiles

from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import (
    ActCompletedPayload,
    AxiomActivatedPayload as _AxiomActivatedPayload,
    EmergenceDetectedPayload as _EmergenceDetectedPayload,
)
from cynic.core.phi import WAG_MIN
from cynic.core.config import CynicConfig
from cynic.act.telemetry import compute_reward

from cynic.api.state import build_kernel, set_state, get_state, restore_state

from cynic.api.routers.core import router_core
from cynic.api.routers.actions import router_actions
from cynic.api.routers.health import router_health
from cynic.api.routers.sdk import router_sdk, _sdk_sessions
from cynic.api.routers.act import router_act
from cynic.api.routers.ws import router_ws

logger = logging.getLogger("cynic.api.server")

_boot_time = time.time()


# ════════════════════════════════════════════════════════════════════════════
# LIFESPAN — kernel startup / shutdown
# ════════════════════════════════════════════════════════════════════════════

@asynccontextmanager
async def lifespan(app: FastAPI):
    """Build the kernel once on startup, tear down on shutdown."""
    logger.info("*sniff* CYNIC kernel booting...")

    # ── T35: Instance ID + MCP auto-config ────────────────────────────────
    # Each process gets a unique 8-char hex instance_id.
    # Solves two problems:
    #   1. Multi-instance race on guidance.json → each instance writes guidance-{id}.json
    #   2. Cursor/Windsurf MCP connection → auto-write ~/.cursor/mcp.json if absent
    _instance_id = uuid.uuid4().hex[:8]
    logger.info("*sniff* CYNIC instance_id: %s", _instance_id)

    from cynic.api.state import set_instance_id as _set_instance_id
    _set_instance_id(_instance_id)

    # Write ~/.cynic/instance.json — runtime metadata for debugging + MCP tools
    # ── Load config from env (ONE place for all env vars) ─────────────────
    config = CynicConfig.from_env()
    for issue in config.validate():
        logger.warning(issue)

    _instance_meta_path = os.path.join(os.path.expanduser("~"), ".cynic", "instance.json")
    try:
        os.makedirs(os.path.dirname(_instance_meta_path), exist_ok=True)
        with open(_instance_meta_path, "w", encoding="utf-8") as _fh:
            json.dump({
                "instance_id": _instance_id,
                "port": config.port,
                "started_at": time.time(),
            }, _fh, indent=2)
    except Exception as _exc:
        logger.debug("instance.json write failed: %s", _exc)

    # MCP auto-config for Cursor/Windsurf — non-destructive (never overwrites existing)
    _mcp_port = config.port
    _mcp_config = {"cynic": {"url": f"http://localhost:{_mcp_port}"}}
    for _mcp_target in [
        os.path.join(os.path.expanduser("~"), ".cursor", "mcp.json"),
        os.path.join(os.path.expanduser("~"), ".windsurf", "mcp.json"),
    ]:
        if not os.path.exists(_mcp_target):
            try:
                os.makedirs(os.path.dirname(_mcp_target), exist_ok=True)
                with open(_mcp_target, "w", encoding="utf-8") as _fh:
                    json.dump(_mcp_config, _fh, indent=2)
                logger.info("MCP auto-config written: %s", _mcp_target)
            except Exception as _mcp_exc:
                logger.debug("MCP auto-config skipped (%s): %s", _mcp_target, _mcp_exc)

    # ── LLM Registry: discover all available LLMs ─────────────────────────
    # Ollama (primary — free, local, parallel)
    # Claude (API — for MACRO cycle reasoning)
    # Gemini (API — free tier alternative)
    from cynic.llm.adapter import get_registry
    registry = get_registry()

    # Ollama parallel hint — must be set before `ollama serve` (Ollama env var)
    if config.ollama_num_parallel is None:
        logger.info(
            "Tip: set OLLAMA_NUM_PARALLEL=4 before starting Ollama for "
            "parallel MCTS (7 calls → 2 batches instead of 7 sequential)"
        )

    discovered = await registry.discover(
        ollama_url=config.ollama_url,
        claude_api_key=config.anthropic_api_key,
        gemini_api_key=config.google_api_key,
        models_dir=config.models_dir,
        llama_gpu_layers=config.llama_gpu_layers,
        llama_threads=config.llama_threads,
    )
    if discovered:
        logger.info("*ears perk* LLMs discovered: %s", discovered)
    else:
        logger.info("No LLMs available — heuristic mode only (Ollama not running?)")

    # ── Storage — SurrealDB (primary) then asyncpg (fallback) ─────────────
    # Priority: SURREAL_URL > DATABASE_URL > no persistence
    surreal = None
    db_pool = None

    if config.has_surreal:
        try:
            # T02: use init_storage() so get_storage() works anywhere in the codebase
            from cynic.core.storage.surreal import init_storage as _surreal_init
            surreal = await _surreal_init(
                url=config.surreal_url,
                user=config.surreal_user,
                password=config.surreal_pass,
                namespace=config.surreal_ns,
                database=config.surreal_db,
            )
            logger.info("*tail wag* SurrealDB active — primary storage (singleton set)")
        except Exception as exc:
            logger.warning("SurrealDB unavailable (%s) — falling back to asyncpg", exc)
            surreal = None

    if surreal is None:
        db_url = config.database_url
        if db_url:
            try:
                import asyncpg  # type: ignore
                db_pool = await asyncpg.create_pool(dsn=db_url, min_size=2, max_size=10)
                from cynic.core.storage.postgres import SCHEMA_SQL
                async with db_pool.acquire() as conn:
                    await conn.execute(SCHEMA_SQL)
                logger.info("PostgreSQL active — legacy storage")
            except Exception as exc:
                logger.warning("DB unavailable (%s) — running without persistence", exc)
                db_pool = None

    # ── Build kernel (always — persistence is wired after) ─────────────────
    state = build_kernel(db_pool=db_pool, registry=registry)

    # ── Warm-start from SurrealDB ───────────────────────────────────────────
    if surreal is not None:
        try:
            q_entries = await surreal.qtable.get_all()
            loaded = state.qtable.load_from_entries(q_entries)
            logger.info("Q-Table warm-start (SurrealDB): %d entries", loaded)

            residual_rows = await surreal.residuals.recent(limit=89)
            r_loaded = state.residual_detector.load_from_entries(residual_rows)
            logger.info("ResidualDetector warm-start (SurrealDB): %d points", r_loaded)

            from cynic.dogs.base import DogId
            scholar_dog = state.orchestrator.dogs.get(DogId.SCHOLAR)
            if scholar_dog is not None:
                if hasattr(scholar_dog, "set_scholar_repo"):
                    scholar_dog.set_scholar_repo(surreal.scholar)
                scholar_rows = await surreal.scholar.recent_entries(limit=89)
                s_loaded = scholar_dog.load_from_entries(scholar_rows)
                logger.info("Scholar warm-start (SurrealDB): %d entries", s_loaded)

            bench_loaded = await registry.load_benchmarks_from_surreal(surreal)
            logger.info("LLM Benchmark warm-start (SurrealDB): %d entries", bench_loaded)
            registry.set_surreal(surreal)
        except Exception as exc:
            logger.warning("SurrealDB warm-start failed (%s) — starting cold", exc)

    # ── Warm-start from asyncpg (legacy path) ──────────────────────────────
    elif db_pool is not None:
        try:
            loaded = await state.qtable.load_from_db(db_pool)
            logger.info("Q-Table warm-start: %d entries loaded", loaded)

            registry.set_db_pool(db_pool)
            bench_loaded = await registry.load_benchmarks_from_db(db_pool)
            logger.info("Benchmark warm-start: %d routing entries loaded", bench_loaded)

            state.residual_detector.set_db_pool(db_pool)
            residual_loaded = await state.residual_detector.load_from_db(db_pool)
            logger.info("ResidualDetector warm-start: %d points loaded", residual_loaded)

            from cynic.benchmark.registry import BenchmarkRegistry
            await BenchmarkRegistry.create_tables(db_pool)
            state.orchestrator.benchmark_registry = BenchmarkRegistry(db_pool)
            logger.info("BenchmarkRegistry wired: probe runs will be persisted")

            from cynic.dogs.base import DogId
            from cynic.core.storage.postgres import ScholarRepository
            scholar_dog = state.orchestrator.dogs.get(DogId.SCHOLAR)
            if scholar_dog is not None:
                if hasattr(scholar_dog, "set_scholar_repo"):
                    scholar_dog.set_scholar_repo(ScholarRepository())
                if hasattr(scholar_dog, "set_db_pool"):
                    scholar_dog.set_db_pool(db_pool)
                scholar_loaded = await scholar_dog.load_from_db(db_pool)
                logger.info("Scholar warm-start: %d buffer entries loaded", scholar_loaded)
        except Exception as exc:
            logger.warning("asyncpg warm-start failed (%s)", exc)
    else:
        logger.info("No storage configured — running without persistence")

    # ── SurrealDB event persistence (judgment + residual + sdk) ────────────
    if surreal is not None:
        async def _surreal_persist_judgment(event: Event) -> None:
            try:
                p = event.payload or {}
                jid = p.get("judgment_id")
                if jid:
                    await surreal.judgments.save(p)
                    # Also keep Q-Table in sync
                    sk = p.get("state_key", "")
                    verdict = p.get("verdict", "")
                    q_score = float(p.get("q_score", 0.0))
                    if sk and verdict:
                        await surreal.qtable.update(sk, verdict, q_score / 100.0)
            except Exception:
                pass

        async def _surreal_persist_residual(event: Event) -> None:
            try:
                p = event.payload or {}
                if p.get("judgment_id"):
                    await surreal.residuals.append(p)
            except Exception:
                pass

        get_core_bus().on(CoreEvent.JUDGMENT_CREATED, _surreal_persist_judgment)
        get_core_bus().on(CoreEvent.RESIDUAL_HIGH, _surreal_persist_residual)
        logger.info("SurrealDB persistence wired (JUDGMENT_CREATED + RESIDUAL_HIGH)")

    # ── EventBusBridge — wire 3 buses together ────────────────────────────
    from cynic.core.event_bus import create_default_bridge
    _bridge = create_default_bridge()
    _bridge.start()
    logger.info("EventBusBridge active: %d rules", len(_bridge._rules))

    set_state(state)
    await restore_state(state)  # γ2 + γ4: EScore + session context (cross-crash)
    state.scheduler.start()

    # ── AutoBenchmark — periodic LLM probe every 55 min (T09) ────────────
    from cynic.act.auto_benchmark import AutoBenchmark
    state.auto_benchmark = AutoBenchmark(registry)
    state.auto_benchmark.start()

    # ── ClaudeCodeRunner — CYNIC spawns Claude Code autonomously ──────────
    # CYNIC is the BRAIN. Claude Code is the HANDS.
    # When ACT_REQUESTED fires (via /ws/stream or internal DECIDE), CYNIC
    # spawns `claude --sdk-url ws://localhost:PORT/ws/sdk` as a subprocess.
    # No human needed to launch Claude Code.
    from cynic.act.runner import ClaudeCodeRunner
    state.runner = ClaudeCodeRunner(
        bus=get_core_bus(),
        sessions_registry=_sdk_sessions,
        port=config.port,
    )
    logger.info("*sniff* ClaudeCodeRunner wired (port=%d)", config.port)

    # Wire ACT_REQUESTED → runner.execute() → ACT_COMPLETED (T30: closed loop)
    # Routing:
    #   bash / git_read action types → UniversalActuator (direct, no LLM)
    #   all other types              → ClaudeCodeRunner (LLM-mediated)
    _DIRECT_ACTION_TYPES = frozenset({"bash", "git_read"})

    async def _on_act_requested(event: Event) -> None:
        action_id   = event.payload.get("action_id", "")
        action_type = (event.payload.get("action_type") or "").lower()

        async def _execute_and_emit() -> None:
            if action_type in _DIRECT_ACTION_TYPES:
                # Direct execution via UniversalActuator (no LLM overhead)
                act_result = await state.universal_actuator.dispatch(event.payload)
                await get_core_bus().emit(Event.typed(
                    CoreEvent.ACT_COMPLETED,
                    ActCompletedPayload(
                        action_id=action_id,
                        success=act_result.success,
                        cost_usd=0.0,
                        exec_id="",
                        error=act_result.error,
                    ),
                    source="act_requested_handler",
                ))
                return

            if state.runner is None:
                return
            prompt = event.payload.get("action", "")
            if not prompt:
                return
            cwd = event.payload.get("target")
            if not isinstance(cwd, str):
                cwd = None
            result = await state.runner.execute(prompt, cwd=cwd)
            await get_core_bus().emit(Event.typed(
                CoreEvent.ACT_COMPLETED,
                ActCompletedPayload(
                    action_id=action_id,
                    success=result.get("success", False),
                    cost_usd=result.get("cost_usd", 0.0),
                    exec_id=result.get("exec_id", ""),
                    error=result.get("error", ""),
                ),
                source="act_requested_handler",
            ))

        asyncio.create_task(_execute_and_emit())

    get_core_bus().on(CoreEvent.ACT_REQUESTED, _on_act_requested)

    # Wire DECISION_MADE → runner.execute() (auto-ACT with budget guard)
    # Throttle: max 1 auto-act per 60s — prevents runaway judgment → act loops.
    # Reality filter: only CODE + CYNIC act on code/self (not MARKET/SOLANA).
    _act_guard = {"last_t": 0.0}
    _AUTO_ACT_INTERVAL_S = 60.0
    _AUTO_ACT_REALITIES = frozenset({"CODE", "CYNIC"})

    async def _on_decision_made(event: Event) -> None:
        if state.runner is None:
            return
        p = event.payload or {}
        if p.get("reality", "") not in _AUTO_ACT_REALITIES:
            return
        action_prompt = p.get("action_prompt", "")
        if not action_prompt:
            return
        now = time.time()
        elapsed = now - _act_guard["last_t"]
        if elapsed < _AUTO_ACT_INTERVAL_S:
            logger.debug(
                "Auto-ACT throttled: %.0fs since last (min=%.0fs)",
                elapsed, _AUTO_ACT_INTERVAL_S,
            )
            return
        _act_guard["last_t"] = now

        decision_state_key = p.get("state_key", "")
        decision_action = p.get("recommended_action", "WAG")
        decision_judgment_id = p.get("judgment_id", "")

        # ── Mark matching ActionProposer entry as AUTO_EXECUTED ───────────
        # Links the auto-ACT path back to the proposal queue (closes the loop).
        for _a in state.action_proposer.pending():
            if _a.judgment_id == decision_judgment_id:
                state.action_proposer.mark_auto_executed(_a.action_id)
                logger.debug("Auto-ACT: marked proposal %s AUTO_EXECUTED", _a.action_id)
                break

        # ── Chain depth enforcement: F(4)=3 max depth ─────────────────────
        # Prevents INVESTIGATE→REFACTOR→INVESTIGATE runaway loops.
        # If the proposal derived from this judgment is at depth >= 3, emit
        # EMERGENCE_DETECTED (pattern: ACTION_CHAIN_MAX_DEPTH) and skip execution.
        _chain_depth = 0
        for _ca in state.action_proposer.all_actions():
            if _ca.judgment_id == decision_judgment_id:
                _chain_depth = _ca.chain_depth
                break

        if _chain_depth >= 3:  # fibonacci(4) = 3
            await get_core_bus().emit(Event.typed(
                CoreEvent.EMERGENCE_DETECTED,
                _EmergenceDetectedPayload(
                    pattern_type="ACTION_CHAIN_MAX_DEPTH",
                    chain_depth=_chain_depth,
                ),
                source="action_chain",
            ))
            logger.warning(
                "*GROWL* ACTION_CHAIN_MAX_DEPTH: chain blocked at depth %d — EMERGENCE_DETECTED",
                _chain_depth,
            )
            return  # skip execution

        logger.info(
            "*ears perk* Auto-ACT fired: reality=%s verdict=%s state=%s",
            p.get("reality"), decision_action, decision_state_key,
        )

        async def _act_and_learn() -> None:
            """Execute via CC, then feed result back to the DECISION state Q-entry."""
            result = await state.runner.execute(action_prompt, timeout=120.0)
            is_success = result.get("success", False)
            cost = float(result.get("cost_usd", 0.0))
            # Reward: compute_reward reuses the SDK telemetry formula
            reward = compute_reward(not is_success, 0, cost)
            from cynic.learning.qlearning import LearningSignal as _LS
            state.qtable.update(_LS(
                state_key=decision_state_key,
                action=decision_action,
                reward=reward,
                judgment_id=result.get("exec_id", ""),
                loop_name="ACT_RESULT",
            ))
            logger.info(
                "*%s* ACT_RESULT → Q[%s][%s]=updated reward=%.3f (success=%s cost=$%.4f)",
                "tail wag" if is_success else "GROWL",
                decision_state_key, decision_action, reward, is_success, cost,
            )
            # T30: emit ACT_COMPLETED so _on_act_completed can close the full loop
            await get_core_bus().emit(Event.typed(
                CoreEvent.ACT_COMPLETED,
                ActCompletedPayload(
                    action_id="",
                    success=is_success,
                    cost_usd=cost,
                    exec_id=result.get("exec_id", ""),
                    error=result.get("error", ""),
                ),
                source="auto_act_and_learn",
            ))

        asyncio.create_task(_act_and_learn())

    get_core_bus().on(CoreEvent.DECISION_MADE, _on_decision_made)
    logger.info(
        "*sniff* Auto-ACT loop wired (throttle=%.0fs, realities=%s)",
        _AUTO_ACT_INTERVAL_S, sorted(_AUTO_ACT_REALITIES),
    )

    # ── consciousness.json — unified metathinking output (Ring 3) ─────────
    # Written after every JUDGMENT_CREATED (throttled to F(7)=13s).
    # Aggregates the full organism state: KernelMirror + LLM routing + guidance.
    # Read by: TUI dashboard, Claude Code hooks, external monitoring tools.
    _CONSCIOUSNESS_PATH = os.path.join(
        os.path.expanduser("~"), ".cynic", "consciousness.json"
    )
    _consciousness_last_write = [0.0]
    _CONSCIOUSNESS_MIN_INTERVAL_S = 13.0  # F(7) — throttle disk writes

    async def _write_consciousness(event: Event) -> None:
        now = time.time()
        if now - _consciousness_last_write[0] < _CONSCIOUSNESS_MIN_INTERVAL_S:
            return
        _consciousness_last_write[0] = now
        try:
            snap = state.kernel_mirror.snapshot(state)
            diff = state.kernel_mirror.diff(snap)

            # ── KernelMirror → CONSCIOUSNESS signal ───────────────────────────
            # Ring 3 self-reflection: when the organism sees itself clearly
            # (overall_health ≥ WAG_MIN = 61.8), signal CONSCIOUSNESS (A10).
            # This closes the self-model loop: perception → snapshot → awareness.
            health = snap.get("overall_health", 0.0)
            if health >= WAG_MIN and state.axiom_monitor is not None:
                new_state_m = state.axiom_monitor.signal("CONSCIOUSNESS")
                if new_state_m == "ACTIVE":
                    await get_core_bus().emit(Event.typed(
                        CoreEvent.AXIOM_ACTIVATED,
                        _AxiomActivatedPayload(
                            axiom="CONSCIOUSNESS",
                            maturity=state.axiom_monitor.get_maturity("CONSCIOUSNESS"),
                            trigger="MIRROR_SNAPSHOT",
                            overall_health=round(health, 1),
                        ),
                        source="mirror",
                    ))
                    logger.info(
                        "MIRROR: overall_health=%.1f >= WAG_MIN → CONSCIOUSNESS ACTIVE",
                        health,
                    )

            payload: dict = {
                "timestamp": round(now, 3),
                "uptime_s": round(state.uptime_s, 1),
                "mirror": snap,
                "diff": diff,
            }
            if state.llm_router is not None:
                payload["llm_routing"] = state.llm_router.stats()
            os.makedirs(os.path.dirname(_CONSCIOUSNESS_PATH), exist_ok=True)
            with open(_CONSCIOUSNESS_PATH, "w", encoding="utf-8") as fh:
                json.dump(payload, fh)
        except Exception as _exc:
            logger.debug("consciousness.json write skipped: %s", _exc)

    get_core_bus().on(CoreEvent.JUDGMENT_CREATED, _write_consciousness)
    logger.info("consciousness.json writer armed (throttle=%.0fs)", _CONSCIOUSNESS_MIN_INTERVAL_S)

    llm_count = len(registry.get_available())
    logger.info(
        "*tail wag* CYNIC kernel alive — %d dogs, %d LLMs, learning active, scheduler running, runner ready",
        len(state.dogs), llm_count,
    )

    yield

    # Shutdown
    logger.info("*yawn* CYNIC kernel shutting down...")
    get_core_bus().off(CoreEvent.DECISION_MADE, _on_decision_made)
    _bridge.stop()
    await state.scheduler.stop()
    if state.auto_benchmark is not None:
        await state.auto_benchmark.stop()
    state.learning_loop.stop()
    if state.runner is not None:
        await state.runner.shutdown()
    if surreal is not None:
        from cynic.core.storage.surreal import close_storage as _surreal_close
        await _surreal_close()
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

# Serve static files (dashboard.html) — only if directory exists
import pathlib as _pathlib
_static_dir = _pathlib.Path(__file__).parent.parent / "static"
if _static_dir.is_dir():
    app.mount("/static", StaticFiles(directory=str(_static_dir)), name="static")


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


# ── Register all routers ───────────────────────────────────────────────────
app.include_router(router_core)
app.include_router(router_actions)
app.include_router(router_health)
app.include_router(router_sdk)
app.include_router(router_act)
app.include_router(router_ws)
