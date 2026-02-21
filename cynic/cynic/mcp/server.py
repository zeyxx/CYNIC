"""
MCP Server — Lightweight async bootstrap bridge.

Exposes CYNIC organism state to Claude Code via three endpoints:
- POST /observe  — Get current CYNIC state
- POST /act      — Execute Claude Code action
- POST /learn    — Human feedback → Q-Table learning

Uses aiohttp (stdlib-adjacent) for minimal dependencies.
Connects to ServiceStateRegistry (Component 1 Tier 1 nervous system).
"""
from __future__ import annotations

import asyncio
import json
import logging
import time
from typing import Any, Callable

try:
    from aiohttp import web
except ImportError:
    web = None  # type: ignore[assignment]

from cynic.mcp.models import (
    ObserveRequest, ObserveResponse, ActRequest, ActResponse, LearnRequest, LearnResponse,
    ErrorResponse, ComponentHealthSnapshot, RegistrySnapshot,
)
from cynic.mcp.utils import setup_logging
from cynic.nervous import ComponentSnapshot as NervousComponentSnapshot

logger = logging.getLogger("cynic.mcp.server")


class MCPServer:
    """MCP bootstrap bridge — connects Claude Code ↔ CYNIC organism."""

    def __init__(
        self,
        port: int = 8766,
        host: str = "127.0.0.1",
        get_state_fn: Optional[Callable] = None,  # () -> CynicOrganism
    ):
        """
        Initialize MCP server.

        Args:
            port: Listen port
            host: Listen host
            get_state_fn: Callable that returns current CynicOrganism state
        """
        self.port = port
        self.host = host
        self.get_state_fn = get_state_fn
        self.app: web.Optional[Application] = None
        self.runner: web.Optional[AppRunner] = None

    async def start(self) -> None:
        """Start the MCP server."""
        if web is None:
            raise RuntimeError("aiohttp not installed. Install with: pip install aiohttp")

        self.app = web.Application()
        self._setup_routes()

        self.runner = web.AppRunner(self.app)
        await self.runner.setup()
        site = web.TCPSite(self.runner, self.host, self.port)
        await site.start()

        logger.info(f"MCP Server listening on {self.host}:{self.port}")

    async def stop(self) -> None:
        """Stop the MCP server."""
        if self.runner:
            await self.runner.cleanup()
            logger.info("MCP Server stopped")

    def _setup_routes(self) -> None:
        """Register endpoint handlers."""
        assert self.app is not None
        self.app.router.add_post("/observe", self._handle_observe)
        self.app.router.add_post("/act", self._handle_act)
        self.app.router.add_post("/learn", self._handle_learn)
        self.app.router.add_get("/health", self._handle_health)
        logger.info("Routes registered: /observe, /act, /learn, /health")

    # ═══════════════════════════════════════════════════════════════════════
    # ENDPOINT HANDLERS
    # ═══════════════════════════════════════════════════════════════════════

    async def _handle_observe(self, request: web.Request) -> web.Response:
        """
        GET current CYNIC state.

        Request: ObserveRequest
        Response: ObserveResponse
        """
        try:
            body = await request.json()
            req = ObserveRequest(**body)

            state = self.get_state_fn() if self.get_state_fn else None
            if not state:
                return self._json_response(
                    ErrorResponse(
                        timestamp=time.time(),
                        status="error",
                        error="CYNIC state not initialized",
                    ),
                    status=503,
                )

            # Get ServiceStateRegistry snapshot (Component 1) — async call
            snapshot = await state.service_registry.snapshot()
            registry_snap = RegistrySnapshot(
                timestamp=snapshot.timestamp_ms / 1000.0,  # Convert ms to seconds
                components=[
                    ComponentHealthSnapshot(
                        name=comp.name,
                        status=comp.status.name,  # HealthStatus -> string
                        timestamp=comp.last_update_ms / 1000.0,  # Convert ms to seconds
                        judgment_count=0,  # Not tracked in ComponentSnapshot
                        last_judgment_id=comp.last_judgment_id,
                    )
                    for comp in snapshot.components.values()
                ],
                health_summary={
                    "HEALTHY": snapshot.healthy_count,
                    "DEGRADED": snapshot.degraded_count,
                    "STALLED": snapshot.stalled_count,
                    "FAILED": snapshot.failed_count,
                },
                total_components=snapshot.total_components,
            )

            resp = ObserveResponse(
                timestamp=time.time(),
                registry_snapshot=registry_snap,
                recent_judgments=[] if not req.include_judgments else [],
                recent_events=[] if not req.include_events else [],
                status="ok",
            )
            return self._json_response(resp)
        except Exception as exc:
            logger.exception("observe error")
            return self._json_response(
                ErrorResponse(
                    timestamp=time.time(),
                    status="error",
                    error=str(exc),
                ),
                status=400,
            )

    async def _handle_act(self, request: web.Request) -> web.Response:
        """
        Execute Claude Code action (REAL execution, not mock).

        Wire the request to ClaudeCodeRunner, which spawns
        `claude --sdk-url ws://localhost:PORT/ws/sdk` and executes the task.

        Request: ActRequest
        Response: ActResponse
        """
        try:
            body = await request.json()
            req = ActRequest(**body)

            state = self.get_state_fn() if self.get_state_fn else None
            if not state:
                return self._json_response(
                    ErrorResponse(
                        timestamp=time.time(),
                        status="error",
                        error="CYNIC state not initialized",
                    ),
                    status=503,
                )

            # ── REAL EXECUTION: Use ClaudeCodeRunner ──────────────────────────
            if not state.runner:
                return self._json_response(
                    ErrorResponse(
                        timestamp=time.time(),
                        status="error",
                        error="ClaudeCodeRunner not initialized in organism",
                    ),
                    status=503,
                )

            start_time = time.time()

            # Execute via real Claude Code subprocess
            result_dict = await state.runner.execute(
                prompt=req.action.prompt,
                cwd=None,  # Use default
                model=req.action.context.get("model", "claude-haiku-4-5-20251001"),
                timeout=req.timeout_s,
            )

            exec_time = time.time() - start_time

            # Parse runner result and convert to ActResult
            success = result_dict.get("success", False)
            error = result_dict.get("error") if not success else None

            # On success: session_id, cost_usd, exec_id available
            # Summarize output
            output_summary = ""
            if success:
                output_summary = f"Session: {result_dict.get('session_id', 'unknown')}, Cost: ${result_dict.get('cost_usd', 0):.4f}"
            else:
                output_summary = error or "Unknown error"

            from cynic.mcp.models import ActResult
            result = ActResult(
                action_id=req.action.action_id,
                success=success,
                output=output_summary,
                error=error,
                execution_time_s=exec_time,
                learning_signal={
                    "session_id": result_dict.get("session_id"),
                    "cost_usd": result_dict.get("cost_usd"),
                    "exec_id": result_dict.get("exec_id"),
                } if success else None,
            )

            resp = ActResponse(
                timestamp=time.time(),
                result=result,
                status="ok" if success else "error",
            )
            return self._json_response(resp)
        except asyncio.TimeoutError:
            logger.warning("act: Claude Code execution timed out")
            return self._json_response(
                ErrorResponse(
                    timestamp=time.time(),
                    status="error",
                    error=f"Action execution timed out after {req.timeout_s}s",
                ),
                status=408,
            )
        except Exception as exc:
            logger.exception("act error")
            return self._json_response(
                ErrorResponse(
                    timestamp=time.time(),
                    status="error",
                    error=str(exc),
                ),
                status=400,
            )

    async def _handle_learn(self, request: web.Request) -> web.Response:
        """
        Human feedback → Q-Table learning (REAL, not mock).

        Takes feedback signal (rating: -1 to +1) and updates Q-Table.
        Rating is normalized to reward [0, 1] for TD(0) update.

        Request: LearnRequest (signal with judgment_id, rating, comment)
        Response: LearnResponse (with new_q_score and applied learning rate)
        """
        try:
            body = await request.json()
            req = LearnRequest(**body)

            state = self.get_state_fn() if self.get_state_fn else None
            if not state:
                return self._json_response(
                    ErrorResponse(
                        timestamp=time.time(),
                        status="error",
                        error="CYNIC state not initialized",
                    ),
                    status=503,
                )

            # ── REAL Q-TABLE UPDATE ─────────────────────────────────────────
            if not state.qtable:
                return self._json_response(
                    ErrorResponse(
                        timestamp=time.time(),
                        status="error",
                        error="QTable not initialized in organism",
                    ),
                    status=503,
                )

            # Look up judgment by ID to get state_key and action
            # For now, extract from signal context (will be passed by client)
            # Rating: -1 to +1 → Reward: 0 to 1 (normalize via (rating + 1) / 2)
            reward = (req.signal.rating + 1.0) / 2.0

            # Build learning signal
            from cynic.learning.qlearning import LearningSignal
            learning_signal = LearningSignal(
                state_key=req.signal.judgment_id,  # TODO: map judgment_id → state_key
                action="WAG",  # TODO: map judgment_id → action (verdict)
                reward=reward,
                judgment_id=req.signal.judgment_id,
                timestamp=time.time(),
            )

            # Update Q-Table
            entry = state.qtable.update(learning_signal)

            from cynic.mcp.models import LearnResult
            result = LearnResult(
                judgment_id=req.signal.judgment_id,
                qtable_updated=req.update_qtable,
                new_q_score=entry.q_value,  # Real Q-value from TD(0)
                learning_rate_applied=state.qtable._alpha,  # Real learning rate
            )

            resp = LearnResponse(
                timestamp=time.time(),
                result=result,
                status="ok",
            )
            return self._json_response(resp)
        except Exception as exc:
            logger.exception("learn error")
            return self._json_response(
                ErrorResponse(
                    timestamp=time.time(),
                    status="error",
                    error=str(exc),
                ),
                status=400,
            )

    async def _handle_health(self, request: web.Request) -> web.Response:
        """Health check endpoint."""
        return self._json_response({"status": "ok", "timestamp": time.time()})

    # ═══════════════════════════════════════════════════════════════════════
    # RESPONSE HELPERS
    # ═══════════════════════════════════════════════════════════════════════

    def _json_response(
        self,
        data: Any,
        status: int = 200,
    ) -> web.Response:
        """Serialize Pydantic model to JSON response."""
        if hasattr(data, "model_dump"):
            # Pydantic v2
            json_str = json.dumps(data.model_dump(), default=str)
        else:
            json_str = json.dumps(data, default=str)
        return web.Response(text=json_str, status=status, content_type="application/json")


async def run_mcp_server(
    port: int = 8766,
    get_state_fn: Optional[Callable] = None,
) -> MCPServer:
    """
    Start MCP server and return it for lifecycle management.

    Usage:
        server = await run_mcp_server(port=8766, get_state_fn=get_state)
        try:
            await asyncio.Event().wait()  # Run forever
        finally:
            await server.stop()
    """
    setup_logging("cynic.mcp")
    server = MCPServer(port=port, get_state_fn=get_state_fn)
    await server.start()
    return server
