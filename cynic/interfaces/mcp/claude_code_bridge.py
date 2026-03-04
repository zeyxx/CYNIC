"""Claude Code MCP Bridge — thin wrapper around universal CYNIC tools."""
from __future__ import annotations

import asyncio
import json
import logging
import subprocess  # module-level: tests patch claude_code_bridge.subprocess.Popen
import aiohttp     # module-level: tests patch claude_code_bridge.aiohttp.ClientSession

try:
    from mcp.types import TextContent
except ImportError:
    class TextContent:  # type: ignore[no-redef]
        """Fallback TextContent for when mcp is not available."""

        def __init__(self, type: str, text: str) -> None:
            self.type = type
            self.text = text

from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter
from cynic.interfaces.mcp.kernel_tools import (
    tool_ask_cynic,
    tool_get_job_status,
    tool_get_test_results,
    tool_learn_cynic,
    tool_query_telemetry,
    tool_run_empirical_test,
    tool_test_axiom_irreducibility,
)
from cynic.interfaces.mcp.kernel_startup import (
    ensure_kernel_running,
    do_spawn_kernel,
    CYNIC_URL,
)

logger = logging.getLogger(__name__)

_adapter: ClaudeCodeAdapter | None = None
_lock = asyncio.Lock()


async def get_adapter() -> ClaudeCodeAdapter:
    """Singleton ClaudeCodeAdapter."""
    global _adapter
    async with _lock:
        if _adapter is None:
            _adapter = ClaudeCodeAdapter(cynic_url=CYNIC_URL)
    return _adapter


async def _ensure_kernel_running(timeout: float = 30.0, spawn_if_down: bool = False) -> bool:
    """Check kernel health; uses module-level aiohttp (patchable by tests)."""
    def spawn_fn() -> None:
        """Spawn kernel using module-level subprocess (sync wrapper)."""
        do_spawn_kernel(subprocess_module=subprocess)

    return await ensure_kernel_running(
        aiohttp_module=aiohttp,
        spawn_fn=spawn_fn,
        timeout=timeout,
        spawn_if_down=spawn_if_down,
    )


async def _spawn_kernel() -> subprocess.Popen | None:  # type: ignore[type-arg]
    """Spawn CYNIC kernel; uses module-level subprocess (patchable by tests)."""
    return do_spawn_kernel(subprocess_module=subprocess)


def _fmt(result: dict) -> list[TextContent]:
    """Format a raw dict result as MCP TextContent list."""
    if "error" in result:
        return [TextContent(type="text", text=f"Error: {result['error']}")]
    return [TextContent(type="text", text=json.dumps(result, indent=2))]


async def _tool_cynic_run_empirical_test(args: dict) -> list[TextContent]:
    """MCP tool: run empirical test."""
    adapter = await get_adapter()
    return _fmt(await tool_run_empirical_test(adapter, args))


async def _tool_cynic_get_job_status(args: dict) -> list[TextContent]:
    """MCP tool: get job status."""
    adapter = await get_adapter()
    return _fmt(await tool_get_job_status(adapter, args))


async def _tool_cynic_get_test_results(args: dict) -> list[TextContent]:
    """MCP tool: get test results."""
    adapter = await get_adapter()
    return _fmt(await tool_get_test_results(adapter, args))


async def _tool_cynic_test_axiom_irreducibility(args: dict) -> list[TextContent]:
    """MCP tool: test axiom irreducibility."""
    adapter = await get_adapter()
    return _fmt(await tool_test_axiom_irreducibility(adapter, args))


async def _tool_cynic_query_telemetry(args: dict) -> list[TextContent]:
    """MCP tool: query telemetry."""
    adapter = await get_adapter()
    return _fmt(await tool_query_telemetry(adapter, args))


async def _tool_ask_cynic(args: dict) -> list[TextContent]:
    """MCP tool: ask CYNIC a question."""
    adapter = await get_adapter()
    return _fmt(await tool_ask_cynic(adapter, args))


async def _tool_learn_cynic(args: dict) -> list[TextContent]:
    """MCP tool: teach CYNIC."""
    adapter = await get_adapter()
    return _fmt(await tool_learn_cynic(adapter, args))
