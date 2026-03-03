"""
CYNIC Deploy Command

Manage Docker stack via Python Docker SDK (no CLI friction).

Usage:
  python -m cynic.interfaces.cli deploy status      # Show all container status
  python -m cynic.interfaces.cli deploy logs        # Stream logs from services
  python -m cynic.interfaces.cli deploy restart     # Restart a service
  python -m cynic.interfaces.cli deploy up          # Deploy full stack
  python -m cynic.interfaces.cli deploy down        # Shutdown stack
  python -m cynic.interfaces.cli deploy health      # Continuous health monitoring
"""
import asyncio
import logging
import sys

from cynic.deployment import DockerManager

logger = logging.getLogger(__name__)


def setup_logging():
    """Configure logging."""
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )


async def cmd_status():
    """Show status of all CYNIC services."""
    mgr = DockerManager()
    await mgr.initialize()

    status = await mgr.get_status()

    if status.failures:
        for _failure in status.failures:
            pass

    for _name, container in status.containers.items():
        f" " {', '.join(container.ports)}" if container.ports else """


    return 0 if status.all_healthy else 1


async def cmd_logs(service: str | None = None):
    """Stream logs from a service."""
    if not service:
        service = "cynic-kernel"  # Default

    mgr = DockerManager()
    await mgr.initialize()

    logs = await mgr.get_logs(service, lines=100)

    if logs:
        return 0
    else:
        return 1


async def cmd_restart(service: str | None = None):
    """Restart a service."""
    if not service:
        service = "cynic-kernel"  # Default

    mgr = DockerManager()
    await mgr.initialize()

    result = await mgr.restart_service(service, wait_healthy_s=30.0)

    if result:
        return 0
    else:
        return 1


async def cmd_deploy(rebuild: bool = False):
    """Deploy full CYNIC stack."""
    mgr = DockerManager()
    await mgr.initialize()

    result = await mgr.deploy_stack(rebuild=rebuild)

    if result:

        # Show status
        await mgr.get_status()
        return 0
    else:

        # Show status for debugging
        await mgr.get_status()
        return 1


async def cmd_shutdown():
    """Graceful shutdown of stack."""
    mgr = DockerManager()
    await mgr.initialize()

    result = await mgr.shutdown_stack()

    if result:
        return 0
    else:
        return 1


async def cmd_health(interval_s: float = 10.0):
    """Continuous health monitoring."""
    mgr = DockerManager()
    await mgr.initialize()


    try:
        await mgr.health_check_loop(interval_s=interval_s, auto_restart=True)
    except KeyboardInterrupt:
        return 0


async def main():
    """CLI entry point."""
    setup_logging()

    if len(sys.argv) < 2:
        return 1

    cmd = sys.argv[1]
    args = sys.argv[2:] if len(sys.argv) > 2 else []

    try:
        if cmd == "status":
            return await cmd_status()

        elif cmd == "logs":
            service = args[0] if args else None
            return await cmd_logs(service)

        elif cmd == "restart":
            service = args[0] if args else None
            return await cmd_restart(service)

        elif cmd == "up":
            rebuild = "--rebuild" in args
            return await cmd_deploy(rebuild=rebuild)

        elif cmd == "down":
            return await cmd_shutdown()

        elif cmd == "health":
            interval = float(args[0]) if args else 10.0
            return await cmd_health(interval_s=interval)

        else:
            return 1

    except CynicError as e:
        logger.error(f"Error: {e}", exc_info=True)
        return 1


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))