"""
CYNIC Docker Deployment Manager

Native Python Docker API management instead of Bash CLI.
Paradigm shift: Docker = native CYNIC capability, not external friction.

"Le chien qui gère ses conteneurs" — κυνικός
"""
import asyncio
import json
import logging
from dataclasses import dataclass, field, asdict
from datetime import datetime
from pathlib import Path
from typing import Optional, Dict, List
import docker
from docker import DockerClient
from docker.errors import DockerException, NotFound

logger = logging.getLogger(__name__)


@dataclass
class ContainerStatus:
    """Status of a single container."""
    name: str
    image: str
    status: str  # 'running', 'exited', 'created', etc.
    health: Optional[str] = None  # 'healthy', 'unhealthy', 'starting', None
    ports: List[str] = field(default_factory=list)
    uptime_s: Optional[float] = None
    last_error: Optional[str] = None

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class StackStatus:
    """Overall stack status."""
    timestamp: str
    containers: Dict[str, ContainerStatus]
    all_healthy: bool
    failures: List[str]

    def to_dict(self) -> dict:
        return {
            "timestamp": self.timestamp,
            "containers": {k: v.to_dict() for k, v in self.containers.items()},
            "all_healthy": self.all_healthy,
            "failures": self.failures,
        }


class DockerManager:
    """Manage CYNIC's Docker stack via native Python Docker SDK."""

    CYNIC_SERVICES = ["cynic-kernel", "cynic-ollama", "cynic-surrealdb"]
    STACK_NAME = "cynic"
    DEPLOY_DIR = Path(__file__).parent.parent.parent / "cynic"  # cynic/cynic/ root
    COMPOSE_FILE = DEPLOY_DIR / "docker-compose.yml"
    LOG_DIR = Path.home() / ".cynic" / "deployments"

    def __init__(self):
        """Initialize Docker manager."""
        self.client: Optional[DockerClient] = None
        self.log_dir = self.LOG_DIR
        self.log_dir.mkdir(parents=True, exist_ok=True)

    async def initialize(self) -> bool:
        """Connect to Docker daemon (async-safe)."""
        try:
            self.client = docker.from_env()
            # Test connection
            self.client.ping()
            logger.info("Docker API connected")
            return True
        except OSError as e:
            logger.error(f"Failed to connect Docker API: {e}")
            self.client = None
            return False

    async def get_status(self) -> StackStatus:
        """Get current status of all CYNIC containers (no CLI friction)."""
        if not self.client:
            await self.initialize()

        containers = {}
        failures = []

        try:
            for service_name in self.CYNIC_SERVICES:
                try:
                    container = self.client.containers.get(service_name)
                    status_str = container.status  # 'running', 'exited', etc.

                    # Get health
                    health = None
                    if container.attrs.get("State", {}).get("Health"):
                        health = container.attrs["State"]["Health"].get("Status")

                    # Parse ports
                    ports = []
                    port_bindings = container.attrs.get("HostConfig", {}).get("PortBindings", {})
                    for container_port, host_bindings in port_bindings.items():
                        for binding in host_bindings or []:
                            host_port = binding.get("HostPort", "?")
                            ports.append(f"{host_port}→{container_port}")

                    # Uptime (if running)
                    uptime_s = None
                    if status_str == "running":
                        started = container.attrs["State"].get("StartedAt", "")
                        if started:
                            start_time = datetime.fromisoformat(started.replace("Z", "+00:00"))
                            uptime_s = (datetime.now(start_time.tzinfo) - start_time).total_seconds()

                    # Check health
                    is_healthy = status_str == "running" and (health is None or health == "healthy")
                    if not is_healthy:
                        failures.append(f"{service_name}: {status_str} (health: {health})")

                    containers[service_name] = ContainerStatus(
                        name=service_name,
                        image=container.image.tags[0] if container.image.tags else "unknown",
                        status=status_str,
                        health=health,
                        ports=ports,
                        uptime_s=uptime_s,
                    )
                except NotFound:
                    containers[service_name] = ContainerStatus(
                        name=service_name,
                        image="unknown",
                        status="not_found",
                        last_error="Container does not exist",
                    )
                    failures.append(f"{service_name}: not found")
                except Exception as e:
                    containers[service_name] = ContainerStatus(
                        name=service_name,
                        image="unknown",
                        status="error",
                        last_error=str(e),
                    )
                    failures.append(f"{service_name}: {str(e)}")

        except httpx.RequestError as e:
            logger.error(f"Error fetching container status: {e}")

        all_healthy = len(failures) == 0
        status = StackStatus(
            timestamp=datetime.now().isoformat(),
            containers=containers,
            all_healthy=all_healthy,
            failures=failures,
        )

        # Persist to log
        self._write_status_log(status)

        return status

    async def get_logs(self, service_name: str, lines: int = 50) -> Optional[str]:
        """Get logs from a service (no CLI)."""
        if not self.client:
            await self.initialize()

        try:
            container = self.client.containers.get(service_name)
            logs = container.logs(tail=lines, decode=True)
            return logs
        except json.JSONDecodeError as e:
            logger.error(f"Failed to get logs for {service_name}: {e}")
            return None

    async def restart_service(self, service_name: str, wait_healthy_s: float = 30.0) -> bool:
        """Gracefully restart a service and wait for health."""
        if not self.client:
            await self.initialize()

        try:
            container = self.client.containers.get(service_name)
            logger.info(f"Restarting {service_name}...")
            container.restart(timeout=10)

            # Wait for health
            start_time = asyncio.get_event_loop().time()
            while asyncio.get_event_loop().time() - start_time < wait_healthy_s:
                container.reload()
                status = container.status

                if status == "running":
                    health = container.attrs.get("State", {}).get("Health", {}).get("Status")
                    if health is None or health == "healthy":
                        logger.info(f"{service_name} healthy after restart")
                        return True

                await asyncio.sleep(1.0)

            logger.warning(f"{service_name} not healthy after {wait_healthy_s}s")
            return False

        except httpx.RequestError as e:
            logger.error(f"Failed to restart {service_name}: {e}")
            return False

    async def deploy_stack(self, rebuild: bool = False) -> bool:
        """
        Deploy the full 3-service stack from docker-compose.yml.
        Returns True if all services started and passed health checks.
        """
        if not self.client:
            await self.initialize()

        if not self.COMPOSE_FILE.exists():
            logger.error(f"docker-compose.yml not found at {self.COMPOSE_FILE}")
            return False

        try:
            import subprocess

            # Run docker-compose up
            cmd = [
                "docker-compose",
                "-f",
                str(self.COMPOSE_FILE),
                "up",
                "--build" if rebuild else "",
                "-d",
            ]
            cmd = [x for x in cmd if x]  # Remove empty strings

            logger.info(f"Running: {' '.join(cmd)}")
            result = subprocess.run(cmd, cwd=str(self.DEPLOY_DIR), capture_output=True, text=True)

            if result.returncode != 0:
                logger.error(f"docker-compose up failed: {result.stderr}")
                return False

            logger.info("Stack deployment initiated")

            # Wait for health (60s total)
            for attempt in range(60):
                status = await self.get_status()
                if status.all_healthy:
                    logger.info("All services healthy")
                    return True

                await asyncio.sleep(1.0)

            logger.warning("Stack deployed but not all services healthy after 60s")
            status = await self.get_status()
            logger.warning(f"Failures: {status.failures}")
            return False

        except httpx.RequestError as e:
            logger.error(f"Deploy failed: {e}")
            return False

    async def shutdown_stack(self) -> bool:
        """Graceful shutdown of all services."""
        try:
            import subprocess

            cmd = ["docker-compose", "-f", str(self.COMPOSE_FILE), "down"]
            logger.info(f"Running: {' '.join(cmd)}")
            result = subprocess.run(cmd, cwd=str(self.DEPLOY_DIR), capture_output=True, text=True)

            if result.returncode != 0:
                logger.error(f"docker-compose down failed: {result.stderr}")
                return False

            logger.info("Stack shutdown complete")
            return True

        except asyncio.TimeoutError as e:
            logger.error(f"Shutdown failed: {e}")
            return False

    async def health_check_loop(self, interval_s: float = 10.0, auto_restart: bool = True):
        """
        Continuous health monitoring loop.
        Auto-restarts unhealthy services if auto_restart=True.
        """
        logger.info(f"Health check loop started (interval: {interval_s}s)")

        while True:
            try:
                status = await self.get_status()

                if not status.all_healthy and auto_restart:
                    logger.warning(f"Unhealthy services detected: {status.failures}")
                    for service_name in self.CYNIC_SERVICES:
                        container_status = status.containers.get(service_name)
                        if container_status and container_status.status != "running":
                            logger.warning(f"Auto-restarting {service_name}")
                            await self.restart_service(service_name)

                await asyncio.sleep(interval_s)

            except ValidationError as e:
                logger.error(f"Health check loop error: {e}")
                await asyncio.sleep(5.0)

    def _write_status_log(self, status: StackStatus):
        """Persist status to ~/.cynic/deployments/{timestamp}.json"""
        try:
            filename = self.log_dir / f"{datetime.now().strftime('%Y%m%d-%H%M%S')}-status.json"
            with open(filename, "w") as f:
                json.dump(status.to_dict(), f, indent=2)
        except OSError as e:
            logger.warning(f"Failed to write status log: {e}")


async def main():
    """CLI entry point for testing."""
    import sys

    logging.basicConfig(level=logging.INFO)

    mgr = DockerManager()

    if len(sys.argv) < 2:
        print("Usage: python docker_manager.py [status|logs|restart|deploy|shutdown|health]")
        return

    cmd = sys.argv[1]

    if cmd == "status":
        status = await mgr.get_status()
        print(json.dumps(status.to_dict(), indent=2))

    elif cmd == "logs":
        service = sys.argv[2] if len(sys.argv) > 2 else "cynic-kernel"
        logs = await mgr.get_logs(service, lines=50)
        if logs:
            print(logs)

    elif cmd == "restart":
        service = sys.argv[2] if len(sys.argv) > 2 else "cynic-kernel"
        success = await mgr.restart_service(service)
        print(f"Restart {service}: {'OK' if success else 'FAILED'}")

    elif cmd == "deploy":
        rebuild = "--rebuild" in sys.argv
        success = await mgr.deploy_stack(rebuild=rebuild)
        print(f"Deploy: {'OK' if success else 'FAILED'}")

    elif cmd == "shutdown":
        success = await mgr.shutdown_stack()
        print(f"Shutdown: {'OK' if success else 'FAILED'}")

    elif cmd == "health":
        await mgr.health_check_loop(interval_s=10.0, auto_restart=True)

    else:
        print(f"Unknown command: {cmd}")


if __name__ == "__main__":
    asyncio.run(main())
