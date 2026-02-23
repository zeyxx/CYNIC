"""WebSocket endpoint for MCP protocol.

Claude Code connects here instead of stdio.
Translates JSON-RPC messages to organism events via MCPRouter.

Protocol:
  1. Client opens WebSocket at /ws/mcp
  2. Client sends JSON-RPC 2.0 messages (tools/list, tools/call)
  3. Server responds with JSON-RPC 2.0 results or errors
  4. Connection stays open for multiple request/response pairs

Design:
  - Fresh MCPRouter per WebSocket connection (stateless)
  - Uses handle_message_async() for all messages (supports both sync and async methods)
  - Graceful disconnect handling
  - JSON-RPC error responses for unknown methods
"""
from __future__ import annotations

import logging

from fastapi import APIRouter, WebSocket, WebSocketDisconnect

from cynic.mcp.router import MCPRouter

logger = logging.getLogger("cynic.api.routers.mcp_websocket")

router = APIRouter(tags=["mcp-ws"])


@router.websocket("/ws/mcp")
async def websocket_mcp(websocket: WebSocket) -> None:
    """WebSocket endpoint for MCP JSON-RPC protocol.

    Each connection gets a fresh MCPRouter instance.
    Messages are routed through handle_message_async() which
    supports tools/list (sync) and tools/call (async).
    """
    await websocket.accept()
    mcp_router = MCPRouter()

    logger.info("MCP WebSocket: client connected")

    try:
        while True:
            message = await websocket.receive_json()
            logger.debug("MCP RX: %s", message)

            response = await mcp_router.handle_message_async(message)

            await websocket.send_json(response)
            logger.debug("MCP TX: %s", response)

    except WebSocketDisconnect:
        logger.info("MCP WebSocket: client disconnected")
    except Exception as exc:
        logger.error("MCP WebSocket error: %s", exc)
        try:
            await websocket.close(code=1011, reason=str(exc))
        except RuntimeError:
            pass  # Already closed
