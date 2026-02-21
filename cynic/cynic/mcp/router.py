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

import logging
from typing import Any

from cynic.core.event_bus import CoreEvent, Event, get_core_bus
from cynic.mcp.service import MCPBridge, MCPTool

logger = logging.getLogger(__name__)

# JSON-RPC 2.0 error codes
_METHOD_NOT_FOUND = -32601
_INTERNAL_ERROR = -32603


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
        """Route a tools/call message to bridge.handle_call.

        Emits an MCP_TOOL_CALLED event to the CORE bus before dispatching
        to the bridge.  The event carries full protocol context (request_id,
        source) that the bridge-level event does not include.
        """
        params = message.get("params", {})
        tool_name = params.get("name", "")
        arguments = params.get("arguments", {})

        # Emit protocol-level event BEFORE bridge dispatch.
        # Even if bridge.handle_call fails, the organism sees the attempt.
        event = Event.typed(
            CoreEvent.MCP_TOOL_CALLED,
            payload={
                "tool_name": tool_name,
                "arguments": arguments,
                "request_id": msg_id,
                "source": "websocket",
            },
            source="mcp_router",
        )
        await get_core_bus().emit(event)

        try:
            result = await self.bridge.handle_call(tool_name, arguments)
            return _jsonrpc_result(msg_id, result)
        except (KeyError, RuntimeError) as exc:
            logger.warning("MCP tools/call failed: %s", exc)
            return _jsonrpc_error(msg_id, _INTERNAL_ERROR, str(exc))
