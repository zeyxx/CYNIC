"""
Stdio MCP Server for CYNIC - Cline integration via Model Context Protocol.

Uses the `mcp` Python package to implement stdio-based MCP server.
Exposes tools for Cline to call CYNIC autonomously:
  - cynic_run_empirical_test(count, seed) -> run 1000+ judgments
  - cynic_get_job_status(job_id) -> poll progress
  - cynic_get_results(job_id) -> fetch Q-scores + metrics
  - cynic_run_irreducibility_test(axiom) -> test axiom necessity
  - cynic_query_telemetry(metric) -> SONA heartbeat data

Runs via stdin/stdout (no HTTP port needed).
Cline calls via MCP protocol (JSON-RPC over stdio).
"""
from __future__ import annotations

import asyncio
import json
import logging
import sys
from typing import Any, Optional

try:
    from mcp.server import Server
    from mcp.server.stdio import stdio_server
    from mcp.types import Tool, TextContent, CallToolResult
    from mcp.server.models import InitializationOptions
except ImportError as e:
    raise ImportError(f"Install mcp: pip install mcp (error: {e})")

# Alias for backward compatibility
ToolResult = CallToolResult

from cynic.mcp.empirical_runner import EmpiricalRunner

logger = logging.getLogger("cynic.mcp.stdio_server")


class CynicMCPServer:
    """
    Stdio MCP server exposing CYNIC judgment testing interface.

    Flow:
    1. Cline: "Run an empirical test"
       -> Calls cynic_run_empirical_test(count=1000)
    2. Server: Spawns async job, returns {job_id: "test-..."}
    3. Cline: "What's the status?"
       -> Calls cynic_get_job_status(job_id="test-...")
    4. Server: Returns {progress_percent: 45, eta_s: 300}
    5. [After job completes] Cline: "Get results"
       -> Calls cynic_get_results(job_id)
    6. Server: Returns {q_scores: [...], avg_q: 52.4, learning_efficiency: 1.18x}
    """

    def __init__(self, organism_getter: callable):
        """
        Initialize MCP server.

        Args:
            organism_getter: Callable returning CynicOrganism instance
        """
        self.organism_getter = organism_getter
        self.runner = EmpiricalRunner(organism_getter)

        # MCP server instance
        self.mcp_server = Server("cynic")
        self._setup_tools()

    def _setup_tools(self) -> None:
        """Register all tools with MCP server."""

        @self.mcp_server.call_tool()
        async def handle_tool_call(name: str, arguments: dict):
            """Route tool calls to handlers."""
            try:
                if name == "cynic_run_empirical_test":
                    return await self._handle_run_empirical_test(arguments)
                elif name == "cynic_get_job_status":
                    return await self._handle_get_job_status(arguments)
                elif name == "cynic_get_results":
                    return await self._handle_get_results(arguments)
                elif name == "cynic_run_irreducibility_test":
                    return await self._handle_run_irreducibility_test(arguments)
                elif name == "cynic_query_telemetry":
                    return await self._handle_query_telemetry(arguments)
                else:
                    return [TextContent(type="text", text=f"Unknown tool: {name}")]
            except Exception as e:
                logger.exception(f"Tool error: {name}")
                return [TextContent(type="text", text=f"Error: {str(e)}")]

        # Register tools with descriptions
        self.mcp_server.tools = [
            Tool(
                name="cynic_run_empirical_test",
                description="Run an empirical test of CYNIC judgment system. Spawns async batch runner with N iterations.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "count": {
                            "type": "integer",
                            "description": "Number of judgment iterations (default: 1000)",
                        },
                        "seed": {
                            "type": "integer",
                            "description": "Random seed for reproducibility (optional)",
                        },
                    },
                    "required": [],
                },
            ),
            Tool(
                name="cynic_get_job_status",
                description="Get status and progress of a running test job.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "job_id": {
                            "type": "string",
                            "description": "Job ID returned from cynic_run_empirical_test",
                        },
                    },
                    "required": ["job_id"],
                },
            ),
            Tool(
                name="cynic_get_results",
                description="Get complete results from a finished test job (Q-scores, metrics, emergence events).",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "job_id": {
                            "type": "string",
                            "description": "Job ID of completed test",
                        },
                    },
                    "required": ["job_id"],
                },
            ),
            Tool(
                name="cynic_run_irreducibility_test",
                description="Test if each axiom is irreducible (necessary) for CYNIC judgment quality.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "axiom": {
                            "type": "string",
                            "description": "Test specific axiom (PHI, VERIFY, CULTURE, BURN, FIDELITY), or null for all",
                        },
                    },
                    "required": [],
                },
            ),
            Tool(
                name="cynic_query_telemetry",
                description="Query CYNIC system telemetry from SONA heartbeat.",
                inputSchema={
                    "type": "object",
                    "properties": {
                        "metric": {
                            "type": "string",
                            "description": "Metric name: uptime_s, q_table_entries, total_judgments, learning_rate",
                        },
                    },
                    "required": ["metric"],
                },
            ),
        ]

    async def _handle_run_empirical_test(self, args: dict) -> ToolResult:
        """Handle: cynic_run_empirical_test(count, seed)."""
        count = args.get("count", 1000)
        seed = args.get("seed")

        job_id = await self.runner.spawn_test(count=count, seed=seed)

        result = {
            "job_id": job_id,
            "status": "queued",
            "message": f"Started empirical test with {count} iterations",
        }

        return ToolResult(
            content=[TextContent(type="text", text=json.dumps(result, indent=2))],
            is_error=False,
        )

    async def _handle_get_job_status(self, args: dict) -> ToolResult:
        """Handle: cynic_get_job_status(job_id)."""
        job_id = args.get("job_id")
        if not job_id:
            return ToolResult(
                content=[TextContent(type="text", text="job_id required")],
                is_error=True,
            )

        status = await self.runner.get_job_status(job_id)

        return ToolResult(
            content=[TextContent(type="text", text=json.dumps(status, indent=2))],
            is_error=False,
        )

    async def _handle_get_results(self, args: dict) -> ToolResult:
        """Handle: cynic_get_results(job_id)."""
        job_id = args.get("job_id")
        if not job_id:
            return ToolResult(
                content=[TextContent(type="text", text="job_id required")],
                is_error=True,
            )

        results = await self.runner.get_results(job_id)
        if not results:
            return ToolResult(
                content=[TextContent(type="text", text=f"Job {job_id} not ready or not found")],
                is_error=True,
            )

        return ToolResult(
            content=[TextContent(type="text", text=json.dumps(results, indent=2))],
            is_error=False,
        )

    async def _handle_run_irreducibility_test(self, args: dict) -> ToolResult:
        """Handle: cynic_run_irreducibility_test(axiom)."""
        axiom = args.get("axiom")

        results = await self.runner.run_irreducibility_test(axiom=axiom)

        return ToolResult(
            content=[TextContent(type="text", text=json.dumps(results, indent=2))],
            is_error=False,
        )

    async def _handle_query_telemetry(self, args: dict) -> ToolResult:
        """Handle: cynic_query_telemetry(metric)."""
        metric = args.get("metric", "uptime_s")

        telemetry = await self.runner.query_telemetry(metric=metric)

        return ToolResult(
            content=[TextContent(type="text", text=json.dumps(telemetry, indent=2))],
            is_error=False,
        )

    async def run(self) -> None:
        """
        Start the MCP server on stdin/stdout.

        This blocks forever, reading JSON-RPC requests from stdin
        and writing responses to stdout.
        """
        logger.info("Starting CYNIC MCP server (stdio)...")

        # Use MCP SDK's built-in stdio_server helper which handles async wrapping
        async with stdio_server() as (read_stream, write_stream):
            logger.info("MCP stdio streams connected")

            # Create initialization options for the MCP server
            from mcp.types import ServerCapabilities

            init_options = InitializationOptions(
                server_name="CYNIC",
                server_version="1.0.0",
                capabilities=ServerCapabilities(),
                instructions="CYNIC AI organism for memecoin governance decisions"
            )
            logger.info(f"MCP server initialized: {init_options.server_name} v{init_options.server_version}")

            # Run the server with the properly wrapped streams
            await self.mcp_server.run(
                read_stream,
                write_stream,
                init_options,
                raise_exceptions=True
            )


async def start_mcp_server(organism_getter: callable) -> None:
    """
    Standalone entry point: initialize and run MCP server.

    Args:
        organism_getter: Callable returning CynicOrganism
    """
    server = CynicMCPServer(organism_getter)
    await server.run()


if __name__ == "__main__":
    # For testing: python -m cynic.mcp.stdio_server
    # (requires organism to be pre-initialized)

    async def dummy_organism():
        """Placeholder for testing."""
        return None

    asyncio.run(start_mcp_server(dummy_organism))
