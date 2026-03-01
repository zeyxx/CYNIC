"""Monitor machine resources: CPU, memory, disk, network, temperature, health."""

from __future__ import annotations

import logging
import time
from dataclasses import dataclass

import psutil

logger = logging.getLogger(__name__)


@dataclass(frozen=True)
class MachineState:
    """Snapshot of current machine state.

    Immutable data model representing a point-in-time snapshot of machine
    resources, constraints, and health status.
    """

    cpu_percent: float
    """CPU utilization percentage [0, 100]."""

    memory_percent: float
    """Memory utilization percentage [0, 100]."""

    disk_percent: float
    """Disk utilization percentage [0, 100]."""

    network_bandwidth: float
    """Network bandwidth in bytes per second (estimated)."""

    temperature: float
    """CPU temperature in Celsius (0 if unavailable)."""

    health: dict[str, bool]
    """Health indicators: is_healthy, cpu_ok, memory_ok, disk_ok, etc."""

    timestamp: float
    """Unix timestamp when this state was captured."""


class MachineMonitor:
    """Monitor machine resources and detect constraints."""

    # Warning thresholds
    RAM_WARNING_THRESHOLD = 75  # %
    DISK_WARNING_THRESHOLD = 70  # %
    CPU_WARNING_THRESHOLD = 80  # %

    def __init__(self):
        """Initialize machine monitor."""
        self._last_network_bytes = None
        self._last_network_time = None

    async def get_state(self) -> MachineState:
        """Get current machine state snapshot.

        Returns:
            MachineState: Current machine metrics and health status.
        """
        return MachineState(
            cpu_percent=self._get_cpu_percent(),
            memory_percent=self._get_memory_percent(),
            disk_percent=self._get_disk_percent(),
            network_bandwidth=await self._get_network_bandwidth(),
            temperature=self._get_temperature(),
            health=self._check_health(),
            timestamp=time.time(),
        )

    async def get_snapshot(self) -> MachineState:
        """Alias for get_state to match SymbioticStateManager interface."""
        return await self.get_state()

    async def detect_constraints(self) -> list[str]:
        """Detect machine resource constraints and limitations.

        Checks against warning thresholds and returns list of constraint messages.

        Returns:
            list[str]: List of constraint/warning messages (empty if none).
        """
        constraints = []
        state = await self.get_state()

        if state.memory_percent >= self.RAM_WARNING_THRESHOLD:
            constraints.append(
                f"Memory usage critical: {state.memory_percent:.1f}% "
                f"(threshold: {self.RAM_WARNING_THRESHOLD}%)"
            )

        if state.disk_percent >= self.DISK_WARNING_THRESHOLD:
            constraints.append(
                f"Disk usage warning: {state.disk_percent:.1f}% "
                f"(threshold: {self.DISK_WARNING_THRESHOLD}%)"
            )

        if state.cpu_percent >= self.CPU_WARNING_THRESHOLD:
            constraints.append(
                f"CPU usage high: {state.cpu_percent:.1f}% "
                f"(threshold: {self.CPU_WARNING_THRESHOLD}%)"
            )

        if not state.health.get("is_healthy", True):
            constraints.append("Machine health check failed")

        return constraints

    def _get_cpu_percent(self) -> float:
        """Get current CPU utilization percentage.

        Returns:
            float: CPU usage [0, 100].
        """
        try:
            return float(psutil.cpu_percent(interval=0.1))
        except Exception as e:
            logger.warning(f"Failed to get CPU percent: {e}")
            return 0.0

    def _get_memory_percent(self) -> float:
        """Get current memory utilization percentage.

        Returns:
            float: Memory usage [0, 100].
        """
        try:
            return float(psutil.virtual_memory().percent)
        except Exception as e:
            logger.warning(f"Failed to get memory percent: {e}")
            return 0.0

    def _get_disk_percent(self) -> float:
        """Get current disk utilization percentage.

        Returns:
            float: Disk usage [0, 100].
        """
        try:
            return float(psutil.disk_usage("/").percent)
        except Exception as e:
            logger.warning(f"Failed to get disk percent: {e}")
            return 0.0

    async def _get_network_bandwidth(self) -> float:
        """Get estimated network bandwidth.

        Calculates bytes per second by sampling network I/O.

        Returns:
            float: Network bandwidth in bytes per second.
        """
        try:
            net_io = psutil.net_io_counters()
            current_bytes = net_io.bytes_sent + net_io.bytes_recv
            current_time = time.time()

            if self._last_network_bytes is not None and self._last_network_time is not None:
                time_delta = current_time - self._last_network_time
                if time_delta > 0:
                    bandwidth = (current_bytes - self._last_network_bytes) / time_delta
                    self._last_network_bytes = current_bytes
                    self._last_network_time = current_time
                    return max(0.0, float(bandwidth))

            self._last_network_bytes = current_bytes
            self._last_network_time = current_time
            return 0.0
        except Exception as e:
            logger.warning(f"Failed to get network bandwidth: {e}")
            return 0.0

    def _get_temperature(self) -> float:
        """Get CPU temperature in Celsius.

        Returns:
            float: Temperature in Celsius, or 0 if unavailable.
        """
        try:
            # Check if sensors_temperatures is available (not on all systems)
            if not hasattr(psutil, "sensors_temperatures"):
                return 0.0

            temps = psutil.sensors_temperatures()
            if temps:
                # Try to get core temperature (most common sensors)
                for _sensor_name, entries in temps.items():
                    if entries:
                        # Return first sensor reading
                        return float(entries[0].current)
            return 0.0
        except Exception as e:
            logger.debug(f"Failed to get temperature: {e}")
            return 0.0

    def _check_health(self) -> dict[str, bool]:
        """Check overall machine health.

        Returns:
            dict[str, bool]: Health indicators.
        """
        cpu = self._get_cpu_percent()
        memory = self._get_memory_percent()
        disk = self._get_disk_percent()

        is_healthy = (
            cpu < self.CPU_WARNING_THRESHOLD
            and memory < self.RAM_WARNING_THRESHOLD
            and disk < self.DISK_WARNING_THRESHOLD
        )

        return {
            "is_healthy": is_healthy,
            "cpu_ok": cpu < self.CPU_WARNING_THRESHOLD,
            "memory_ok": memory < self.RAM_WARNING_THRESHOLD,
            "disk_ok": disk < self.DISK_WARNING_THRESHOLD,
        }
