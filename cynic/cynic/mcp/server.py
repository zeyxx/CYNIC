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

logger = logging.getLogger("cynic.mcp.server")


class MCPServer:
    """MCP bootstrap bridge — connects Claude Code ↔ CYNIC organism."""

    def __init__(
        self,
        port: int = 8766,
        host: str = "127.0.0.1",
        get_state_fn: Callable | None = None,  # () -> CynicOrganism
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
        self.app: web.Application | None = None
        self.runner: web.AppRunner | None = None

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

            # Get ServiceStateRegistry snapshot (Component 1)
            snapshot = state.service_registry.snapshot()
            registry_snap = RegistrySnapshot(
                timestamp=snapshot.timestamp,
                components=[
                    ComponentHealthSnapshot(
                        name=comp.name,
                        status=comp.status.name,
                        timestamp=comp.timestamp,
                        judgment_count=comp.judgment_count,
                        last_judgment_id=comp.last_judgment_id,
                    )
                    for comp in snapshot.components
                ],
                health_summary=snapshot.health_counts,
                total_components=len(snapshot.components),
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
        Execute Claude Code action.

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

            # TODO: Wire to ClaudeCodeRunner when available
            # For now, return placeholder
            start_time = time.time()
            exec_time = time.time() - start_time

            from cynic.mcp.models import ActResult
            result = ActResult(
                action_id=req.action.action_id,
                success=True,
                output="Action execution not yet wired",
                error=None,
                execution_time_s=exec_time,
            )

            resp = ActResponse(
                timestamp=time.time(),
                result=result,
                status="ok",
            )
            return self._json_response(resp)
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
        Human feedback → Q-Table learning.

        Request: LearnRequest
        Response: LearnResponse
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

            # TODO: Wire to Q-Table learning when available
            # For now, return placeholder
            from cynic.mcp.models import LearnResult
            result = LearnResult(
                judgment_id=req.signal.judgment_id,
                qtable_updated=req.update_qtable,
                new_q_score=None,
                learning_rate_applied=0.038,
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
    get_state_fn: Callable | None = None,
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
