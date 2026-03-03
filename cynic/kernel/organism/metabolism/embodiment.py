"""
Embodiment Layer " The physical body of CYNIC.
Anatomy: Peripheral Nervous System & Somatic Sensors.

This layer connects the abstract organism to the hardware reality.
Axiom Alignment: BURN " resources are finite and costly.
"""

import logging
import platform
import time
from dataclasses import dataclass, field
from typing import Any, Optional

import psutil

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.phi import PHI_INV

logger = logging.getLogger("cynic.kernel.organism.layers.embodiment")


@dataclass
class SomaticState:
    """The physical state of the hardware body."""

    cpu_percent: float = 0.0
    ram_percent: float = 0.0
    disk_usage: float = 0.0
    battery_percent: float | None = None
    is_charging: bool = True
    cpu_temp: float | None = None
    cpu_count: int = field(default_factory=psutil.cpu_count)
    architecture: str = field(default_factory=platform.machine)
    processor: str = field(default_factory=platform.processor)
    timestamp: float = field(default_factory=time.time)

    def to_dict(self) -> dict[str, Any]:
        return {
            "cpu": self.cpu_percent,
            "ram": self.ram_percent,
            "disk": self.disk_usage,
            "temp": self.cpu_temp,
            "processor": self.processor,
            "architecture": self.architecture,
            "cpu_count": self.cpu_count,
            "timestamp": self.timestamp,
        }


class HardwareBody:
    """
    The 'Skin' and 'Nerves' of CYNIC.
    Pulls hardware metrics and injects them into the Organism's consciousness.
    Optimized for Ryzen 5700G baselines.
    """

    def __init__(self, bus: EventBus) -> None:
        self._last_state: SomaticState | None = None
        self._start_time = time.time()
        self._bus = bus
        # Initial synchronous pulse to seed the state
        self._sync_pulse()

    def _sync_pulse(self) -> None:
        """Synchronous pulse for initialization."""
        try:
            cpu = psutil.cpu_percent(interval=0.1)
            ram = psutil.virtual_memory().percent
            disk = psutil.disk_usage("/").percent
            self._last_state = SomaticState(
                cpu_percent=cpu, ram_percent=ram, disk_usage=disk
            )
        except Exception:
            self._last_state = SomaticState()

    async def pulse(self) -> SomaticState:
        """One heartbeat of the physical body."""
        try:
            # 1. Gather hardware metrics (Non-blocking)
            cpu = psutil.cpu_percent(interval=None)
            ram = psutil.virtual_memory().percent
            disk = psutil.disk_usage("/").percent

            # Temperature (OS Dependent)
            temp = None
            if hasattr(psutil, "sensors_temperatures"):
                temps = psutil.sensors_temperatures()
                # Try common Ryzen sensors
                for name in ["k10temp", "amdgpu", "coretemp"]:
                    if name in temps and temps[name]:
                        temp = temps[name][0].current
                        break

            state = SomaticState(
                cpu_percent=cpu,
                ram_percent=ram,
                disk_usage=disk,
                cpu_temp=temp,
            )

            self._last_state = state

            # 2. Emit somatic sensation to the bus
            await self._bus.emit(
                Event(type="organism.somatic_sensation", payload=state.to_dict())
            )

            # 3. High-stress detection
            if cpu > 80.0:
                await self._bus.emit(
                    Event.typed(
                        CoreEvent.ANOMALY_DETECTED,
                        payload={
                            "type": "CPU_STRESS",
                            "value": cpu,
                            "source": "somatic",
                        },
                        source="hardware_body",
                    )
                )

            return state

        except Exception as e:
            logger.debug(f"HardwareBody pulse error: {e}")
            return SomaticState()

    def get_metabolic_cost(self) -> float:
        """
        Calculate current energy price. 
        Scales with CPU/RAM pressure.
        """
        if not self._last_state:
            return 1.0

        # Base load normalized [0, 1]
        load = (self._last_state.cpu_percent + self._last_state.ram_percent) / 200.0
        # Quadratic penalty phi-scaled
        penalty = (load**2) * (1.0 / PHI_INV)

        return 1.0 + penalty
