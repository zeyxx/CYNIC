"""MCP Bridge Observability Endpoints.

Exposes MCPBridge health, metrics, and registered tools.
Three read-only endpoints for monitoring the MCP protocol layer.

Auto-registered by auto_register_routers().
"""
from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends

from cynic.api.state import AppContainer, get_app_container

router = APIRouter(prefix="/api/mcp/bridge", tags=["mcp-observability"])


@router.get("/health")
async def mcp_bridge_health(
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """MCPBridge health — running status, tool count, error rate."""
    bridge = container.organism.senses.mcp_bridge
    return bridge.get_health()


@router.get("/metrics")
async def mcp_bridge_metrics(
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """MCPBridge metrics — call counts, latency stats, uptime."""
    bridge = container.organism.senses.mcp_bridge
    return bridge.get_metrics()


@router.get("/tools")
async def mcp_bridge_tools(
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """List all MCP tools registered in the bridge."""
    bridge = container.organism.senses.mcp_bridge
    tools = [
        {
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.input_schema,
        }
        for tool in bridge.tools.values()
    ]
    return {"tools": tools, "count": len(tools)}
