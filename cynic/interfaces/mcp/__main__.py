"""
MCP Server Entry Point - Start CYNIC as MCP server for Cline.

Can run in two modes:
1. HTTP mode: python -m cynic.interfaces.mcp (if CYNIC_MCP_STDIO_ONLY=0)
   Starts FastAPI at :8766 + stdio MCP server
2. stdio mode only: CYNIC_MCP_STDIO_ONLY=1 python -m cynic.interfaces.mcp (default)
   Starts only stdio MCP (no HTTP)

Configuration is loaded from environment variables via CynicConfig.
No direct os.getenv() calls - all config goes through the config system.

Usage:
  docker-compose up cynic-mcp
  - Starts CYNIC with both HTTP and stdio MCP
"""
from __future__ import annotations

import asyncio
import logging
import os
import sys

# Windows UTF-8 encoding fix
if sys.platform == "win32":
    os.environ["PYTHONIOENCODING"] = "utf-8"
    if hasattr(sys.stdout, "reconfigure"):
        try:
            sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass
    if hasattr(sys.stderr, "reconfigure"):
        try:
            sys.stderr.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("cynic.interfaces.mcp.__main__")


async def main() -> None:
    """Initialize CYNIC organism and start MCP servers."""

    # Load configuration (Rule 3: centralize config reads)
    try:
        from cynic.kernel.core.config import CynicConfig
        config = CynicConfig.from_env()
    except ImportError as e:
        logger.error(f"Failed to load config: {e}")
        sys.exit(1)

    # Import here to ensure async context is ready
    try:
        from cynic.interfaces.mcp.stdio_server import CynicMCPServer
        from cynic.kernel.core.consciousness import get_consciousness
    except ImportError as e:
        logger.error(f"Import error: {e}")
        sys.exit(1)

    logger.info("Initializing CYNIC organism...")

    try:
        # Awaken CYNIC organism (get or create global consciousness state)
        organism = get_consciousness()
        logger.info("CYNIC organism awakened successfully")
    except Exception as e:
        logger.error(f"Failed to awaken CYNIC: {e}")
        sys.exit(1)

    # Define organism getter for MCP server
    def get_organism() -> object | None:
        return organism

    # Check if stdio-only mode (from config, not direct os.getenv)
    stdio_only = config.mcp_stdio_only

    if not stdio_only:
        # Start HTTP MCP server (backwards compatible, disabled by default)
        try:
            from cynic.interfaces.mcp.server import MCPServer

            http_server = MCPServer(port=8766, get_state_fn=get_organism)
            logger.info("Starting HTTP MCP server at :8766...")
            await http_server.start()
        except Exception as e:
            logger.warning(f"Failed to start HTTP MCP server (non-critical): {e}")

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
        # On Windows, use ProactorEventLoop for proper stdio handling
        if sys.platform == "win32":
            asyncio.set_event_loop_policy(asyncio.WindowsProactorEventLoopPolicy())
        asyncio.run(main())
    except KeyboardInterrupt:
        logger.info("Interrupted")
        sys.exit(0)
