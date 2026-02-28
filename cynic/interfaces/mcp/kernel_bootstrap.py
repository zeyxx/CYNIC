"""
CYNIC Kernel Bootstrap — Unified Initialization

Handles coordinated kernel startup with:
- Docker-first preference (if available)
- Subprocess fallback (if Docker unavailable)
- Atomic initialization (via file locks)
- Health verification (exponential backoff)
"""

import asyncio
import json
import logging
import os
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Literal, Optional

import aiohttp

from cynic.interfaces.mcp.kernel_lock import KernelLockManager, KernelLockError, get_lock_manager

logger = logging.getLogger(__name__)

CYNIC_URL = "http://127.0.0.1:8765"
CYNIC_PORT = 8765
BOOTSTRAP_TIMEOUT = 30.0
HEALTH_CHECK_TIMEOUT = 5.0


@dataclass
class BootstrapResult:
    """Result of kernel bootstrap operation."""
    kernel_running: bool
    startup_type: Literal["docker", "subprocess", "already_running", "error"]
    duration_s: float
    error: Optional[str] = None
    owner_info: Optional[tuple] = None  # (pid, hostname, timestamp)


class DockerClient:
    """Simple Docker client for health checks."""

    def __init__(self):
        self.available = False
        self.container_name = "cynic"
        self._check_availability()

    def _check_availability(self):
        """Check if Docker daemon is available."""
        try:
            import docker

            self.client = docker.from_env()
            self.available = True
            logger.debug("Docker client available")
        except ImportError:
            logger.debug("Docker SDK not installed")
        except Exception as e:
            logger.debug("Docker daemon unavailable: %s", e)

    async def is_container_healthy(self, timeout: float = 10.0) -> bool:
        """Check if CYNIC container is healthy."""
        if not self.available:
            return False

        try:
            import docker

            client = docker.from_env()
            containers = client.containers.list()

            for container in containers:
                if self.container_name in container.name:
                    # Check container status
                    if container.status != "running":
                        logger.debug("Container %s not running (status=%s)",
                                     self.container_name, container.status)
                        return False

                    # Check health if available
                    health = container.attrs.get("State", {}).get("Health", {})
                    if health.get("Status") == "healthy":
                        logger.debug("Container %s is healthy", self.container_name)
                        return True

                    if health.get("Status") == "starting":
                        logger.debug("Container %s is starting", self.container_name)
                        return False

                    # No health check configured, assume OK if running
                    logger.debug("Container %s running (no health check)", self.container_name)
                    return True

            logger.debug("Container %s not found", self.container_name)
            return False

        except Exception as e:
            logger.debug("Docker check failed: %s", e)
            return False


class KernelBootstrap:
    """Unified kernel initialization with Docker preference."""

    def __init__(self, cynic_url: str = CYNIC_URL, timeout: float = BOOTSTRAP_TIMEOUT):
        self.cynic_url = cynic_url
        self.timeout = timeout
        self.docker_client = DockerClient()
        self.lock_manager: Optional[KernelLockManager] = None
        self.kernel_process: Optional[subprocess.Popen] = None

    async def initialize(self) -> BootstrapResult:
        """
        Bootstrap kernel with atomic initialization.

        Sequence:
        1. Acquire lock (serialize startup)
        2. Check if kernel already running
        3. Try Docker (if available)
        4. Fall back to subprocess
        5. Health check with retries
        6. Release lock

        Returns:
            BootstrapResult with startup_type and status
        """
        start_time = time.time()
        self.lock_manager = get_lock_manager()

        try:
            # Step 1: Acquire lock
            logger.info("Acquiring kernel lock...")
            await self.lock_manager.acquire(timeout=self.timeout)

            # Step 2: Check if already running
            logger.info("Checking if kernel is already running...")
            if await self._health_check(timeout=5.0):
                logger.info("Kernel already running")
                holder = await self.lock_manager.get_holder()
                duration = time.time() - start_time
                return BootstrapResult(
                    kernel_running=True,
                    startup_type="already_running",
                    duration_s=duration,
                    owner_info=holder,
                )

            # Step 3: Try Docker
            logger.info("Checking Docker availability...")
            if self.docker_client.available:
                if await self.docker_client.is_container_healthy():
                    logger.info("Using Docker container")
                    if await self._health_check(timeout=self.timeout - 5):
                        duration = time.time() - start_time
                        holder = await self.lock_manager.get_holder()
                        return BootstrapResult(
                            kernel_running=True,
                            startup_type="docker",
                            duration_s=duration,
                            owner_info=holder,
                        )

            # Step 4: Fall back to subprocess
            logger.info("Falling back to subprocess spawn...")
            await self._spawn_kernel()

            # Step 5: Health check
            logger.info("Verifying kernel health...")
            if await self._health_check(timeout=self.timeout):
                duration = time.time() - start_time
                holder = await self.lock_manager.get_holder()
                logger.info("Kernel initialized via subprocess in %.1fs", duration)
                return BootstrapResult(
                    kernel_running=True,
                    startup_type="subprocess",
                    duration_s=duration,
                    owner_info=holder,
                )

            # Failed to initialize
            duration = time.time() - start_time
            error_msg = f"Kernel failed to become healthy after {duration:.1f}s"
            logger.error(error_msg)
            return BootstrapResult(
                kernel_running=False,
                startup_type="error",
                duration_s=duration,
                error=error_msg,
            )

        except KernelLockError as e:
            duration = time.time() - start_time
            error_msg = f"Lock error: {e}"
            logger.error(error_msg)
            return BootstrapResult(
                kernel_running=False,
                startup_type="error",
                duration_s=duration,
                error=error_msg,
            )

        finally:
            # Always release lock
            if self.lock_manager:
                try:
                    await self.lock_manager.release()
                except Exception as e:
                    logger.warning("Failed to release lock: %s", e)

    async def _spawn_kernel(self):
        """Spawn CYNIC kernel as subprocess."""
        try:
            repo_root = Path(__file__).parent.parent.parent
            cmd = [sys.executable, "-m", "cynic.interfaces.api.entry", "--port", str(CYNIC_PORT)]

            logger.info("Spawning kernel: %s (cwd=%s)", " ".join(cmd), repo_root)

            self.kernel_process = subprocess.Popen(
                cmd,
                cwd=str(repo_root),
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if sys.platform == "win32" else 0,
            )

            logger.info("Kernel spawned with PID: %s", self.kernel_process.pid)

        except Exception as e:
            logger.error("Failed to spawn kernel: %s", e, exc_info=True)
            raise RuntimeError(f"Cannot spawn kernel: {e}") from e

    async def _health_check(self, timeout: float = HEALTH_CHECK_TIMEOUT) -> bool:
        """
        Check if kernel is healthy at cynic_url/health.

        Uses curl for reliable health checking (works on Windows with ProactorEventLoop).
        Implements exponential backoff retry logic:
        - Attempt 1: no wait
        - Attempt 2: 0.5s wait
        - Attempt 3: 1.0s wait
        - Attempt 4: 2.0s wait
        """
        start_time = time.time()
        attempt = 0
        backoff = [0, 0.5, 1.0, 2.0, 4.0]
        loop = asyncio.get_event_loop()

        while time.time() - start_time < timeout:
            attempt += 1

            try:
                # Use curl for health check (works reliably on Windows with ProactorEventLoop)
                result = await loop.run_in_executor(
                    None,
                    lambda: subprocess.run(
                        ["curl", "-s", "-m", "5", f"{self.cynic_url}/health"],
                        capture_output=True,
                        text=True,
                        timeout=6
                    )
                )

                if result.returncode == 0:
                    elapsed = time.time() - start_time
                    logger.debug("Health check passed after %.1fs (attempt %d)",
                                 elapsed, attempt)
                    return True

                logger.debug("Health check failed (attempt %d, curl exit code %d)", attempt, result.returncode)

            except (OSError, asyncio.TimeoutError, Exception) as e:
                logger.debug("Health check error (attempt %d): %s", attempt, type(e).__name__)

            # Backoff before retry
            if attempt < len(backoff):
                delay = backoff[attempt]
                if delay > 0:
                    remaining = timeout - (time.time() - start_time)
                    if remaining > delay:
                        await asyncio.sleep(delay)

        elapsed = time.time() - start_time
        logger.debug("Health check failed after %.1fs", elapsed)
        return False


# Module-level singleton
_bootstrap: Optional[KernelBootstrap] = None


async def get_bootstrap() -> KernelBootstrap:
    """Get or create the kernel bootstrap singleton."""
    global _bootstrap
    if _bootstrap is None:
        _bootstrap = KernelBootstrap()
    return _bootstrap
