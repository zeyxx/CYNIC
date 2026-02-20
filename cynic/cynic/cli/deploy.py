"""
CYNIC Deploy Command

Manage Docker stack via Python Docker SDK (no CLI friction).

Usage:
  python -m cynic.cli deploy status      # Show all container status
  python -m cynic.cli deploy logs        # Stream logs from services
  python -m cynic.cli deploy restart     # Restart a service
  python -m cynic.cli deploy up          # Deploy full stack
  python -m cynic.cli deploy down        # Shutdown stack
  python -m cynic.cli deploy health      # Continuous health monitoring
"""
import asyncio
import json
import sys
import logging
from typing import Optional

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
    print("\n" + "=" * 70)
    print("CYNIC DOCKER STACK STATUS")
    print("=" * 70)
    print(f"Timestamp: {status.timestamp}")
    print(f"All Healthy: {'✓ YES' if status.all_healthy else '✗ NO'}")

    if status.failures:
        print(f"\n⚠ Failures ({len(status.failures)}):")
        for failure in status.failures:
            print(f"  - {failure}")

    print("\nServices:")
    for name, container in status.containers.items():
        health_indicator = "✓" if container.status == "running" else "✗"
        health_str = f" (health: {container.health})" if container.health else ""
        uptime_str = f" — uptime: {container.uptime_s:.0f}s" if container.uptime_s else ""
        ports_str = f" — {', '.join(container.ports)}" if container.ports else ""

        print(
            f"  {health_indicator} {name}: {container.status}{health_str}{uptime_str}{ports_str}"
        )

    print("\n" + "=" * 70)
    return 0 if status.all_healthy else 1


async def cmd_logs(service: Optional[str] = None):
    """Stream logs from a service."""
    if not service:
        service = "cynic-kernel"  # Default

    mgr = DockerManager()
    await mgr.initialize()

    print(f"\n[LOGS] Streaming from {service}...\n")
    logs = await mgr.get_logs(service, lines=100)

    if logs:
        print(logs)
        return 0
    else:
        print(f"✗ Failed to get logs from {service}")
        return 1


async def cmd_restart(service: Optional[str] = None):
    """Restart a service."""
    if not service:
        service = "cynic-kernel"  # Default

    mgr = DockerManager()
    await mgr.initialize()

    print(f"\n[RESTART] Restarting {service}...")
    result = await mgr.restart_service(service, wait_healthy_s=30.0)

    if result:
        print(f"✓ {service} restarted and healthy")
        return 0
    else:
        print(f"✗ Failed to restart {service} or timeout waiting for health")
        return 1


async def cmd_deploy(rebuild: bool = False):
    """Deploy full CYNIC stack."""
    mgr = DockerManager()
    await mgr.initialize()

    print(f"\n[DEPLOY] Starting CYNIC stack {'(rebuild)' if rebuild else ''}...")
    result = await mgr.deploy_stack(rebuild=rebuild)

    if result:
        print("✓ Stack deployed successfully, all services healthy")

        # Show status
        status = await mgr.get_status()
        print(json.dumps(status.to_dict(), indent=2))
        return 0
    else:
        print("✗ Stack deployment failed or timeout waiting for health")

        # Show status for debugging
        status = await mgr.get_status()
        print("\nCurrent status:")
        print(json.dumps(status.to_dict(), indent=2))
        return 1


async def cmd_shutdown():
    """Graceful shutdown of stack."""
    mgr = DockerManager()
    await mgr.initialize()

    print("\n[SHUTDOWN] Stopping CYNIC stack...")
    result = await mgr.shutdown_stack()

    if result:
        print("✓ Stack shutdown complete")
        return 0
    else:
        print("✗ Shutdown failed")
        return 1


async def cmd_health(interval_s: float = 10.0):
    """Continuous health monitoring."""
    mgr = DockerManager()
    await mgr.initialize()

    print(f"\n[HEALTH] Monitoring with {interval_s}s interval (Ctrl+C to stop)...")
    print("Auto-restart enabled: unhealthy services will be restarted\n")

    try:
        await mgr.health_check_loop(interval_s=interval_s, auto_restart=True)
    except KeyboardInterrupt:
        print("\n✓ Health monitoring stopped")
        return 0


async def main():
    """CLI entry point."""
    setup_logging()

    if len(sys.argv) < 2:
        print(__doc__)
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
            print(f"Unknown command: {cmd}")
            print(__doc__)
            return 1

    except Exception as e:
        logger.error(f"Error: {e}", exc_info=True)
        return 1


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
