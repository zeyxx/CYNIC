"""MCPBridge â€” Base service for MCP protocol integration.

Translates MCP tool calls into organism events.
Lives inside the SensoryCore as a subsystem.

Responsibilities:
- Lifecycle management (startup/shutdown)
- Tool registration (MCPTool dataclass)
- Event emission to organism event bus on tool calls

Architecture:
- Emits to one of the 3 buses (CORE by default)
- Each tool call becomes an "mcp.tool_called" event
- The organism reacts to these events via its normal event handlers
- MCPBridge does NOT process results â€” it only translates input
"""

from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any

from cynic.interfaces.mcp.metrics import MCPMetrics
from cynic.kernel.core.event_bus import (
    CoreEvent,
    Event,
    EventBus,
)

logger = logging.getLogger(__name__)


@dataclass
class MCPTool:
    """A tool exposed via the MCP protocol.

    Attributes:
        name: Unique tool identifier (e.g. "judge", "perceive").
        description: Human-readable purpose of the tool.
        input_schema: JSON Schema dict describing accepted arguments.
    """

    name: str
    description: str
    input_schema: dict[str, Any] = field(default_factory=dict)


class MCPBridge:
    """Bridge between MCP protocol and the CYNIC organism.

    Manages tool registration and translates MCP calls into
    organism events on one of the 3 event buses.

    Usage:
        bridge = MCPBridge(bus_name="CORE")
        bridge.register_tool(MCPTool(name="judge", ...))
        await bridge.startup()
        result = await bridge.handle_call("judge", {"prompt": "..."})
        await bridge.shutdown()
    """

    def __init__(self, bus: EventBus) -> None:
        self.is_running = False
        self.tools: dict[str, MCPTool] = {}
        self._bus = bus
        self.metrics = MCPMetrics()

    async def startup(self) -> None:
        """Start the bridge. Idempotent — safe to call twice."""
        if self.is_running:
            return
        self.is_running = True
        logger.info("MCPBridge active with %d tools", len(self.tools))

    async def shutdown(self) -> None:
        """Stop the bridge and cleanup resources."""
        self.is_running = False
        logger.info("MCPBridge stopped")

    def register_tool(self, tool: MCPTool) -> None:
        """Register a tool. Overwrites if name already exists."""
        self.tools[tool.name] = tool
        logger.debug("Registered MCP tool: %s", tool.name)

    async def handle_call(self, tool_name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        """Handle an MCP tool call by emitting an event.

        Args:
            tool_name: Name of the registered tool to invoke.
            arguments: Arguments dict from the MCP protocol message.

        Returns:
            Acknowledgment dict with status, tool_name, and event_id.

        Raises:
            RuntimeError: If the bridge is not running.
            KeyError: If the tool is not registered.
        """
        start = time.perf_counter()
        try:
            if not self.is_running:
                raise RuntimeError("MCPBridge is not running â€” call startup() first")

            if tool_name not in self.tools:
                raise KeyError(f"Tool {tool_name!r} not registered")

            event = Event(
                type=CoreEvent.MCP_TOOL_CALLED,
                payload={"tool_name": tool_name, "arguments": arguments},
                source="mcp_bridge",
            )

            await self._bus.emit(event)

            latency_ms = (time.perf_counter() - start) * 1000
            self.metrics.record_call(latency_ms, success=True)

            logger.info("MCP call: tool=%s event_id=%s latency=%.1fms", tool_name, event.event_id, latency_ms)

            return {
                "status": "emitted",
                "tool_name": tool_name,
                "event_id": event.event_id,
            }
        except Exception:
            latency_ms = (time.perf_counter() - start) * 1000
            self.metrics.record_call(latency_ms, success=False)
            raise

    def get_metrics(self) -> dict[str, Any]:
        """Return current metrics as a serializable dict."""
        return self.metrics.to_dict()

    def get_health(self) -> dict[str, Any]:
        """Return health status of the bridge.

        Returns:
            Dict with status, uptime, tools count, total calls, and error rate.
            status is "healthy" when running, "degraded" when stopped.
        """
        error_rate = 0.0
        if self.metrics.total_calls > 0:
            error_rate = self.metrics.failed_calls / self.metrics.total_calls
        return {
            "status": "healthy" if self.is_running else "degraded",
            "uptime_s": round(self.metrics.uptime_s, 2),
            "tools_registered": len(self.tools),
            "total_calls": self.metrics.total_calls,
            "error_rate": error_rate,
        }
