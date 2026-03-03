"""
CYNIC Kernel Health Monitor " Background Health Tracking

Monitors kernel health with:
- Periodic health checks (configurable interval)
- State tracking (HEALTHY, DEGRADED, CRITICAL)
- Event emission on state changes
- Automatic recovery detection
"""

import asyncio
import logging
import time
from collections.abc import Callable
from dataclasses import dataclass
from enum import Enum
from typing import Any

logger = logging.getLogger(__name__)


class HealthState(Enum):
    """Kernel health state."""

    HEALTHY = "healthy"
    DEGRADED = "degraded"
    CRITICAL = "critical"
    UNKNOWN = "unknown"


@dataclass
class HealthStatus:
    """Current kernel health status."""

    state: HealthState
    last_check: float | None
    consecutive_failures: int
    last_error: str | None
    check_count: int


class KernelHealthMonitor:
    """Background kernel health monitoring."""

    def __init__(
        self,
        health_check_fn: Callable[[], Any],
        check_interval: float = 60.0,
        max_consecutive_failures: int = 3,
    ):
        """
        Initialize health monitor.

        Args:
            health_check_fn: Async callable that returns True if healthy
            check_interval: Seconds between checks (default: 60)
            max_consecutive_failures: Failures before CRITICAL state (default: 3)
        """
        self.health_check_fn = health_check_fn
        self.check_interval = check_interval
        self.max_consecutive_failures = max_consecutive_failures

        self.state = HealthState.UNKNOWN
        self.last_check: float | None = None
        self.consecutive_failures = 0
        self.last_error: str | None = None
        self.check_count = 0

        self._monitor_task: asyncio.Task | None = None
        self._running = False

    async def start(self):
        """Start health monitoring background task."""
        if self._running:
            logger.warning("Health monitor already running")
            return

        self._running = True
        self._monitor_task = asyncio.create_task(self._monitor_loop())
        logger.info(
            "Kernel health monitor started (interval=%.1fs)", self.check_interval
        )

    async def stop(self):
        """Stop health monitoring."""
        self._running = False
        if self._monitor_task:
            try:
                await asyncio.wait_for(self._monitor_task, timeout=5.0)
            except TimeoutError:
                self._monitor_task.cancel()
        logger.info("Kernel health monitor stopped")

    async def _monitor_loop(self):
        """Background health check loop."""
        while self._running:
            try:
                await asyncio.sleep(self.check_interval)

                if not self._running:
                    break

                await self._perform_check()

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error("Monitor loop error: %s", e, exc_info=True)
                await asyncio.sleep(5.0)  # Backoff on error

    async def _perform_check(self):
        """Perform single health check."""
        try:
            self.check_count += 1
            self.last_check = time.time()

            # Run health check
            is_healthy = await asyncio.wait_for(self.health_check_fn(), timeout=5.0)

            old_state = self.state

            if is_healthy:
                # Kernel is healthy
                self.consecutive_failures = 0
                self.last_error = None

                if old_state != HealthState.HEALTHY:
                    self.state = HealthState.HEALTHY
                    logger.info(
                        "Kernel health: HEALTHY (recovered from %s)", old_state.value
                    )

            else:
                # Kernel is unhealthy
                self.consecutive_failures += 1
                self.last_error = "Health check returned False"

                if self.consecutive_failures >= self.max_consecutive_failures:
                    new_state = HealthState.CRITICAL
                else:
                    new_state = HealthState.DEGRADED

                if old_state != new_state:
                    self.state = new_state
                    logger.warning(
                        "Kernel health: %s (failures=%d/%d)",
                        new_state.value,
                        self.consecutive_failures,
                        self.max_consecutive_failures,
                    )

        except TimeoutError:
            self.consecutive_failures += 1
            self.last_error = "Health check timeout (5s)"

            new_state = (
                HealthState.CRITICAL
                if self.consecutive_failures >= self.max_consecutive_failures
                else HealthState.DEGRADED
            )
            old_state = self.state

            if old_state != new_state:
                self.state = new_state
                logger.warning(
                    "Kernel health: %s (timeout, failures=%d/%d)",
                    new_state.value,
                    self.consecutive_failures,
                    self.max_consecutive_failures,
                )

        except Exception as e:
            self.consecutive_failures += 1
            self.last_error = f"Check error: {type(e).__name__}"

            new_state = (
                HealthState.CRITICAL
                if self.consecutive_failures >= self.max_consecutive_failures
                else HealthState.DEGRADED
            )
            old_state = self.state

            if old_state != new_state:
                self.state = new_state
                logger.error(
                    "Kernel health: %s (error: %s, failures=%d/%d)",
                    new_state.value,
                    type(e).__name__,
                    self.consecutive_failures,
                    self.max_consecutive_failures,
                )

    def get_status(self) -> HealthStatus:
        """Get current health status."""
        return HealthStatus(
            state=self.state,
            last_check=self.last_check,
            consecutive_failures=self.consecutive_failures,
            last_error=self.last_error,
            check_count=self.check_count,
        )

    def is_healthy(self) -> bool:
        """Quick check if kernel is considered healthy."""
        return self.state == HealthState.HEALTHY

    def is_critical(self) -> bool:
        """Quick check if kernel is in critical state."""
        return self.state == HealthState.CRITICAL

    def reset(self):
        """Reset health state after recovery."""
        logger.info("Resetting kernel health state")
        self.state = HealthState.UNKNOWN
        self.consecutive_failures = 0
        self.last_error = None
        self.last_check = None
