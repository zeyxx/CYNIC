"""MCPRouter — JSON-RPC protocol translator for the MCP bridge.

Translates MCP protocol messages into MCPBridge operations:
- tools/list  -> returns registered tools (synchronous)
- tools/call  -> invokes bridge.handle_call (async)

On each tools/call, emits an MCP_TOOL_CALLED event to the CORE bus
with full protocol context (request_id, source) before dispatching
to the bridge.  This connects the WebSocket protocol layer to the
organism's event-driven architecture.
"""

from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass, field
from typing import Any, Optional

from cynic.api.state import get_app_container
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.event_bus import CoreEvent, Event, get_core_bus
from cynic.core.judgment import Cell
from cynic.mcp.service import MCPBridge, MCPTool

logger = logging.getLogger(__name__)

# JSON-RPC 2.0 error codes
_METHOD_NOT_FOUND = -32601
_INTERNAL_ERROR = -32603


@dataclass
class _CallMetadata:
    """Metadata for a single concurrent MCP tool call."""

    call_id: int
    tool_name: str
    started_at: float
    task: Optional[asyncio.Task] = None
    timeout: float = 30.0
    status: str = "running"  # running, completed, failed, timeout
    error: Optional[str] = None
    duration: float = 0.0


def _jsonrpc_result(msg_id: Any, result: dict[str, Any]) -> dict[str, Any]:
    """Build a JSON-RPC 2.0 success response."""
    return {"jsonrpc": "2.0", "id": msg_id, "result": result}


def _jsonrpc_error(msg_id: Any, code: int, message: str) -> dict[str, Any]:
    """Build a JSON-RPC 2.0 error response."""
    return {"jsonrpc": "2.0", "id": msg_id, "error": {"code": code, "message": message}}


class MCPRouter:
    """Translate MCP JSON-RPC messages into MCPBridge operations.

    Handles two MCP methods:
        tools/list  - Return all registered tools (sync).
        tools/call  - Call a tool via the bridge (async).

    Usage:
        router = MCPRouter()
        # Synchronous for tools/list:
        response = router.handle_message({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})
        # Asynchronous for tools/call:
        response = await router.handle_message_async(msg)
    """

    def __init__(self) -> None:
        self.bridge = MCPBridge(bus_name="CORE")
        self._setup_default_tools()
        # Concurrent call tracking
        self.active_calls: dict[int, _CallMetadata] = {}
        self._call_id_counter = 0
        self._call_lock = asyncio.Lock()

    def _get_next_call_id(self) -> int:
        """Get the next unique call ID (non-async, called inside async lock)."""
        self._call_id_counter += 1
        return self._call_id_counter

    def get_active_calls(self) -> dict[int, dict[str, Any]]:
        """Get information about all active calls."""
        return {
            call_id: {
                "call_id": meta.call_id,
                "tool": meta.tool_name,
                "started_at": meta.started_at,
                "elapsed": time.time() - meta.started_at,
                "status": meta.status,
                "timeout": meta.timeout,
            }
            for call_id, meta in self.active_calls.items()
        }

    def _setup_default_tools(self) -> None:
        """Register the two default tools: ask_cynic, observe_cynic."""
        self.bridge.register_tool(MCPTool(
            name="ask_cynic",
            description="Ask CYNIC to judge, analyze, or respond to a prompt.",
            input_schema={
                "type": "object",
                "properties": {
                    "prompt": {"type": "string", "description": "The question or request for CYNIC."},
                },
                "required": ["prompt"],
            },
        ))
        self.bridge.register_tool(MCPTool(
            name="observe_cynic",
            description="Observe CYNIC's current conscious state and organism health.",
            input_schema={
                "type": "object",
                "properties": {
                    "aspect": {
                        "type": "string",
                        "description": "Which aspect to observe (e.g. 'health', 'dogs', 'memory').",
                    },
                },
            },
        ))

    def handle_message(self, message: dict[str, Any]) -> dict[str, Any]:
        """Handle a synchronous MCP message (tools/list or unknown methods).

        For tools/call, use handle_message_async instead.
        """
        msg_id = message.get("id")
        method = message.get("method")

        if method == "tools/list":
            return self._handle_tools_list(msg_id)

        if method == "tools/call":
            return _jsonrpc_error(
                msg_id,
                _INTERNAL_ERROR,
                "tools/call requires async — use handle_message_async()",
            )

        return _jsonrpc_error(
            msg_id,
            _METHOD_NOT_FOUND,
            f"Unknown method: {method!r}",
        )

    async def handle_message_async(self, message: dict[str, Any]) -> dict[str, Any]:
        """Handle any MCP message, including async tools/call."""
        msg_id = message.get("id")
        method = message.get("method")

        if method == "tools/list":
            return self._handle_tools_list(msg_id)

        if method == "tools/call":
            return await self._handle_tools_call(message, msg_id)

        return _jsonrpc_error(
            msg_id,
            _METHOD_NOT_FOUND,
            f"Unknown method: {method!r}",
        )

    def _handle_tools_list(self, msg_id: Any) -> dict[str, Any]:
        """Return all registered tools in MCP format."""
        tools = [
            {
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.input_schema,
            }
            for tool in self.bridge.tools.values()
        ]
        return _jsonrpc_result(msg_id, {"tools": tools})

    async def _handle_tools_call(
        self, message: dict[str, Any], msg_id: Any
    ) -> dict[str, Any]:
        """Route a tools/call message to appropriate handler.

        Special routing for ask_cynic and observe_cynic (via orchestrator).
        Other tools route to bridge.handle_call.

        Emits an MCP_TOOL_CALLED event to the CORE bus before dispatching.
        Tracks concurrent calls with unique call IDs.
        """
        params = message.get("params", {})
        tool_name = params.get("name", "")
        arguments = params.get("arguments", {})

        # Get next call ID (with async-safe counter)
        async with self._call_lock:
            call_id = self._get_next_call_id()

        # Create call metadata
        call_metadata = _CallMetadata(
            call_id=call_id,
            tool_name=tool_name,
            started_at=time.time(),
        )

        # Track the call
        self.active_calls[call_id] = call_metadata

        logger.debug(f"[CALL {call_id}] Starting {tool_name}")

        try:
            # Get current task
            call_metadata.task = asyncio.current_task()

            # Emit protocol-level event BEFORE dispatch.
            # Even if handler fails, the organism sees the attempt.
            event = Event.typed(
                CoreEvent.MCP_TOOL_CALLED,
                payload={
                    "tool_name": tool_name,
                    "arguments": arguments,
                    "request_id": msg_id,
                    "source": "websocket",
                    "call_id": call_id,
                },
                source="mcp_router",
            )
            await get_core_bus().emit(event)

            try:
                # Route ask_cynic to orchestrator (returns Judgment result)
                if tool_name == "ask_cynic":
                    result = await self._handle_ask_cynic(msg_id, arguments)

                # Route observe_cynic to state snapshot
                elif tool_name == "observe_cynic":
                    result = await self._handle_observe_cynic(msg_id, arguments)

                # All other tools via bridge
                else:
                    tool_result = await self.bridge.handle_call(tool_name, arguments)
                    result = _jsonrpc_result(msg_id, tool_result)

                # Mark as completed
                call_metadata.status = "completed"
                call_metadata.duration = time.time() - call_metadata.started_at
                logger.debug(f"[CALL {call_id}] {tool_name} completed in {call_metadata.duration:.2f}s")

                return result

            except (KeyError, RuntimeError) as exc:
                call_metadata.status = "failed"
                call_metadata.error = str(exc)
                call_metadata.duration = time.time() - call_metadata.started_at
                logger.warning(f"[CALL {call_id}] MCP tools/call failed: {exc}")
                return _jsonrpc_error(msg_id, _INTERNAL_ERROR, str(exc))

        except Exception as exc:
            call_metadata.status = "failed"
            call_metadata.error = str(exc)
            call_metadata.duration = time.time() - call_metadata.started_at
            logger.exception(f"[CALL {call_id}] Unexpected error in tools/call handler")
            return _jsonrpc_error(msg_id, _INTERNAL_ERROR, str(exc))

        finally:
            # Clean up from active calls after completion
            self.active_calls.pop(call_id, None)
            logger.debug(f"[CALL {call_id}] Cleaned up call metadata")

    async def _handle_ask_cynic(
        self, msg_id: Any, arguments: dict[str, Any]
    ) -> dict[str, Any]:
        """
        Handle ask_cynic tool — wire to orchestrator for real judgment.

        Builds a Cell from the prompt, calls orchestrator.run() with REFLEX level,
        and returns the Judgment result (verdict, q_score, confidence, judgment_id).

        On any error, emits MCP_TOOL_CALLED and returns fallback (fire-and-forget ack).
        """
        try:
            prompt = arguments.get("prompt", "")
            if not prompt:
                return _jsonrpc_error(msg_id, _INTERNAL_ERROR, "prompt required")

            # Get app container (orchestrator)
            container = get_app_container()
            if not container or not container.orchestrator:
                logger.debug("ask_cynic: orchestrator not available, falling back to fire-and-forget")
                return _jsonrpc_result(msg_id, {"status": "emitted"})

            # Build Cell for CODE/JUDGE analysis
            cell = Cell(
                reality="CODE",
                analysis="JUDGE",
                content=prompt,
                context=prompt,  # For LLM scoring context
                lod=0,  # Pattern level (fast)
                budget_usd=0.001,  # Small budget for MCP queries
            )

            # Run orchestrator at REFLEX level (instant, no LLM)
            try:
                judgment = await container.orchestrator.run(
                    cell,
                    level=ConsciousnessLevel.REFLEX,
                    budget_usd=0.001,
                )

                # Return Judgment fields as result
                return _jsonrpc_result(msg_id, {
                    "verdict": judgment.verdict,
                    "q_score": round(judgment.q_score, 3),
                    "confidence": round(judgment.confidence, 3),
                    "judgment_id": judgment.judgment_id,
                    "reality": judgment.cell.reality,
                    "cost_usd": round(judgment.cost_usd, 6),
                })
            except Exception as orch_err:
                logger.warning("orchestrator.run() failed for ask_cynic: %s", orch_err)
                # Fallback: emit event and return ack
                return _jsonrpc_result(msg_id, {
                    "status": "emitted",
                    "error": str(orch_err),
                })
        except Exception as exc:
            logger.exception("ask_cynic handler error")
            return _jsonrpc_error(msg_id, _INTERNAL_ERROR, str(exc))

    async def _handle_observe_cynic(
        self, msg_id: Any, arguments: dict[str, Any]
    ) -> dict[str, Any]:
        """
        Handle observe_cynic tool — return current organism state snapshot.

        Returns consciousness state, Q-Table stats, and health metrics.
        Helps Claude Code understand CYNIC's current condition.
        """
        try:
            aspect = arguments.get("aspect", "health")

            # Get app container
            container = get_app_container()
            if not container:
                return _jsonrpc_result(msg_id, {"status": "alive", "aspect": aspect})

            result = {"aspect": aspect, "timestamp": time.time()}

            # Snapshot consciousness state
            if container.consciousness:
                try:
                    result["consciousness"] = container.consciousness.to_dict()
                except Exception as e:
                    logger.debug("Failed to snapshot consciousness: %s", e)
                    result["consciousness"] = {"error": str(e)}

            # Snapshot Q-Table stats
            if container.qtable:
                try:
                    stats = container.qtable.stats()
                    result["qtable"] = stats
                except Exception as e:
                    logger.debug("Failed to get Q-Table stats: %s", e)
                    result["qtable"] = {"error": str(e)}

            # Snapshot service registry if available
            if hasattr(container, "service_registry") and container.service_registry:
                try:
                    snapshot = await container.service_registry.snapshot()
                    result["registry"] = {
                        "total_components": snapshot.total_components,
                        "healthy": snapshot.healthy_count,
                        "degraded": snapshot.degraded_count,
                        "stalled": snapshot.stalled_count,
                        "failed": snapshot.failed_count,
                    }
                except Exception as e:
                    logger.debug("Failed to snapshot registry: %s", e)
                    result["registry"] = {"error": str(e)}

            return _jsonrpc_result(msg_id, result)
        except Exception as exc:
            logger.exception("observe_cynic handler error")
            return _jsonrpc_error(msg_id, _INTERNAL_ERROR, str(exc))
