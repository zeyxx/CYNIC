"""
MCP Server Entry Point — Start CYNIC as MCP server for Cline.

Can run in two modes:
1. HTTP mode (default): python -m cynic.mcp
   → Starts FastAPI at :8766 + stdio MCP server
2. stdio mode only: CYNIC_MCP_STDIO_ONLY=1 python -m cynic.mcp
   → Starts only stdio MCP (no HTTP)

Usage:
  docker-compose up cynic-mcp
  → Starts CYNIC with both HTTP and stdio MCP
"""
from __future__ import annotations

import asyncio
import logging
import os
import sys
from typing import Optional

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("cynic.mcp.__main__")


async def main() -> None:
    """Initialize CYNIC organism and start MCP servers."""

    # Import here to ensure async context is ready
    try:
        from cynic.core.consciousness import Consciousness
        from cynic.mcp.stdio_server import CynicMCPServer
    except ImportError as e:
        logger.error(f"Import error: {e}")
        sys.exit(1)

    logger.info("Initializing CYNIC organism...")

    try:
        # Awaken CYNIC organism
        consciousness = Consciousness()
        organism = await consciousness.awaken()
        logger.info("CYNIC organism awakened successfully")
    except Exception as e:
        logger.error(f"Failed to awaken CYNIC: {e}")
        sys.exit(1)

    # Define organism getter for MCP server
    def get_organism() -> Optional[object]:
        return organism

    # Check if stdio-only mode
    stdio_only = os.getenv("CYNIC_MCP_STDIO_ONLY", "0") == "1"

    tasks = []

    if not stdio_only:
        # Start HTTP MCP server (backwards compatible)
        try:
            from cynic.mcp.server import MCPServer

            http_server = MCPServer(port=8766, get_state_fn=get_organism)
            logger.info("Starting HTTP MCP server at :8766...")
            await http_server.start()

            # Create task that monitors HTTP server (doesn't block)
            async def http_server_monitor():
                # HTTP server runs forever once started
                await asyncio.sleep(float("inf"))

            tasks.append(asyncio.create_task(http_server_monitor()))
        except Exception as e:
            logger.error(f"Failed to start HTTP MCP server: {e}")

    # Start stdio MCP server (main entry point for Cline)
    logger.info("Starting stdio MCP server for Cline...")
    stdio_server = CynicMCPServer(get_organism)

    # Run stdio server (blocks forever, reading from stdin)
    try:
        await stdio_server.run()
    except KeyboardInterrupt:
        logger.info("Shutdown requested")
        sys.exit(0)
    except Exception as e:
        logger.exception(f"Stdio MCP server error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        logger.info("Interrupted")
        sys.exit(0)
