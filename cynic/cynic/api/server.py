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
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from fastapi import APIRouter, FastAPI, HTTPException, Query, Request, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse, JSONResponse
from fastapi.staticfiles import StaticFiles

from cynic.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.judgment import Cell, Judgment
from cynic.core.phi import PHI, MAX_CONFIDENCE, WAG_MIN
from cynic.learning.qlearning import LearningSignal

from cynic.act.telemetry import (
    SessionTelemetry as SDKTelemetry,
    TelemetryStore,
    classify_task,
    compute_reward,
    estimate_complexity,
)
from cynic.api.models import (
    JudgeRequest, JudgeResponse,
    PerceiveRequest, PerceiveResponse,
    LearnRequest, LearnResponse,
    FeedbackRequest, FeedbackResponse,
    PolicyResponse,
    HealthResponse,
    StatsResponse,
)
from cynic.api.state import build_kernel, set_state, get_state, restore_state
from cynic.core.storage.postgres import JudgmentRepository, SDKSessionRepository as _SDKSessionRepo

logger = logging.getLogger("cynic.api.server")

_boot_time = time.time()

# Path for JSONL session persistence (survives restarts)
_SDK_SESSIONS_JSONL = os.path.join(os.path.expanduser("~"), ".cynic", "sdk_sessions.jsonl")


def _append_sdk_session_jsonl(record: SDKTelemetry) -> None:
    """Append one completed SDK session to JSONL file (fire-and-forget)."""
    try:
        import dataclasses as _dc
        os.makedirs(os.path.dirname(_SDK_SESSIONS_JSONL), exist_ok=True)
        with open(_SDK_SESSIONS_JSONL, "a", encoding="utf-8") as fh:
            fh.write(json.dumps(_dc.asdict(record)) + "\n")
    except Exception as exc:
        logger.debug("sdk_sessions.jsonl append skipped: %s", exc)


# Path for social signals — SocialWatcher reads; human interactions write.
_SOCIAL_SIGNAL_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "social.json")
# Rolling cap: F(8)=21 signals max (prevent unbounded growth)
_SOCIAL_SIGNAL_CAP = 21


def _append_social_signal(
    source: str,
    sentiment: float,
    volume: float,
    topic: str,
    signal_type: str,
) -> None:
    """
    Append one social signal to ~/.cynic/social.json (fire-and-forget).

    SocialWatcher reads this file every 89s and submits SOCIAL×PERCEIVE
    cells. The read=False flag ensures each signal is processed exactly once.

    Closes the Social loop: human interactions → sentiment → SocialWatcher →
    MICRO judgment → QTable + SYMBIOSIS axiom signal.
    """
    try:
        import time as _t
        os.makedirs(os.path.dirname(_SOCIAL_SIGNAL_PATH), exist_ok=True)
        signal = {
            "ts": _t.time(),
            "source": source,
            "sentiment": round(max(-1.0, min(1.0, sentiment)), 3),
            "volume": round(max(0.0, min(100.0, volume)), 1),
            "topic": topic,
            "signal_type": signal_type,
            "read": False,
        }
        if os.path.exists(_SOCIAL_SIGNAL_PATH):
            with open(_SOCIAL_SIGNAL_PATH, encoding="utf-8") as fh:
                existing = json.load(fh)
            if not isinstance(existing, list):
                existing = [existing]
        else:
            existing = []
        existing.append(signal)
        if len(existing) > _SOCIAL_SIGNAL_CAP:
            existing = existing[-_SOCIAL_SIGNAL_CAP:]
        with open(_SOCIAL_SIGNAL_PATH, "w", encoding="utf-8") as fh:
            json.dump(existing, fh)
    except Exception as exc:
        logger.debug("social.json append skipped: %s", exc)


# ════════════════════════════════════════════════════════════════════════════
# SDK SESSION REGISTRY
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class SDKSession:
    """
    Tracks one active Claude Code --sdk-url WebSocket session.

    Each session is a running `claude --sdk-url ws://localhost:PORT/ws/sdk`
    process. CYNIC is the server; Claude Code is the client (the HANDS).

    Telemetry fields (prefixed _) are populated during the session and
    used to build a SessionTelemetry record when the result message arrives.
    """
    session_id: str
    ws: Any                                  # WebSocket — typed as Any to avoid circular import issues
    cwd: str = ""
    tools: List[str] = field(default_factory=list)
    model: str = "unknown"
    claude_code_version: str = ""
    cli_session_id: str = ""                 # Claude's internal session ID — used for --resume
    total_cost_usd: float = 0.0
    connected_at: float = field(default_factory=time.time)
    log: List[Dict[str, Any]] = field(default_factory=list)

    # ── Telemetry (populated during session, consumed at result) ──────────
    _task_prompt: str = ""                           # last task sent to this session
    _tool_sequence: List[str] = field(default_factory=list)  # ordered tool names
    _result_text: str = ""                           # Claude's result description
    _input_tokens: int = 0                           # accumulated from assistant msgs
    _output_tokens: int = 0

    def record(self, msg_type: str, data: Dict[str, Any]) -> None:
        self.log.append({"type": msg_type, "data": data, "ts": time.time()})

    def to_dict(self) -> Dict[str, Any]:
        return {
            "session_id": self.session_id,
            "cwd": self.cwd,
            "model": self.model,
            "claude_code_version": self.claude_code_version,
            "cli_session_id": self.cli_session_id,
            "tools": self.tools,
            "total_cost_usd": round(self.total_cost_usd, 6),
            "connected_at": self.connected_at,
            "events": len(self.log),
            "tool_count": len(self._tool_sequence),
        }


# Process-level registry of active SDK sessions
_sdk_sessions: Dict[str, SDKSession] = {}


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
    _instance_meta_path = os.path.join(os.path.expanduser("~"), ".cynic", "instance.json")
    try:
        os.makedirs(os.path.dirname(_instance_meta_path), exist_ok=True)
        with open(_instance_meta_path, "w", encoding="utf-8") as _fh:
            json.dump({
                "instance_id": _instance_id,
                "port": int(os.getenv("PORT", 8765)),
                "started_at": time.time(),
            }, _fh, indent=2)
    except Exception as _exc:
        logger.debug("instance.json write failed: %s", _exc)

    # MCP auto-config for Cursor/Windsurf — non-destructive (never overwrites existing)
    _mcp_port = int(os.getenv("PORT", 8765))
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

    # Local inference env vars (optional — requires llama-cpp-python installed)
    models_dir = os.getenv("CYNIC_MODELS_DIR")          # e.g. ~/.cynic/models
    llama_gpu_layers = int(os.getenv("LLAMA_CPP_GPU_LAYERS", "-1"))  # -1=iGPU, 0=CPU
    llama_threads = int(os.getenv("LLAMA_CPP_THREADS", "8"))

    # Ollama parallel hint — must be set before `ollama serve` (Ollama env var)
    if os.getenv("OLLAMA_NUM_PARALLEL") is None:
        logger.info(
            "Tip: set OLLAMA_NUM_PARALLEL=4 before starting Ollama for "
            "parallel MCTS (7 calls → 2 batches instead of 7 sequential)"
        )

    discovered = await registry.discover(
        ollama_url=os.getenv("OLLAMA_URL", "http://localhost:11434"),
        claude_api_key=os.getenv("ANTHROPIC_API_KEY"),
        gemini_api_key=os.getenv("GOOGLE_API_KEY"),
        models_dir=models_dir,
        llama_gpu_layers=llama_gpu_layers,
        llama_threads=llama_threads,
    )
    if discovered:
        logger.info("*ears perk* LLMs discovered: %s", discovered)
    else:
        logger.info("No LLMs available — heuristic mode only (Ollama not running?)")

    # ── Storage — SurrealDB (primary) then asyncpg (fallback) ─────────────
    # Priority: SURREAL_URL > DATABASE_URL > no persistence
    surreal = None
    db_pool = None

    surreal_url = os.getenv("SURREAL_URL")
    if surreal_url:
        try:
            # T02: use init_storage() so get_storage() works anywhere in the codebase
            from cynic.core.storage.surreal import init_storage as _surreal_init
            surreal = await _surreal_init(
                url=surreal_url,
                user=os.getenv("SURREAL_USER", "root"),
                password=os.getenv("SURREAL_PASS", "cynic_phi_618"),
                namespace=os.getenv("SURREAL_NS", "cynic"),
                database=os.getenv("SURREAL_DB", "cynic"),
            )
            logger.info("*tail wag* SurrealDB active — primary storage (singleton set)")
        except Exception as exc:
            logger.warning("SurrealDB unavailable (%s) — falling back to asyncpg", exc)
            surreal = None

    if surreal is None:
        db_url = os.getenv("DATABASE_URL")
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
            scholar_dog = state.orchestrator.dogs.get(DogId.SCHOLAR)
            if scholar_dog is not None and hasattr(scholar_dog, "set_db_pool"):
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
    runner_port = int(os.getenv("PORT", 8765))
    state.runner = ClaudeCodeRunner(
        bus=get_core_bus(),
        sessions_registry=_sdk_sessions,
        port=runner_port,
    )
    logger.info("*sniff* ClaudeCodeRunner wired (port=%d)", runner_port)

    # Wire ACT_REQUESTED → runner.execute() → ACT_COMPLETED (T30: closed loop)
    async def _on_act_requested(event: Event) -> None:
        if state.runner is None:
            return
        prompt = event.payload.get("action", "")
        if not prompt:
            return
        cwd = event.payload.get("target")
        if not isinstance(cwd, str):
            cwd = None
        action_id = event.payload.get("action_id", "")

        async def _execute_and_emit() -> None:
            result = await state.runner.execute(prompt, cwd=cwd)
            await get_core_bus().emit(Event(
                type=CoreEvent.ACT_COMPLETED,
                payload={
                    "action_id": action_id,
                    "success":   result.get("success", False),
                    "cost_usd":  result.get("cost_usd", 0.0),
                    "exec_id":   result.get("exec_id", ""),
                    "error":     result.get("error", ""),
                },
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
            await get_core_bus().emit(Event(
                type=CoreEvent.EMERGENCE_DETECTED,
                source="action_chain",
                payload={"pattern_type": "ACTION_CHAIN_MAX_DEPTH", "chain_depth": _chain_depth},
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
            await get_core_bus().emit(Event(
                type=CoreEvent.ACT_COMPLETED,
                payload={
                    "action_id": "",   # auto-ACT has no explicit action_id
                    "success":  is_success,
                    "cost_usd": cost,
                    "exec_id":  result.get("exec_id", ""),
                    "error":    result.get("error", ""),
                },
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
                    await get_core_bus().emit(Event(
                        type=CoreEvent.AXIOM_ACTIVATED,
                        payload={
                            "axiom":          "CONSCIOUSNESS",
                            "maturity":       state.axiom_monitor.get_maturity("CONSCIOUSNESS"),
                            "trigger":        "MIRROR_SNAPSHOT",
                            "overall_health": round(health, 1),
                        },
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

# ── APIRouters ─────────────────────────────────────────────────────────────
# Routes are grouped by domain. Each router is registered via
# app.include_router() at the bottom of this file.
#
#   router_core    — judge · perceive · learn · feedback · policy
#   router_actions — proposed-actions · self-probes
#   router_health  — health · stats · introspect · axioms · lod · mirror · consciousness
#   router_sdk     — ws/sdk · sdk/*
#   router_act     — act/execute · act/telemetry
#   router_ws      — ws/stream · ws/events
# ───────────────────────────────────────────────────────────────────────────
router_core    = APIRouter(tags=["core"])
router_actions = APIRouter(tags=["actions"])
router_health  = APIRouter(tags=["health"])
router_sdk     = APIRouter(tags=["sdk"])
router_act     = APIRouter(tags=["act"])
router_ws      = APIRouter(tags=["ws"])

# ── Dashboard convenience route ────────────────────────────────────────────
@router_health.get("/dashboard", include_in_schema=False)
async def dashboard() -> FileResponse:
    """Serve the live CYNIC kernel dashboard (connects to /ws/stream)."""
    path = _static_dir / "dashboard.html"
    if not path.is_file():
        raise HTTPException(status_code=404, detail="dashboard.html not found")
    return FileResponse(str(path), media_type="text/html")


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

@router_core.post("/judge", response_model=JudgeResponse)
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


_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")


_judgment_repo = JudgmentRepository()
_sdk_session_repo = _SDKSessionRepo()


def _persist_judgment(judgment: Judgment) -> None:
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

@router_core.post("/perceive", response_model=PerceiveResponse)
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
            await get_core_bus().emit(Event(
                type=CoreEvent.AXIOM_ACTIVATED,
                payload={"axiom": "SYMBIOSIS", "maturity": state.axiom_monitor.get_maturity("SYMBIOSIS")},
                source="user_feedback",
            ))
    except Exception:
        pass

    # USER_FEEDBACK bus event — makes human rating visible organism-wide.
    # Handlers in state.py react (EScore JUDGE update for agent:cynic).
    # This is separate from the inline SYMBIOSIS signal above — bus event lets
    # other components (SelfProber, future handlers) react without touching this code.
    try:
        await get_core_bus().emit(Event(
            type=CoreEvent.USER_FEEDBACK,
            payload={
                "rating":       req.rating,
                "reward":       reward,
                "sentiment":    (req.rating - 3) / 2.0,
                "state_key":    last["state_key"],
                "action":       last["action"],
                "judgment_id":  last.get("judgment_id", ""),
            },
            source="feedback_endpoint",
        ))
    except Exception:
        pass

    # USER_CORRECTION: rating=1 = user explicitly says CYNIC was WRONG.
    # Separate from USER_FEEDBACK (covers all ratings) — this is the active correction.
    # ANTIFRAGILITY: being corrected and learning from it = growth through stress.
    if req.rating == 1:
        try:
            await get_core_bus().emit(Event(
                type=CoreEvent.USER_CORRECTION,
                payload={
                    "rating":      req.rating,
                    "state_key":   last["state_key"],
                    "action":      last["action"],
                    "judgment_id": last.get("judgment_id", ""),
                },
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
# GET /actions — list proposed actions
# POST /actions/{id}/accept — approve a proposed action
# POST /actions/{id}/reject — decline a proposed action
# ════════════════════════════════════════════════════════════════════════════

@router_actions.get("/actions")
async def list_actions(
    status: Optional[str] = Query(default=None, description="Filter by status: PENDING/ACCEPTED/REJECTED/AUTO_EXECUTED"),
) -> Dict[str, Any]:
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
async def accept_action(action_id: str) -> Dict[str, Any]:
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
            await get_core_bus().emit(Event(
                type=CoreEvent.AXIOM_ACTIVATED,
                payload={"axiom": "ANTIFRAGILITY", "maturity": state.axiom_monitor.get_maturity("ANTIFRAGILITY")},
                source="action_accept",
            ))
    except Exception:
        pass

    # ── L1 closure: accepted → fire ACT_REQUESTED → runner executes ──────
    # This closes the Machine→Actions loop: accept = authorize execution.
    if action.prompt:
        await get_core_bus().emit(Event(
            type=CoreEvent.ACT_REQUESTED,
            payload={"action": action.prompt, "target": None, "action_id": action.action_id},
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
async def reject_action(action_id: str) -> Dict[str, Any]:
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
) -> Dict[str, Any]:
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
) -> Dict[str, Any]:
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
async def dismiss_probe(probe_id: str) -> Dict[str, Any]:
    """Dismiss a self-improvement proposal — marks it DISMISSED."""
    state = get_state()
    proposal = state.self_prober.dismiss(probe_id)
    if proposal is None:
        raise HTTPException(status_code=404, detail=f"Probe {probe_id} not found")
    return {"dismissed": True, "proposal": proposal.to_dict()}


@router_actions.post("/self-probes/{probe_id}/apply")
async def apply_probe(probe_id: str) -> Dict[str, Any]:
    """Mark a self-improvement proposal as APPLIED."""
    state = get_state()
    proposal = state.self_prober.apply(probe_id)
    if proposal is None:
        raise HTTPException(status_code=404, detail=f"Probe {probe_id} not found")
    logger.info("*tail wag* Self-probe %s APPLIED", probe_id)
    return {"applied": True, "proposal": proposal.to_dict()}


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


# ════════════════════════════════════════════════════════════════════════════
# GET /health
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/health", response_model=HealthResponse)
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

    sched_stats = state.scheduler.stats()

    # Determine status
    status = "alive"
    if not state.learning_loop._active:
        status = "degraded"

    # T02: check SurrealDB singleton status (no I/O — just checks if initialized)
    _storage_status: Dict[str, Any] = {}
    try:
        from cynic.core.storage.surreal import get_storage as _get_storage
        _get_storage()  # raises RuntimeError if not initialized
        _storage_status["surreal"] = "connected"
    except RuntimeError:
        _storage_status["surreal"] = "disconnected"
    except Exception:
        _storage_status["surreal"] = "error"

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
        scheduler=sched_stats,
        llm_adapters=[a.adapter_id for a in __import__("cynic.llm.adapter", fromlist=["get_registry"]).get_registry().get_available()],
        judgments_total=judge_stats["judgments_total"],
        phi=PHI,
        storage=_storage_status,
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /stats
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/stats", response_model=StatsResponse)
async def stats() -> StatsResponse:
    """Detailed kernel metrics — everything CYNIC knows about itself."""
    state = get_state()

    return StatsResponse(
        judgments=state.orchestrator.stats(),
        learning=state.qtable.stats(),
        top_states=state.qtable.top_states(n=10),
        consciousness=get_consciousness().to_dict(),
        compressor=state.context_compressor.stats(),
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /  (root — for quick sanity check)
# ════════════════════════════════════════════════════════════════════════════

# ════════════════════════════════════════════════════════════════════════════
# GET /introspect  (MetaCognition — composant 9/9, self-model)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/introspect")
async def introspect() -> Dict[str, Any]:
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
        # δ1+δ2+γ4 intelligence layer
        "emergent_axioms": state.axiom_monitor.stats(),
        "lod": state.lod_controller.status(),
        "escore_top": state.escore_tracker.top_entities(n=5),
        "message": "*sniff* Je me lis moi-même. Le chien qui se connaît.",
    }


# ════════════════════════════════════════════════════════════════════════════
# GET /axioms  (δ1 Emergent Axiom Dashboard)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/axioms")
async def axioms() -> Dict[str, Any]:
    """
    Emergent Axiom dashboard — A6-A9 activation status.

    Returns live maturity scores and tier for the 4 emergent axioms:
      A6. AUTONOMY     — Dogs coordinate without human approval
      A7. SYMBIOSIS    — Human×Machine mutual value creation
      A8. EMERGENCE    — Patterns beyond core axioms
      A9. ANTIFRAGILITY — System improves under chaos
    """
    state = get_state()
    return state.axiom_monitor.dashboard()


# ════════════════════════════════════════════════════════════════════════════
# GET /lod  (δ2 Survival LOD status)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/lod")
async def lod() -> Dict[str, Any]:
    """
    Survival LOD status — current graceful degradation level.

    LOD 0 FULL:      All Dogs + LLM + all consciousness levels
    LOD 1 REDUCED:   Skip slow Dogs, L2 MICRO max
    LOD 2 EMERGENCY: REFLEX only, no LLM
    LOD 3 MINIMAL:   GUARDIAN only, survival mode
    """
    state = get_state()
    return state.lod_controller.status()


@router_health.get("/account/stats")
async def account_stats() -> Dict[str, Any]:
    """
    AccountAgent step-6 ledger — cost tracking + budget enforcement.

    Returns per-reality and per-dog cost breakdown, budget remaining,
    and BUDGET_WARNING / BUDGET_EXHAUSTED event emission status.

    Step 6 of the 7-step cycle: PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE
    """
    state = get_state()
    if state.account_agent is None:
        return {"error": "AccountAgent not initialized", "total_cost_usd": 0.0}
    return state.account_agent.stats()


@router_health.get("/decide/stats")
async def decide_stats() -> Dict[str, Any]:
    """
    DecideAgent Ring-2 stats — MCTS decision counts.

    Shows decisions_made (BARK/GROWL auto-decided via NestedMCTS) and
    skipped (WAG/HOWL or low-confidence judgments not escalated).
    """
    state = get_state()
    if state.decide_agent is None:
        return {"decisions_made": 0, "skipped": 0}
    return state.decide_agent.stats()


@router_health.get("/sage/stats")
async def sage_stats() -> Dict[str, Any]:
    """
    SAGE Dog temporal MCTS activation stats.

    Shows heuristic vs LLM (temporal) judgment counts.
    llm_activation_rate > 0 → Temporal MCTS is firing (Ollama available).
    llm_activation_rate == 0 → Heuristic-only mode (Ollama unavailable).
    """
    from cynic.dogs.base import DogId
    state = get_state()
    orch = state.orchestrator
    sage = orch.dogs.get(DogId.SAGE) if orch and hasattr(orch, "dogs") else None
    if sage is None:
        return {"available": False, "heuristic_count": 0, "llm_count": 0}
    heuristic = getattr(sage, "_heuristic_count", 0)
    llm = getattr(sage, "_llm_count", 0)
    total = heuristic + llm
    return {
        "available": True,
        "heuristic_count": heuristic,
        "llm_count": llm,
        "total_judgments": total,
        "llm_activation_rate": round(llm / total, 3) if total > 0 else 0.0,
        "temporal_mcts_active": llm > 0,
    }


@router_health.get("/residual/stats")
async def residual_stats() -> Dict[str, Any]:
    """
    ResidualDetector stats — residual variance history + pattern detection (T04).

    observations > 0  → warm-start succeeded (SurrealDB loaded history on boot)
    anomaly_rate > 0  → some judgments had high residual variance (≥38.2%)
    patterns_detected → EMERGENCE patterns found (SPIKE / RISING / STABLE_HIGH)
    """
    state = get_state()
    return state.residual_detector.stats()


@router_health.get("/llm/benchmarks")
async def llm_benchmarks() -> Dict[str, Any]:
    """
    LLM Benchmark routing matrix — per-(dog, task_type, llm_id) perf history (T05).

    Persisted to SurrealDB after each update_benchmark() call.
    Warmed from SurrealDB on boot so routing survives restarts.
    Used by LLMRouter to select the best LLM for each Dog × Task combination.
    """
    from cynic.llm.adapter import get_registry as _get_registry
    reg = _get_registry()
    matrix = [
        {
            "dog_id":          dog_id,
            "task_type":       task_type,
            "llm_id":          llm_id,
            "quality_score":   round(r.quality_score, 2),
            "speed_score":     round(r.speed_score, 3),
            "cost_score":      round(r.cost_score, 3),
            "composite_score": round(r.composite_score, 3),
            "error_rate":      round(r.error_rate, 3),
            "sample_count":    r.sample_count,
        }
        for (dog_id, task_type, llm_id), r in reg._benchmarks.items()
    ]
    return {"count": len(matrix), "matrix": matrix}


@router_health.get("/auto-benchmark/stats")
async def auto_benchmark_stats() -> Dict[str, Any]:
    """AutoBenchmark probe stats — interval, runs, enabled flag (T09)."""
    state = get_state()
    if state.auto_benchmark is None:
        return {"enabled": False, "runs": 0, "interval_s": 0}
    return state.auto_benchmark.stats()


@router_health.post("/auto-benchmark/run")
async def auto_benchmark_run() -> Dict[str, Any]:
    """Trigger an immediate AutoBenchmark round (T09)."""
    state = get_state()
    if state.auto_benchmark is None:
        return {"completed": 0, "message": "auto_benchmark not initialised"}
    completed = await state.auto_benchmark.run_once()
    return {"completed": completed}


@router_health.get("/mirror")
async def mirror() -> Dict[str, Any]:
    """
    KernelMirror — Ring 3 unified self-reflection snapshot.

    Aggregates all subsystem stats into a single response:
      - qtable: 7×7×7 matrix coverage + learning stats
      - axioms: A6-A11 tier + maturity scores
      - lod: current LOD + health dimensions
      - account: budget ledger + cost by reality/dog
      - escore: per-dog reputation (7 dimensions)
      - residual: spike/stable/rising signal counts
      - sage: temporal MCTS vs heuristic ratio
      - dogs: judgment counts + latency profiles
      - overall_health: geometric mean [0, 100]
      - tier: BARK/GROWL/WAG/HOWL

    Use for dashboards, health checks, and CONSCIOUSNESS signal evaluation.
    """
    state = get_state()
    snap = state.kernel_mirror.snapshot(state)
    snap["diff"] = state.kernel_mirror.diff(snap)
    return snap


# ════════════════════════════════════════════════════════════════════════════
# GET /consciousness  (unified metathinking output)
# ════════════════════════════════════════════════════════════════════════════

_CONSCIOUSNESS_FILE = os.path.join(os.path.expanduser("~"), ".cynic", "consciousness.json")


@router_health.get("/consciousness")
async def consciousness() -> Dict[str, Any]:
    """
    Unified metathinking output — the organism's complete cognitive state.

    Returns the contents of ~/.cynic/consciousness.json if available,
    otherwise falls back to a live mirror snapshot.

    Updated by the kernel after every JUDGMENT_CREATED (throttled to F(7)=13s).
    """
    # Try reading pre-written file first (avoids re-computing snapshot)
    try:
        import pathlib
        p = pathlib.Path(_CONSCIOUSNESS_FILE)
        if p.exists() and (time.time() - p.stat().st_mtime) < 60.0:
            with p.open("r", encoding="utf-8") as fh:
                return json.load(fh)
    except Exception:
        pass

    # Fallback: live snapshot
    state = get_state()
    snap = state.kernel_mirror.snapshot(state)
    payload: Dict[str, Any] = {
        "timestamp": round(time.time(), 3),
        "uptime_s": round(state.uptime_s, 1),
        "mirror": snap,
        "diff": state.kernel_mirror.diff(snap),
    }
    if state.llm_router is not None:
        payload["llm_routing"] = state.llm_router.stats()
    return payload


# ════════════════════════════════════════════════════════════════════════════
# WS /ws/stream  (real-time event stream)
# ════════════════════════════════════════════════════════════════════════════

@router_ws.websocket("/ws/stream")
async def ws_stream(websocket: WebSocket) -> None:
    """
    WebSocket stream — bidirectional real-time kernel events.

    Events streamed (server -> client):
      JUDGMENT_CREATED  — every judgment result
      LEARNING_EVENT    — Q-table updates
      META_CYCLE        — periodic evolution ticks

    Messages received (client -> server):
      {"type": "ACT", "action": "...", "target": "..."} — emitted as ACT_REQUESTED
      {"type": "ping"}  — responds with {"type": "pong", "ts": ...}
      Any other type    — ignored silently

    Protocol:
      connect -> {"type": "connected", "phi": 1.618...}
      event   -> {"type": <CoreEvent.name>, "payload": {...}, "ts": <float>}
      ping    -> {"type": "ping", "ts": <float>}  (30s keepalive)

    Client disconnect -> clean unsubscribe from all events.
    Queue overflow (>100 buffered events) -> events dropped silently.
    """
    await websocket.accept()
    bus = get_core_bus()
    queue: asyncio.Queue = asyncio.Queue(maxsize=100)

    async def on_event(event: Event) -> None:
        try:
            queue.put_nowait({
                "type": event.event_type.name if hasattr(event.event_type, "name") else str(event.event_type),
                "payload": event.payload,
                "ts": time.time(),
            })
        except asyncio.QueueFull:
            pass  # Drop silently — client is slow, kernel must not block

    stream_events = [
        CoreEvent.JUDGMENT_CREATED,
        CoreEvent.LEARNING_EVENT,
        CoreEvent.META_CYCLE,
        CoreEvent.DECISION_MADE,
    ]
    for ev_type in stream_events:
        bus.on(ev_type, on_event)

    async def _emit_loop() -> None:
        """Send queued events to the WebSocket client."""
        while True:
            try:
                msg = await asyncio.wait_for(queue.get(), timeout=30.0)
                await websocket.send_json(msg)
            except asyncio.TimeoutError:
                # Keepalive ping — proves connection is alive
                await websocket.send_json({"type": "ping", "ts": time.time()})

    async def _receive_loop() -> None:
        """Receive client messages and route them to the bus or respond directly."""
        while True:
            data = await websocket.receive_json()
            msg_type = data.get("type", "")
            if msg_type == "ping":
                await websocket.send_json({"type": "pong", "ts": time.time()})
            elif msg_type == "ACT":
                await bus.emit(Event(
                    type=CoreEvent.ACT_REQUESTED,
                    payload={"action": data.get("action", ""), "target": data.get("target", "")},
                    source="ws_client",
                ))
            # Any other type: ignored silently

    try:
        await websocket.send_json({"type": "connected", "ts": time.time(), "phi": PHI})
        await asyncio.gather(_emit_loop(), _receive_loop())
    except WebSocketDisconnect:
        pass
    finally:
        for ev_type in stream_events:
            bus.off(ev_type, on_event)


# ════════════════════════════════════════════════════════════════════════════
# WS /ws/events  (read-only all-events stream with client-side filter)
# ════════════════════════════════════════════════════════════════════════════

@router_ws.websocket("/ws/events")
async def ws_events(websocket: WebSocket) -> None:
    """
    Read-only WebSocket — streams ALL CoreEvents with client-side filtering.

    Protocol:
      connect  → {"type": "connected", "ts": ..., "phi": 1.618, "all_events": [...]}
      subscribe → client sends {"type": "subscribe", "events": ["JUDGMENT_CREATED", ...]}
                 → server only sends matching events (default: all)
      event    → {"type": <event_name>, "payload": {...}, "source": str, "ts": float}
      ping     → client sends {"type": "ping"} → server responds {"type": "pong", "ts": ...}

    Client disconnect → clean unsubscribe from all events.
    Queue overflow (>100 buffered events) → events dropped silently.
    """
    await websocket.accept()
    bus = get_core_bus()
    queue: asyncio.Queue = asyncio.Queue(maxsize=100)

    # All CoreEvent names → used for connected banner + subscribe validation
    all_event_names: list = [e.name for e in CoreEvent]

    # Active filter — None = all events pass; set = only matching names pass
    _active_filter: list = []  # mutable cell (empty = all events)
    _filter_lock = asyncio.Lock()

    async def on_any_event(event: Event) -> None:
        name = event.event_type.name if hasattr(event.event_type, "name") else str(event.event_type)
        async with _filter_lock:
            passes = (not _active_filter) or (name in _active_filter)
        if not passes:
            return
        try:
            queue.put_nowait({
                "type":    name,
                "payload": event.payload,
                "source":  getattr(event, "source", ""),
                "ts":      time.time(),
            })
        except asyncio.QueueFull:
            pass  # Drop silently — client is slow, kernel must not block

    # Subscribe to ALL CoreEvents
    for ev_type in CoreEvent:
        bus.on(ev_type, on_any_event)

    async def _emit_loop() -> None:
        """Send queued events to the WebSocket client."""
        while True:
            try:
                msg = await asyncio.wait_for(queue.get(), timeout=30.0)
                await websocket.send_json(msg)
            except asyncio.TimeoutError:
                await websocket.send_json({"type": "ping", "ts": time.time()})

    async def _receive_loop() -> None:
        """Receive client messages: subscribe filter or ping."""
        nonlocal _active_filter
        while True:
            data = await websocket.receive_json()
            msg_type = data.get("type", "")
            if msg_type == "ping":
                await websocket.send_json({"type": "pong", "ts": time.time()})
            elif msg_type == "subscribe":
                requested = [e for e in (data.get("events") or []) if e in all_event_names]
                async with _filter_lock:
                    _active_filter = requested
                await websocket.send_json({
                    "type":       "subscribed",
                    "events":     requested or all_event_names,
                    "filter_all": not requested,
                    "ts":         time.time(),
                })

    try:
        await websocket.send_json({
            "type":       "connected",
            "ts":         time.time(),
            "phi":        PHI,
            "all_events": all_event_names,
        })
        await asyncio.gather(_emit_loop(), _receive_loop())
    except WebSocketDisconnect:
        pass
    finally:
        for ev_type in CoreEvent:
            bus.off(ev_type, on_any_event)


# ════════════════════════════════════════════════════════════════════════════
# WS /ws/sdk  (Claude Code --sdk-url server — CYNIC is the BRAIN)
# ════════════════════════════════════════════════════════════════════════════

@router_sdk.websocket("/ws/sdk")
async def ws_sdk(websocket: WebSocket) -> None:
    """
    Claude Code SDK WebSocket server.

    Claude Code connects here as a HEADLESS CLIENT when launched with:
      claude --sdk-url ws://localhost:8765/ws/sdk \\
             --print --output-format stream-json --input-format stream-json

    CYNIC is the SERVER (the BRAIN). Claude Code is the CLIENT (the HANDS).
    CYNIC intercepts every tool use, judges it with GUARDIAN (REFLEX level),
    and learns from every result to build the Q-Table from real usage.

    Message flow (NDJSON — each line is one JSON object):

      CLI → CYNIC: system/init        → record session metadata
      CLI → CYNIC: can_use_tool       → CYNIC judges → control_response allow/deny
      CLI → CYNIC: assistant          → record to session log
      CLI → CYNIC: result             → record cost, emit SDK_RESULT_RECEIVED
      CLI → CYNIC: keep_alive         → respond keep_alive

      CYNIC → CLI: keep_alive         → heartbeat
      CYNIC → CLI: user               → send task (via POST /sdk/task)
      CYNIC → CLI: control_response   → approve/deny/modify tool use
      CYNIC → CLI: set_model          → switch Sonnet/Haiku mid-session

    Bootstrap loop:
      Phase 1: CYNIC intercepts all tool calls → builds Q-Table from real Claude sessions
      Phase 2: Q-Table confidence rises → CYNIC routes simple tasks to Ollama
      Phase 3: 80%+ tasks → Ollama ($0 cost). Claude only for novel tasks.
    """
    await websocket.accept()
    state = get_state()
    bus = get_core_bus()

    session_id = str(uuid.uuid4())
    session = SDKSession(session_id=session_id, ws=websocket)
    _sdk_sessions[session_id] = session

    logger.info("*ears perk* SDK session connected: %s", session_id)

    async def _send(msg: Dict[str, Any]) -> None:
        """Send one NDJSON message to Claude Code."""
        await websocket.send_text(json.dumps(msg) + "\n")

    async def _judge_tool(tool_name: str, tool_input: Dict[str, Any]) -> str:
        """
        Fast REFLEX judgment on a tool use request.
        Returns "BARK"/"GROWL"/"WAG"/"HOWL".
        """
        from cynic.core.judgment import Cell, infer_time_dim
        _tool_content = f"{tool_name}: {json.dumps(tool_input)[:400]}"
        _tool_ctx = f"SDK tool use — session {session_id[:8]}"
        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            time_dim=infer_time_dim(_tool_content, _tool_ctx, "JUDGE"),
            content=_tool_content,
            context=_tool_ctx,
            lod=0,
            budget_usd=0.0005,
        )
        try:
            judgment = await state.orchestrator.run(
                cell, level=ConsciousnessLevel.REFLEX
            )
            return judgment.verdict
        except Exception as exc:
            logger.warning("SDK tool judgment error: %s", exc)
            return "WAG"  # Safe default: allow on error

    try:
        while True:
            raw = await websocket.receive_text()

            # NDJSON: each frame may contain one or more \n-terminated objects
            for line in raw.splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    msg = json.loads(line)
                except json.JSONDecodeError:
                    logger.debug("SDK invalid JSON: %r", line[:100])
                    continue

                msg_type = msg.get("type", "")
                msg_subtype = msg.get("subtype", "")

                # ── system/init ──────────────────────────────────────────────
                if msg_type == "system" and msg_subtype == "init":
                    session.cwd = msg.get("cwd", "")
                    session.tools = msg.get("tools", [])
                    session.model = msg.get("model", "unknown")
                    session.claude_code_version = msg.get("claude_code_version", "")
                    # Claude's internal session ID — persisted for --resume on restart
                    session.cli_session_id = msg.get("session_id", "")
                    session.record("init", {
                        "cwd": session.cwd,
                        "model": session.model,
                        "tools_count": len(session.tools),
                    })

                    logger.info(
                        "*sniff* SDK init: model=%s tools=%d cwd=%s",
                        session.model, len(session.tools), session.cwd,
                    )

                    await bus.emit(Event(
                        type=CoreEvent.SDK_SESSION_STARTED,
                        payload={
                            "session_id": session_id,
                            "cli_session_id": session.cli_session_id,
                            "model": session.model,
                            "cwd": session.cwd,
                            "tools": session.tools,
                        },
                        source="ws_sdk",
                    ))

                    # Respond with keep_alive — server is ready
                    await _send({"type": "keep_alive"})

                # ── can_use_tool (tool permission request) ───────────────────
                elif msg_type == "control_request" and msg_subtype == "can_use_tool":
                    request_id = msg.get("request_id", str(uuid.uuid4()))
                    request = msg.get("request", {})
                    tool_name = request.get("tool_name", "unknown")
                    tool_input = request.get("input", {})

                    verdict = await _judge_tool(tool_name, tool_input)

                    if verdict == "BARK":
                        # GUARDIAN blocks: deny tool use
                        deny_msg = f"*GROWL* CYNIC GUARDIAN blocked: {tool_name}"
                        response = {
                            "type": "control_response",
                            "response": {
                                "subtype": "success",
                                "request_id": request_id,
                                "response": {
                                    "behavior": "deny",
                                    "message": deny_msg,
                                },
                            },
                        }
                        logger.warning("*GROWL* SDK BLOCKED: %s", tool_name)
                    else:
                        # WAG / GROWL / HOWL → allow (GROWL logs warning)
                        if verdict == "GROWL":
                            logger.warning("*sniff* SDK WARNED: %s (Q low)", tool_name)
                        response = {
                            "type": "control_response",
                            "response": {
                                "subtype": "success",
                                "request_id": request_id,
                                "response": {
                                    "behavior": "allow",
                                    "updatedInput": tool_input,
                                },
                            },
                        }

                    await _send(response)

                    behavior = "deny" if verdict == "BARK" else "allow"
                    session._tool_sequence.append(tool_name)  # telemetry: ordered sequence
                    session.record("tool_judged", {
                        "tool": tool_name,
                        "verdict": verdict,
                        "behavior": behavior,
                    })

                    await bus.emit(Event(
                        type=CoreEvent.SDK_TOOL_JUDGED,
                        payload={
                            "session_id": session_id,
                            "tool": tool_name,
                            "verdict": verdict,
                        },
                        source="ws_sdk",
                    ))

                # ── assistant (Claude's response — streaming) ────────────────
                elif msg_type == "assistant":
                    message = msg.get("message", {})
                    usage = message.get("usage", {})
                    content = message.get("content", [])
                    text_blocks = sum(1 for b in content if b.get("type") == "text")
                    tool_blocks = sum(1 for b in content if b.get("type") == "tool_use")
                    # Accumulate tokens across all assistant messages for telemetry
                    session._input_tokens += usage.get("input_tokens", 0)
                    session._output_tokens += usage.get("output_tokens", 0)
                    session.record("assistant", {
                        "text_blocks": text_blocks,
                        "tool_blocks": tool_blocks,
                        "input_tokens": usage.get("input_tokens", 0),
                        "output_tokens": usage.get("output_tokens", 0),
                    })

                # ── result (task complete) ───────────────────────────────────
                elif msg_type == "result":
                    is_error = msg.get("is_error", False)
                    cost = float(msg.get("total_cost_usd") or 0.0)
                    duration_ms = float(msg.get("duration_ms") or 0.0)
                    result_text = msg.get("result", "")
                    result_subtype = msg.get("subtype", "unknown")
                    result_usage = msg.get("usage", {})

                    session.total_cost_usd += cost
                    session._result_text = result_text
                    # Accumulate final usage
                    session._input_tokens += result_usage.get("input_tokens", 0)
                    session._output_tokens += result_usage.get("output_tokens", 0)

                    session.record("result", {
                        "subtype": result_subtype,
                        "is_error": is_error,
                        "cost_usd": cost,
                        "result_text": result_text[:200],
                    })

                    # ── Rich Q-Learning signal (28 states vs 1 before) ───────
                    task_type = classify_task(session._task_prompt)
                    complexity = estimate_complexity(session._tool_sequence)
                    reward = compute_reward(is_error, len(session._tool_sequence), cost)
                    rich_state_key = f"SDK:{session.model}:{task_type}:{complexity}"

                    from cynic.learning.qlearning import LearningSignal as _LS
                    state.qtable.update(_LS(
                        state_key=rich_state_key,
                        action="BARK" if is_error else "HOWL",
                        reward=reward,
                        judgment_id=session_id,
                        loop_name="SDK_RESULT",
                    ))

                    # ── Quality judgment of Claude's output (REFLEX) ─────────
                    judgment_content = (
                        f"Task: {session._task_prompt[:200]}\n"
                        f"Result: {result_text[:300]}\n"
                        f"Tools: {', '.join(session._tool_sequence[:10])}\n"
                        f"Cost: ${cost:.4f} | Error: {is_error} | Type: {task_type}"
                    )
                    try:
                        from cynic.core.judgment import Cell as _Cell, infer_time_dim as _itd
                        quality_cell = _Cell(
                            reality="CODE", analysis="JUDGE",
                            time_dim=_itd(judgment_content, "", "JUDGE"),
                            content=judgment_content,
                            context=f"SDK quality — session {session_id[:8]}",
                            lod=0, budget_usd=0.001,
                        )
                        qj = await state.orchestrator.run(
                            quality_cell, level=ConsciousnessLevel.REFLEX
                        )
                        q_score = round(qj.q_score, 3)
                        verdict = qj.verdict
                        confidence = round(min(qj.confidence, MAX_CONFIDENCE), 3)
                    except Exception as _exc:
                        logger.debug("Quality judgment skipped: %s", _exc)
                        q_score, verdict, confidence = 30.0, "GROWL", 0.382

                    # ── Build and store SessionTelemetry ─────────────────────
                    tool_judgments = [e for e in session.log if e["type"] == "tool_judged"]
                    allowed = sum(1 for e in tool_judgments if e["data"]["behavior"] == "allow")
                    denied = sum(1 for e in tool_judgments if e["data"]["behavior"] == "deny")

                    telemetry_record = SDKTelemetry(
                        session_id=session_id,
                        task=session._task_prompt[:500],
                        task_type=task_type,
                        complexity=complexity,
                        model=session.model,
                        tools_sequence=session._tool_sequence.copy(),
                        tools_allowed=allowed,
                        tools_denied=denied,
                        tool_allow_rate=round(allowed / max(len(tool_judgments), 1), 3),
                        input_tokens=session._input_tokens,
                        output_tokens=session._output_tokens,
                        total_cost_usd=round(session.total_cost_usd, 6),
                        duration_s=round(duration_ms / 1000, 2),
                        is_error=is_error,
                        result_text=result_text[:500],
                        output_q_score=q_score,
                        output_verdict=verdict,
                        output_confidence=confidence,
                        state_key=rich_state_key,
                        reward=reward,
                        cli_session_id=session.cli_session_id,
                    )
                    state.telemetry_store.add(telemetry_record)

                    # ── JSONL persistence (survives restarts) ─────────────────
                    _append_sdk_session_jsonl(telemetry_record)

                    # ── L2→L1 cross-feed: BARK/error → ActionProposer ─────────
                    # Links L2 (SDK result) → L1 (action queue) automatically.
                    if is_error or verdict == "BARK":
                        await bus.emit(Event(
                            type=CoreEvent.DECISION_MADE,
                            payload={
                                "recommended_action": "BARK",
                                "judgment_id": session_id,
                                "state_key": rich_state_key,
                                "reality": "CYNIC",
                                "content_preview": (session._task_prompt or "")[:60],
                                "action_prompt": (
                                    f"SDK session {session_id[:8]} failed ({task_type}). "
                                    f"Review: {result_text[:200]}"
                                ),
                                "q_value": reward,
                            },
                            source="sdk_result",
                        ))

                    # Persist to DB (fire-and-forget, best-effort)
                    if state._pool is not None:
                        import dataclasses as _dc
                        _rec_dict = _dc.asdict(telemetry_record)
                        async def _persist_sdk_session(d=_rec_dict):
                            try:
                                await _sdk_session_repo.save(d)
                            except Exception as _e:
                                logger.debug("SDK session persist skipped: %s", _e)
                        asyncio.create_task(_persist_sdk_session())

                    await bus.emit(Event(
                        type=CoreEvent.SDK_RESULT_RECEIVED,
                        payload={
                            "session_id": session_id,
                            "is_error": is_error,
                            "cost_usd": cost,
                            "total_cost_usd": round(session.total_cost_usd, 6),
                            "reward": reward,
                            "task_type": task_type,
                            "complexity": complexity,
                            "output_verdict": verdict,
                            "output_q_score": q_score,
                        },
                        source="ws_sdk",
                    ))

                    # ── Ring 4 LLM routing suggestion ─────────────────────────
                    # After recording the result, ask the router whether the NEXT
                    # task of this type should use Haiku instead of Sonnet.
                    # route_to_local=True only once Q-Table is warm enough (PHI_INV
                    # confidence + 3 visits). Cold-start: always stays on Sonnet.
                    if state.llm_router is not None:
                        routing = state.llm_router.route(
                            rich_state_key, state.qtable, task_type, complexity
                        )
                        if routing.route_to_local:
                            logger.info(
                                "LLM_ROUTER: %s → %s (%s)",
                                rich_state_key, routing.recommended_model, routing.reason,
                            )

                    logger.info(
                        "*%s* SDK result: %s task=%s complexity=%s verdict=%s Q=%.1f cost=$%.4f",
                        "tail wag" if not is_error else "GROWL",
                        result_subtype, task_type, complexity, verdict, q_score, cost,
                    )

                # ── keep_alive ───────────────────────────────────────────────
                elif msg_type == "keep_alive":
                    await _send({"type": "keep_alive"})

                # ── everything else: log and ignore ──────────────────────────
                else:
                    logger.debug("SDK unhandled: type=%s subtype=%s", msg_type, msg_subtype)

    except WebSocketDisconnect:
        pass
    except Exception as exc:
        logger.error("SDK session error: %s", exc, exc_info=True)
    finally:
        _sdk_sessions.pop(session_id, None)
        logger.info(
            "*yawn* SDK session ended: %s — %d events, cost=$%.4f",
            session_id, len(session.log), session.total_cost_usd,
        )


# ════════════════════════════════════════════════════════════════════════════
# GET /sdk/sessions  (list active Claude Code sessions)
# ════════════════════════════════════════════════════════════════════════════

@router_sdk.get("/sdk/sessions")
async def sdk_sessions() -> Dict[str, Any]:
    """List all active Claude Code --sdk-url sessions."""
    return {
        "active": len(_sdk_sessions),
        "sessions": [s.to_dict() for s in _sdk_sessions.values()],
    }


@router_sdk.get("/sdk/routing")
async def sdk_routing() -> Dict[str, Any]:
    """
    LLM routing stats — Ring 4 Q-Table driven model selection.

    Shows how often CYNIC routes SDK tasks from Sonnet → Haiku based on
    accumulated Q-Table confidence. local_rate rises as Q-Table warms up.
    """
    state = get_state()
    if state.llm_router is None:
        return {"available": False}
    return {"available": True, **state.llm_router.stats()}


@router_sdk.get("/sdk/last-session")
async def sdk_last_session(cwd: str = "") -> Dict[str, Any]:
    """
    Return the last known cli_session_id for --resume.

    Lookup order:
      1. In-memory active sessions (current process)
      2. JSONL file (~/.cynic/sdk_sessions.jsonl) — survives restarts

    Query param: cwd (optional) — filter by working directory.
    """
    # 1. In-memory active sessions
    candidates = list(_sdk_sessions.values())
    if cwd:
        candidates = [s for s in candidates if s.cwd == cwd]
    if candidates:
        latest = max(candidates, key=lambda s: s.connected_at)
        if latest.cli_session_id:
            return {"cli_session_id": latest.cli_session_id, "found": True, "source": "memory"}

    # 2. JSONL file fallback
    try:
        jsonl_path = _pathlib.Path(_SDK_SESSIONS_JSONL)
        if jsonl_path.exists():
            last_sid = ""
            with jsonl_path.open("r", encoding="utf-8") as fh:
                for line in fh:
                    try:
                        rec = json.loads(line.strip())
                        sid = rec.get("cli_session_id", "")
                        if sid and (not cwd or rec.get("cwd", "") == cwd):
                            last_sid = sid
                    except Exception:
                        pass
            if last_sid:
                return {"cli_session_id": last_sid, "found": True, "source": "jsonl"}
    except Exception:
        pass

    return {"cli_session_id": "", "found": False, "source": "none"}


# ════════════════════════════════════════════════════════════════════════════
# POST /sdk/task  (send a task to a connected Claude Code session)
# ════════════════════════════════════════════════════════════════════════════

@router_sdk.post("/sdk/task")
async def sdk_task(body: Dict[str, Any]) -> Dict[str, Any]:
    """
    Send a task (user message) to a connected Claude Code session.

    Body: {"session_id": "...", "prompt": "...", "model": "claude-haiku-4-5"}

    If session_id is omitted, uses the most recently connected session.
    If model is provided, sends set_model before the task (model routing).
    """
    session_id = body.get("session_id")
    prompt = body.get("prompt", "")
    model_override = body.get("model")

    if not prompt:
        raise HTTPException(status_code=400, detail="prompt is required")

    # Resolve session
    session: Optional[SDKSession] = None
    if session_id:
        session = _sdk_sessions.get(session_id)
    elif _sdk_sessions:
        # Most recent session
        session = max(_sdk_sessions.values(), key=lambda s: s.connected_at)

    if session is None:
        raise HTTPException(
            status_code=404,
            detail=f"No active SDK session. Run: claude --sdk-url ws://HOST:PORT/ws/sdk --print --output-format stream-json --input-format stream-json",
        )

    # Optional model routing (e.g. switch to Haiku for cheap tasks)
    if model_override and model_override != session.model:
        await session.ws.send_text(json.dumps({
            "type": "control_response",
            "response": {
                "subtype": "success",
                "request_id": str(uuid.uuid4()),
                "response": {"subtype": "set_model", "model": model_override},
            },
        }) + "\n")

    # Capture task prompt for telemetry (last task wins — typical single-task sessions)
    session._task_prompt = prompt

    # Send user message
    msg = {
        "type": "user",
        "message": {"role": "user", "content": prompt},
        "parent_tool_use_id": None,
        "session_id": session.session_id,
    }
    await session.ws.send_text(json.dumps(msg) + "\n")
    session.record("task_sent", {"prompt": prompt[:200], "model_override": model_override})

    logger.info("*tail wag* SDK task sent to session %s: %s...", session.session_id[:8], prompt[:80])

    return {
        "sent": True,
        "session_id": session.session_id,
        "model": model_override or session.model,
        "prompt_preview": prompt[:100],
    }


# ════════════════════════════════════════════════════════════════════════════
# CYNIC → Claude context injection (L2 bidirectional loop)
# ════════════════════════════════════════════════════════════════════════════

def _enrich_prompt(prompt: str, state) -> str:
    """
    Inject CYNIC context into Claude's prompt (CYNIC→Claude direction of L2).

    Prepends a compact block with:
    - Compressed session history (≤200 tokens from ContextCompressor)
    - Best learned action from QTable for this task type
    - QTable confidence level

    Returns enriched prompt when useful context exists, raw prompt otherwise.
    Skips enrichment if compressor is empty and QTable has no data (early sessions).
    """
    task_type = classify_task(prompt)
    state_key = f"SDK:default:{task_type}:medium"

    best_action = state.qtable.exploit(state_key)
    confidence = state.qtable.confidence(state_key)

    try:
        context_summary = state.context_compressor.get_compressed_context(budget=200)
    except Exception:
        context_summary = ""

    # Skip enrichment if nothing useful yet (cold start)
    if not context_summary and confidence < 0.10:
        return prompt

    lines = ["# CYNIC Context (kernel guidance)"]
    if context_summary:
        lines.append(f"## Session history\n{context_summary}")
    if confidence >= 0.10:
        lines.append(
            f"## Learned guidance\n"
            f"Task type: {task_type} | Suggested approach: {best_action} "
            f"(confidence: {confidence:.0%})"
        )
    lines.append("---\n")
    return "\n".join(lines) + prompt


# ════════════════════════════════════════════════════════════════════════════
# POST /act/execute  (CYNIC spawns Claude Code autonomously)
# ════════════════════════════════════════════════════════════════════════════

@router_act.post("/act/execute")
async def act_execute(body: Dict[str, Any]) -> Dict[str, Any]:
    """
    CYNIC executes a task by spawning Claude Code autonomously.

    Body:
        {"prompt": "...", "cwd": "/path/to/project", "model": "claude-haiku-4-5",
         "timeout": 300}

    CYNIC launches `claude --sdk-url ws://localhost:PORT/ws/sdk` as a subprocess.
    Every tool call Claude makes is intercepted and judged by GUARDIAN.
    The result is returned when Claude's result message arrives.

    This is the ACT phase of the PERCEIVE → JUDGE → DECIDE → ACT cycle.
    No human needed — CYNIC does it entirely.
    """
    state = get_state()

    if state.runner is None:
        raise HTTPException(
            status_code=503,
            detail="ClaudeCodeRunner not initialized — kernel not started via lifespan",
        )

    prompt = body.get("prompt", "")
    if not prompt:
        raise HTTPException(status_code=400, detail="prompt is required")

    cwd = body.get("cwd")
    model = body.get("model")
    timeout = float(body.get("timeout", 300.0))

    logger.info("*ears perk* ACT requested: %s...", prompt[:80])

    enriched = _enrich_prompt(prompt, state)
    result = await state.runner.execute(enriched, cwd=cwd, model=model, timeout=timeout)

    if not result.get("success"):
        # Log failure but return structured response (not HTTP error)
        logger.warning("*GROWL* ACT failed: %s", result.get("error"))

    return {
        "success": result.get("success", False),
        "session_id": result.get("session_id"),
        "cost_usd": result.get("cost_usd", 0.0),
        "total_cost_usd": result.get("total_cost_usd", 0.0),
        "exec_id": result.get("exec_id"),
        "error": result.get("error"),
        "message": (
            f"*tail wag* Task executed (cost=${result.get('cost_usd', 0.0):.4f})"
            if result.get("success")
            else f"*GROWL* Task failed: {result.get('error')}"
        ),
    }


# ════════════════════════════════════════════════════════════════════════════
# GET /act/telemetry  (session telemetry — learning measurement layer)
# ════════════════════════════════════════════════════════════════════════════

@router_act.get("/act/telemetry")
async def act_telemetry(
    n: int = Query(default=10, ge=1, le=100),
    export: bool = Query(default=False),
) -> Dict[str, Any]:
    """
    Session telemetry — CYNIC's learning measurement layer.

    Returns aggregate stats + recent sessions for H1-H5 hypothesis testing.

    Query params:
      n=10       → return last N sessions (max 100)
      export=true → return all sessions (full JSONL export)

    Stats include:
      - count, error_rate, mean_cost, mean_reward
      - verdicts (BARK/GROWL/WAG/HOWL distribution)
      - task_types (debug/refactor/test/review/write/explain/general)
      - complexities (trivial/simple/medium/complex)

    Research use: GET /act/telemetry?export=true → download for H1-H5 analysis
    """
    state = get_state()
    store = state.telemetry_store

    result = {
        "stats": store.stats(),
        "sessions": store.export() if export else store.recent(n),
        "message": f"*sniff* {len(store)} sessions measured — φ sees all.",
    }
    return result


@router_health.get("/")
async def root() -> Dict[str, Any]:
    return {
        "name": "CYNIC Kernel",
        "version": "2.0.0",
        "status": "alive",
        "φ": PHI,
        "routes": [
            "/judge", "/perceive", "/learn", "/policy/{key}",
            "/health", "/stats", "/introspect",
            "/ws/stream", "/ws/sdk",
            "/sdk/sessions", "/sdk/task",
            "/act/execute", "/act/telemetry",
        ],
        "message": "*sniff* Le chien est là.",
    }

# ── Register all routers ───────────────────────────────────────────────────
app.include_router(router_core)
app.include_router(router_actions)
app.include_router(router_health)
app.include_router(router_sdk)
app.include_router(router_act)
app.include_router(router_ws)
