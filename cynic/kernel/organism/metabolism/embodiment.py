"""
Universal Embodiment Layer - The Physical-to-Digital Interface.
Respects SRE & Robotics Lenses.

This module provides a hardware-agnostic somatic system that:
1. Detects the execution environment (Container, Cloud, Physical).
2. Normalizes resource metrics.
3. Provides a standard 'SomaticPulse' for the organism.
"""

from __future__ import annotations

import logging
import os
import platform
import time
from dataclasses import dataclass, field
from typing import Any, Dict, Optional

import psutil

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.phi import PHI_INV

logger = logging.getLogger("cynic.kernel.organism.layers.embodiment")


@dataclass
class SomaticMetrics:
    """Standardized hardware metrics across all platforms."""

    cpu_percent: float = 0.0
    memory_percent: float = 0.0
    io_wait: float = 0.0
    load_average: list[float] = field(default_factory=lambda: [0.0, 0.0, 0.0])
    is_containerized: bool = field(
        default_factory=lambda: os.path.exists("/.dockerenv")
    )
    platform: str = field(default_factory=lambda: platform.system())
    architecture: str = field(default_factory=lambda: platform.machine())
    timestamp: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "cpu": self.cpu_percent,
            "memory": self.memory_percent,
            "platform": self.platform,
            "container": self.is_containerized,
            "load_1m": self.load_average[0],
            "timestamp": self.timestamp,
        }


class UniversalHardwareBody:
    """
    Industrial-grade Somatic Sensor.
    Handles environment discovery without hardcoding specific hardware.
    """

    def __init__(self, bus: EventBus):
        self._bus = bus
        self._last_metrics: Optional[SomaticMetrics] = None
        self._start_time = time.time()
        logger.info(
            f"Somatic System initialized on {platform.system()} ({platform.machine()})"
        )

    async def pulse(self) -> SomaticMetrics:
        """Collect and broadcast standardized somatic metrics."""
        try:
            # 1. Non-blocking metric collection
            # Note: interval=None for psutil.cpu_percent is non-blocking
            cpu = psutil.cpu_percent(interval=None)
            mem = psutil.virtual_memory().percent

            # Load averages (not available on Windows directly via psutil)
            load = [0.0, 0.0, 0.0]
            if hasattr(os, "getloadavg"):
                load = list(os.getloadavg())

            metrics = SomaticMetrics(
                cpu_percent=cpu, memory_percent=mem, load_average=load
            )
            self._last_metrics = metrics

            # 2. Emit to the nervous system
            await self._bus.emit(
                Event(
                    type="organism.somatic_pulse",
                    payload=metrics.to_dict(),
                    source="embodiment",
                )
            )

            # 3. Industrial Alerting (Thresholds should be in config, here hardcoded for safety)
            if cpu > 90.0:
                await self._bus.emit(
                    Event.typed(
                        CoreEvent.ANOMALY_DETECTED,
                        {
                            "type": "RESOURCE_EXHAUSTION",
                            "resource": "cpu",
                            "value": cpu,
                        },
                        source="embodiment",
                    )
                )

            return metrics

        except Exception as e:
            logger.error(f"Somatic Pulse Failure: {e}")
            return SomaticMetrics()

    def get_metabolic_cost(self) -> float:
        """Universal cost calculation based on normalized load."""
        if not self._last_metrics:
            return 1.0

        # Combined load factor [0, 1]
        load_factor = (
            self._last_metrics.cpu_percent + self._last_metrics.memory_percent
        ) / 200.0
        # Industrial Scale: Penalty grows quadratically to trigger backpressure early
        return 1.0 + (load_factor**2) * (1.0 / PHI_INV)


# Alias for backward compatibility
HardwareBody = UniversalHardwareBody
