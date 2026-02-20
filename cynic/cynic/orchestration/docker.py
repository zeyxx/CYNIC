"""
Docker Orchestration — Build, deploy, health checks.

CYNIC manages its own containers: no external CI/CD.
Pure subprocess + JSON state (Docker Desktop API not required).
"""
from __future__ import annotations

import asyncio
import json
import logging
import re
import subprocess
from dataclasses import dataclass, asdict, field
from datetime import datetime
from pathlib import Path
from typing import Optional, List, Dict, Any

logger = logging.getLogger(__name__)

CYNIC_ROOT = Path(__file__).parent.parent.parent.parent
DOCKER_COMPOSE_PATH = CYNIC_ROOT / "docker-compose.yml"
BUILD_STATE_FILE = CYNIC_ROOT / ".cynic" / "docker_state.json"


@dataclass
class ServiceHealth:
    """Health status for a single service."""
    service: str
    status: str  # "healthy", "starting", "unhealthy", "missing"
    latency_ms: float = 0.0
    last_check: datetime = field(default_factory=datetime.utcnow)
    error: Optional[str] = None

    def to_dict(self) -> dict:
        return {
            "service": self.service,
            "status": self.status,
            "latency_ms": self.latency_ms,
            "last_check": self.last_check.isoformat(),
            "error": self.error,
        }


@dataclass
class BuildResult:
    """Result of a build operation."""
    version: str
    image: str
    success: bool
    duration_s: float
    output: str = ""
    error: Optional[str] = None
    timestamp: datetime = field(default_factory=datetime.utcnow)

    def to_dict(self) -> dict:
        return {
            "version": self.version,
            "image": self.image,
            "success": self.success,
            "duration_s": self.duration_s,
            "output": self.output[:500],  # cap output
            "error": self.error,
            "timestamp": self.timestamp.isoformat(),
        }


@dataclass
class DeployResult:
    """Result of a deploy operation."""
    environment: str
    success: bool
    services: List[str] = field(default_factory=list)
    duration_s: float = 0.0
    error: Optional[str] = None
    timestamp: datetime = field(default_factory=datetime.utcnow)

    def to_dict(self) -> dict:
        return {
            "environment": self.environment,
            "success": self.success,
            "services": self.services,
            "duration_s": self.duration_s,
            "error": self.error,
            "timestamp": self.timestamp.isoformat(),
        }


class DockerManager:
    """Manages CYNIC's self-orchestration via Docker Compose."""

    def __init__(self, compose_path: Path = DOCKER_COMPOSE_PATH, venv_path: Optional[Path] = None):
        """
        Initialize manager.

        Args:
            compose_path: Path to docker-compose.yml
            venv_path: Optional path to Python venv for running commands
        """
        self.compose_path = compose_path
        self.venv_path = venv_path
        self._last_build: Optional[BuildResult] = None
        self._last_deploy: Optional[DeployResult] = None
        self._service_health: Dict[str, ServiceHealth] = {}

    async def build(self, version: str = "latest") -> BuildResult:
        """Build CYNIC Docker image."""
        logger.info(f"Building Docker image version={version}")

        start = datetime.utcnow()
        try:
            cmd = ["docker", "build", "-t", f"cynic:{version}", "."]

            # Run build
            result = await asyncio.create_subprocess_exec(
                *cmd,
                cwd=str(self.compose_path.parent),
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await result.communicate()
            output = (stdout.decode() + stderr.decode()).strip()

            duration = (datetime.utcnow() - start).total_seconds()

            if result.returncode == 0:
                build_result = BuildResult(
                    version=version,
                    image=f"cynic:{version}",
                    success=True,
                    duration_s=duration,
                    output=output,
                )
                logger.info(f"Build successful: {build_result.image}")
            else:
                build_result = BuildResult(
                    version=version,
                    image=f"cynic:{version}",
                    success=False,
                    duration_s=duration,
                    output=output,
                    error=f"Build failed with rc={result.returncode}",
                )
                logger.error(f"Build failed: {build_result.error}")

            self._last_build = build_result
            return build_result

        except Exception as exc:
            error_msg = f"Build error: {exc}"
            logger.error(error_msg)
            return BuildResult(
                version=version,
                image=f"cynic:{version}",
                success=False,
                duration_s=(datetime.utcnow() - start).total_seconds(),
                error=error_msg,
            )

    async def deploy(self, environment: str = "dev", pull: bool = True) -> DeployResult:
        """Deploy CYNIC services via docker-compose."""
        logger.info(f"Deploying environment={environment} pull={pull}")

        start = datetime.utcnow()
        try:
            cmd = ["docker-compose", "-f", str(self.compose_path)]

            if pull:
                cmd.extend(["pull", "--quiet"])

            cmd.extend(["up", "-d"])

            # Run deploy
            result = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await result.communicate()
            output = (stdout.decode() + stderr.decode()).strip()

            duration = (datetime.utcnow() - start).total_seconds()

            if result.returncode == 0:
                # Parse service names from compose file
                services = await self._get_services()
                deploy_result = DeployResult(
                    environment=environment,
                    success=True,
                    services=services,
                    duration_s=duration,
                )
                logger.info(f"Deploy successful: {', '.join(services)}")
            else:
                deploy_result = DeployResult(
                    environment=environment,
                    success=False,
                    duration_s=duration,
                    error=f"Deploy failed with rc={result.returncode}",
                )
                logger.error(f"Deploy failed: {deploy_result.error}")

            self._last_deploy = deploy_result
            return deploy_result

        except Exception as exc:
            error_msg = f"Deploy error: {exc}"
            logger.error(error_msg)
            return DeployResult(
                environment=environment,
                success=False,
                duration_s=(datetime.utcnow() - start).total_seconds(),
                error=error_msg,
            )

    async def health_check(self, services: Optional[List[str]] = None) -> List[ServiceHealth]:
        """Check health of services."""
        if services is None:
            services = await self._get_services()

        logger.info(f"Health check: {', '.join(services)}")

        health_results = []
        for service in services:
            start = datetime.utcnow()
            try:
                # Simple docker ps check
                cmd = ["docker", "ps", "--filter", f"name={service}"]
                result = await asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                stdout, _ = await result.communicate()
                output = stdout.decode().strip()

                latency = (datetime.utcnow() - start).total_seconds() * 1000

                if service in output:
                    health = ServiceHealth(
                        service=service,
                        status="healthy",
                        latency_ms=latency,
                    )
                else:
                    health = ServiceHealth(
                        service=service,
                        status="missing",
                        latency_ms=latency,
                        error=f"Service not found in docker ps",
                    )

                self._service_health[service] = health
                health_results.append(health)

            except Exception as exc:
                health = ServiceHealth(
                    service=service,
                    status="unhealthy",
                    error=str(exc),
                )
                self._service_health[service] = health
                health_results.append(health)

        return health_results

    async def stop(self, services: Optional[List[str]] = None) -> Dict[str, Any]:
        """Stop services."""
        try:
            cmd = ["docker-compose", "-f", str(self.compose_path), "down"]

            result = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            await result.communicate()

            if result.returncode == 0:
                logger.info("Services stopped successfully")
                return {"success": True, "message": "Services stopped"}
            else:
                error = f"Stop failed with rc={result.returncode}"
                logger.error(error)
                return {"success": False, "error": error}

        except Exception as exc:
            error_msg = f"Stop error: {exc}"
            logger.error(error_msg)
            return {"success": False, "error": error_msg}

    async def status(self) -> Dict[str, Any]:
        """Get overall orchestration status."""
        return {
            "last_build": self._last_build.to_dict() if self._last_build else None,
            "last_deploy": self._last_deploy.to_dict() if self._last_deploy else None,
            "service_health": {k: v.to_dict() for k, v in self._service_health.items()},
            "timestamp": datetime.utcnow().isoformat(),
        }

    async def _get_services(self) -> List[str]:
        """Extract service names from docker-compose.yml."""
        try:
            import yaml
            with open(self.compose_path) as f:
                compose = yaml.safe_load(f)
            return list(compose.get("services", {}).keys())
        except ImportError:
            # Fallback: parse manually
            try:
                with open(self.compose_path) as f:
                    content = f.read()
                # Simple regex: look for "servicename:" at start of line
                services = re.findall(r"^\s{2}(\w+):", content, re.MULTILINE)
                return services
            except Exception as exc:
                logger.warning(f"Could not parse compose file: {exc}")
                return ["cynic-api", "postgres", "ollama"]  # fallback defaults


# Singleton instance
_docker_manager: Optional[DockerManager] = None


def get_docker_manager(compose_path: Path = DOCKER_COMPOSE_PATH) -> DockerManager:
    """Get or create DockerManager singleton."""
    global _docker_manager
    if _docker_manager is None:
        _docker_manager = DockerManager(compose_path)
    return _docker_manager
