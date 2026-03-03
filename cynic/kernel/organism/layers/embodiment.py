"""
Embodiment Layer â€" The physical body of CYNIC.
Anatomy: Peripheral Nervous System & Somatic Sensors.

This layer connects the abstract organism to the hardware reality.
Axiom Alignment: BURN â€" resources are finite and costly.
"""

from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any

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
    cpu_temp: float | None = None  # May not be available on all OS
    timestamp: float = field(default_factory=time.time)

    def to_dict(self) -> dict[str, Any]:
        return {
            "cpu": self.cpu_percent,
            "ram": self.ram_percent,
            "disk": self.disk_usage,
            "battery": self.battery_percent,
            "charging": self.is_charging,
            "temp": self.cpu_temp,
            "timestamp": self.timestamp,
        }


class HardwareBody:
    """
    The 'Skin' and 'Nerves' of CYNIC.
    Pulls hardware metrics and injects them into the Organism's consciousness.
    """

    def __init__(self, bus: EventBus) -> None:
        from cynic.kernel.core.formulas import get_respiration_interval_s

        self.update_interval = get_respiration_interval_s()
        self._last_state: SomaticState | None = None
        self._start_time = time.time()
        self._bus = bus
        # Initial synchronous pulse to seed the state
        self._sync_pulse()

    def _sync_pulse(self):
        """Synchronous pulse for initialization."""
        try:
            cpu = psutil.cpu_percent(interval=0.1)
            ram = psutil.virtual_memory().percent
            disk = psutil.disk_usage("/").percent
            self._last_state = SomaticState(cpu_percent=cpu, ram_percent=ram, disk_usage=disk)
        except Exception:
            self._last_state = SomaticState()

    async def pulse(self) -> SomaticState:
        """One heartbeat of the physical body."""
        try:
            # 1. Gather hardware metrics
            cpu = psutil.cpu_percent(interval=None)
            ram = psutil.virtual_memory().percent
            disk = psutil.disk_usage("/").percent

            # Battery (Optional)
            batt = psutil.sensors_battery()
            batt_percent = batt.percent if batt else None
            is_charging = batt.power_plugged if batt else True

            # Temperature (OS Dependent)
            temp = None
            if hasattr(psutil, "sensors_temperatures"):
                temps = psutil.sensors_temperatures()
                if "coretemp" in temps:
                    temp = temps["coretemp"][0].current
                elif "cpu_thermal" in temps:
                    temp = temps["cpu_thermal"][0].current

            state = SomaticState(
                cpu_percent=cpu,
                ram_percent=ram,
                disk_usage=disk,
                battery_percent=batt_percent,
                is_charging=is_charging,
                cpu_temp=temp,
            )

            self._last_state = state

            # 2. Emit somatic sensation to the bus
            # This allows cognition to 'feel' the hardware pressure
            await self._bus.emit(Event(type="organism.somatic_sensation", payload=state.to_dict()))

            # 3. Log and Emit critical somatic alerts
            if cpu > 80.0:
                logger.warning(f"Somatic Alert: CPU High Stress ({cpu}%)")
                await self._bus.emit(
                    Event.typed(
                        CoreEvent.ANOMALY_DETECTED,
                        payload={"type": "CPU_STRESS", "value": cpu, "source": "somatic"},
                        source="hardware_body",
                    )
                )
            if ram > 90.0:
                logger.error(f"Somatic Alert: Memory Exhaustion imminent ({ram}%)")
                await self._bus.emit(
                    Event.typed(
                        CoreEvent.ANOMALY_DETECTED,
                        payload={"type": "RAM_STRESS", "value": ram, "source": "somatic"},
                        source="hardware_body",
                    )
                )

            return state

        except Exception as e:
            logger.debug(f"HardwareBody pulse error: {e}")
            return SomaticState()

    def get_metabolic_cost(self) -> float:
        """
        Calculate the current 'energy price' of actions.
        If hardware is stressed, cost increases (PHI-scaled).
        """
        if not self._last_state:
            return 1.0

        # Base cost 1.0, scales up with CPU/RAM pressure
        load = (self._last_state.cpu_percent + self._last_state.ram_percent) / 200.0
        # PHI-weighted penalty for high load
        penalty = (load**2) * (1.0 / PHI_INV)

        return 1.0 + penalty
