"""
CYNIC Kernel Manager — Unified Kernel Lifecycle Coordination

Coordinates kernel initialization, health monitoring, and instance detection.
Ensures only one kernel is spawned even with multiple MCP bridges.

Key responsibilities:
- Serialize kernel initialization via file-based locking
- Prefer Docker containers over subprocess spawning
- Monitor kernel health in background
- Detect stale processes and enable recovery
- Warn if multiple MCP bridge instances detected
"""

import asyncio
import logging
import os
from typing import Optional

from cynic.mcp.kernel_bootstrap import KernelBootstrap, BootstrapResult
from cynic.mcp.kernel_health import KernelHealthMonitor
from cynic.mcp.kernel_lock import KernelLockManager, get_lock_manager

logger = logging.getLogger(__name__)

CYNIC_URL = "http://127.0.0.1:8765"
BOOTSTRAP_TIMEOUT = 30.0


class KernelManager:
    """Unified kernel lifecycle management with health monitoring."""

    def __init__(self, cynic_url: str = CYNIC_URL, bootstrap_timeout: float = BOOTSTRAP_TIMEOUT):
        """
        Initialize kernel manager.

        Args:
            cynic_url: Base URL of CYNIC HTTP API
            bootstrap_timeout: Timeout for kernel initialization (default: 30s)
        """
        self.cynic_url = cynic_url
        self.bootstrap_timeout = bootstrap_timeout

        self.lock_manager = get_lock_manager()
        self.bootstrap: Optional[KernelBootstrap] = None
        self.health_monitor: Optional[KernelHealthMonitor] = None
        self.bootstrap_result: Optional[BootstrapResult] = None
        self._initialized = False
        self._initialization_lock = asyncio.Lock()
        self._instance_count_checked = False

    async def initialize(self) -> bool:
        """
        Initialize kernel (one-time call).

        Sequence:
        1. Acquire lock (serialize startup)
        2. Check if kernel already running
        3. Bootstrap kernel (Docker-first or subprocess)
        4. Start health monitor
        5. Release lock

        Returns:
            True if kernel is running and healthy
        """
        async with self._initialization_lock:
            if self._initialized:
                logger.debug("Kernel already initialized, returning cached state")
                return self.bootstrap_result.kernel_running if self.bootstrap_result else False

            logger.info("Initializing CYNIC kernel...")

            try:
                # Bootstrap the kernel
                self.bootstrap = KernelBootstrap(self.cynic_url, timeout=self.bootstrap_timeout)
                self.bootstrap_result = await self.bootstrap.initialize()

                if not self.bootstrap_result.kernel_running:
                    logger.error(
                        "Kernel bootstrap failed: %s",
                        self.bootstrap_result.error or "Unknown error"
                    )
                    self._initialized = True
                    return False

                logger.info(
                    "Kernel initialized via %s in %.1fs",
                    self.bootstrap_result.startup_type,
                    self.bootstrap_result.duration_s,
                )

                # Start health monitor
                async def health_check() -> bool:
                    """Simple health check for monitor."""
                    if self.bootstrap is None:
                        return False
                    return await self.bootstrap._health_check(timeout=5.0)

                self.health_monitor = KernelHealthMonitor(
                    health_check_fn=health_check,
                    check_interval=60.0,
                    max_consecutive_failures=3,
                )
                await self.health_monitor.start()
                logger.info("Health monitor started")

                # Check for multiple instances (warn if detected)
                await self._check_instance_count()

                self._initialized = True
                return True

            except Exception as e:
                logger.error("Kernel initialization failed: %s", e, exc_info=True)
                self._initialized = True
                return False

    async def shutdown(self):
        """Graceful shutdown of kernel and health monitor."""
        if self.health_monitor:
            try:
                await self.health_monitor.stop()
                logger.info("Health monitor stopped")
            except Exception as e:
                logger.error("Failed to stop health monitor: %s", e)

    async def is_healthy(self) -> bool:
        """Check if kernel is currently healthy."""
        if not self.bootstrap_result or not self.bootstrap_result.kernel_running:
            return False

        if self.health_monitor:
            return self.health_monitor.is_healthy()

        # Fallback: do a direct health check
        if self.bootstrap:
            return await self.bootstrap._health_check(timeout=5.0)

        return False

    async def is_critical(self) -> bool:
        """Check if kernel is in critical state."""
        if self.health_monitor:
            return self.health_monitor.is_critical()
        return False

    def get_status(self):
        """Get detailed health status."""
        return {
            "initialized": self._initialized,
            "kernel_running": self.bootstrap_result.kernel_running if self.bootstrap_result else False,
            "startup_type": self.bootstrap_result.startup_type if self.bootstrap_result else None,
            "bootstrap_duration_s": self.bootstrap_result.duration_s if self.bootstrap_result else None,
            "health_status": self.health_monitor.get_status() if self.health_monitor else None,
        }

    async def _check_instance_count(self):
        """Warn if multiple MCP bridge instances are detected."""
        if self._instance_count_checked:
            return

        self._instance_count_checked = True

        try:
            # Count how many Python processes named "claude_code_bridge" exist
            import subprocess
            import sys

            if sys.platform == "win32":
                # Windows: Use tasklist
                result = subprocess.run(
                    ["tasklist", "/v"],
                    capture_output=True,
                    text=True,
                    timeout=2,
                )
                output = result.stdout
            else:
                # Unix/Linux: Use ps with grep
                result = subprocess.run(
                    ["ps", "aux"],
                    capture_output=True,
                    text=True,
                    timeout=2,
                )
                output = result.stdout

            # Count occurrences of claude_code_bridge
            count = output.count("claude_code_bridge")

            if count > 1:
                logger.warning(
                    "Multiple MCP bridge instances detected (%d). "
                    "Consider closing extra Claude Code windows to save memory.",
                    count,
                )

        except Exception as e:
            logger.debug("Could not check instance count: %s", e)


# Module-level singleton
_manager: Optional[KernelManager] = None


def get_kernel_manager(
    cynic_url: str = CYNIC_URL,
    bootstrap_timeout: float = BOOTSTRAP_TIMEOUT,
) -> KernelManager:
    """Get or create the kernel manager singleton."""
    global _manager
    if _manager is None:
        _manager = KernelManager(cynic_url, bootstrap_timeout)
    return _manager


async def shutdown_kernel_manager():
    """Gracefully shutdown the kernel manager (for cleanup on exit)."""
    global _manager
    if _manager:
        await _manager.shutdown()
