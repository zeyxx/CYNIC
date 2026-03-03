"""
Stdio MCP Server for CYNIC - Cline integration via Model Context Protocol.

Compliant with the latest MCP Python SDK.
"""

from __future__ import annotations

import asyncio
import json
import logging
from typing import Callable, List

try:
    from mcp.server import Server
    from mcp.server.models import InitializationOptions
    from mcp.server.stdio import stdio_server
    from mcp.types import CallToolResult, TextContent, Tool
except ImportError as e:
    raise ImportError(f"Install mcp: pip install mcp (error: {e})")

from cynic.interfaces.mcp.empirical_runner import EmpiricalRunner

logger = logging.getLogger("cynic.interfaces.mcp.stdio_server")


class CynicMCPServer:
    def __init__(self, organism_getter: Callable):
        self.organism_getter = organism_getter
        self.runner = EmpiricalRunner(organism_getter)
        self.mcp_server = Server("cynic")
        self._setup_handlers()

    def _setup_handlers(self) -> None:
        @self.mcp_server.list_tools()
        async def handle_list_tools() -> List[Tool]:
            return [
                Tool(
                    name="cynic_run_empirical_test",
                    description="Run an empirical test of CYNIC judgment system.",
                    inputSchema={
                        "type": "object",
                        "properties": {
                            "count": {"type": "integer"},
                            "seed": {"type": "integer"},
                        },
                    },
                ),
                Tool(
                    name="cynic_query_telemetry",
                    description="Query CYNIC system telemetry.",
                    inputSchema={
                        "type": "object",
                        "properties": {"metric": {"type": "string"}},
                        "required": ["metric"],
                    },
                ),
            ]

        @self.mcp_server.call_tool()
        async def handle_tool_call(name: str, arguments: dict) -> CallToolResult:
            try:
                if name == "cynic_run_empirical_test":
                    count = arguments.get("count", 1000)
                    seed = arguments.get("seed")
                    job_id = await self.runner.spawn_test(count=count, seed=seed)
                    result = {"job_id": job_id, "status": "queued"}
                    return CallToolResult(
                        content=[TextContent(type="text", text=json.dumps(result))]
                    )

                elif name == "cynic_query_telemetry":
                    metric = arguments.get("metric", "uptime_s")
                    telemetry = await self.runner.query_telemetry(metric=metric)
                    return CallToolResult(
                        content=[TextContent(type="text", text=json.dumps(telemetry))]
                    )

                else:
                    return CallToolResult(
                        content=[
                            TextContent(type="text", text=f"Unknown tool: {name}")
                        ],
                        isError=True,
                    )
            except Exception as e:
                logger.exception(f"Tool error: {name}")
                return CallToolResult(
                    content=[TextContent(type="text", text=str(e))], isError=True
                )

    async def run(self) -> None:
        async with stdio_server() as (read_stream, write_stream):
            await self.mcp_server.run(
                read_stream,
                write_stream,
                InitializationOptions(
                    server_name="CYNIC",
                    server_version="1.0.0",
                    capabilities=self.mcp_server.get_capabilities(
                        notification_options={},  # type: ignore
                        experimental_capabilities={},
                    ),
                ),
            )


async def start_mcp_server(organism_getter: Callable) -> None:
    server = CynicMCPServer(organism_getter)
    await server.run()


if __name__ == "__main__":

    async def dummy_organism():
        return None

    asyncio.run(start_mcp_server(dummy_organism))
