"""
CYNIC Infrastructure Orchestrator - Universal Reality Provisioner.
Respects Solutions Architect, SRE, and Robotics Lenses.

This is the sovereign orchestrator (Extreme Level). It does not blindly run
docker-compose. It probes the host environment (Windows, Linux, Cloud),
diagnoses the state of the container engine, and actively provisions
the required realities (SurrealDB, Redis, Vault).
"""

from __future__ import annotations

import asyncio
import logging
import platform
import socket
import subprocess
from dataclasses import dataclass
from typing import Dict, Optional

logger = logging.getLogger("cynic.kernel.infra.orchestrator")


@dataclass
class ServiceState:
    name: str
    is_running: bool
    port_open: bool
    error: Optional[str] = None


class InfrastructureOrchestrator:
    """
    Sovereign orchestrator for CYNIC's external dependencies.
    """

    def __init__(self):
        self.os_type = platform.system()
        self.is_windows = self.os_type == "Windows"

    async def probe_docker_engine(self) -> bool:
        """Deep probe to check if the Docker Engine is actually responding."""
        try:
            # Use docker info to check actual engine connectivity, not just the CLI
            proc = await asyncio.create_subprocess_exec(
                "docker",
                "info",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            _, stderr = await proc.communicate()

            if proc.returncode != 0:
                err_str = stderr.decode().strip()
                logger.warning(
                    f"Docker Engine offline or unreachable: {err_str[:100]}..."
                )

                # Auto-Remediation Attempt for Windows
                if self.is_windows:
                    logger.warning("Attempting to wake up Docker Desktop on Windows...")
                    # Fire and forget the start command
                    subprocess.Popen(
                        [
                            "powershell",
                            "-Command",
                            r"Start-Process -NoNewWindow -FilePath 'C:\Program Files\Docker\Docker\Docker Desktop.exe'",
                        ],
                        creationflags=subprocess.CREATE_NO_WINDOW,
                    )

                    # Patiently wait for the engine to boot (up to 60 seconds)
                    logger.info(
                        "Waiting for Docker Engine to initialize (this may take up to 60s)..."
                    )
                    for i in range(12):
                        await asyncio.sleep(5)
                        check_proc = await asyncio.create_subprocess_exec(
                            "docker",
                            "info",
                            stdout=asyncio.subprocess.PIPE,
                            stderr=asyncio.subprocess.PIPE,
                        )
                        await check_proc.communicate()
                        if check_proc.returncode == 0:
                            logger.info("✅ Docker Engine is now ONLINE.")
                            return True
                        logger.debug(f"Still waiting for Docker... ({i*5}s)")

                    logger.error(
                        "Auto-remediation failed: Docker Engine did not start in time."
                    )
                    return False

                return False
            return True

        except FileNotFoundError:
            logger.critical(
                "Docker CLI not found on host. The organism cannot spawn its realities."
            )
            return False

    def is_port_open(self, host: str, port: int) -> bool:
        """Physical verification of reality."""
        try:
            with socket.create_connection((host, port), timeout=1):
                return True
        except (ConnectionRefusedError, socket.timeout):
            return False

    async def check_vital_organs(self) -> Dict[str, ServiceState]:
        """Audit the core external realities required by CYNIC."""
        return {
            "surrealdb": ServiceState(
                name="surrealdb",
                is_running=False,  # We don't check container state yet, just the port
                port_open=self.is_port_open("localhost", 8000),
            ),
            "redis": ServiceState(
                name="redis",
                is_running=False,
                port_open=self.is_port_open("localhost", 6379),
            ),
        }

    async def provision_reality(self) -> bool:
        """
        The Sovereign boot sequence.
        Attempts to bring all required services online.
        """
        logger.info("Orchestrator: Probing reality infrastructure...")

        organs = await self.check_vital_organs()
        if all(o.port_open for o in organs.values()):
            logger.info(
                "Orchestrator: All vital realities (SurrealDB, Redis) are breathing."
            )
            return True

        logger.warning(
            "Orchestrator: Vital realities are missing. Initiating provision sequence."
        )

        engine_up = await self.probe_docker_engine()
        if not engine_up:
            logger.error("Orchestrator: Cannot provision. Docker Engine is down.")
            return False

        try:
            logger.info("Orchestrator: Spawning realities via docker-compose...")
            proc = await asyncio.create_subprocess_exec(
                "docker-compose",
                "up",
                "-d",
                "surrealdb",
                "redis",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()

            if proc.returncode != 0:
                logger.error(
                    f"Orchestrator: Provisioning failed. {stderr.decode()[:200]}"
                )
                return False

            # Stabilization Wait
            logger.info("Orchestrator: Realities spawned. Waiting for stabilization...")
            for _ in range(15):
                organs = await self.check_vital_organs()
                if all(o.port_open for o in organs.values()):
                    logger.info(
                        "Orchestrator: Realities stabilized. CYNIC can connect."
                    )
                    return True
                await asyncio.sleep(1)

            logger.error(
                "Orchestrator: Timeout waiting for realities to open their ports."
            )
            return False

        except FileNotFoundError:
            logger.critical("Orchestrator: docker-compose not found.")
            return False
