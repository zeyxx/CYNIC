"""Entry point for CYNIC observability CLI application.

This module provides the main entry point for running the CYNIC observability
CLI, which enables interactive monitoring and interaction with the CYNIC
organism's state.

Typical usage:
    python -m cynic.interfaces.cli.main
    python cynic/cli/main.py

The CLI provides a menu-driven interface for observing CYNIC's mind, human state,
machine resources, and managing feedback loops.
"""

import asyncio
import sys
from cynic.kernel.observability.cli.app import CliApp


async def main() -> None:
    """Main entry point for the observability CLI application.

    Initializes the CLI app and runs the interactive menu loop. Handles
    KeyboardInterrupt for graceful shutdown, and catches any other exceptions
    for clean error reporting.

    Exit codes:
        0: Normal exit (user selected option 0)
        1: Error or exception occurred
    """
    app = CliApp()
    try:
        await app.run()
    except KeyboardInterrupt:
        print("\n\nInterrupted. Goodbye!")
        sys.exit(0)
    except Exception as e:
        print(f"\nError: {e}")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
