"""CYNIC DiskWatcher â€” CYNICÃ—PERCEIVE/REFLEX every F(9)=34s."""

from __future__ import annotations

import asyncio
import logging
import os
import shutil
from typing import Any

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.phi import PHI_INV, PHI_INV_3, fibonacci
from cynic.kernel.organism.perception.senses.workers.base import PerceiveWorker

logger = logging.getLogger("cynic.kernel.organism.perception.senses")

# Ï†-derived disk usage thresholds (fraction of disk used)
_DISK_WARN = PHI_INV  # 0.618 â€” 61.8% full â†’ GROWL / LOD 1
_DISK_CRITICAL = 1 - PHI_INV_3  # 0.764 â€” 76.4% full â†’ LOD 2
_DISK_EMERGENCY = 0.90  # 90%   full â†’ BARK  / LOD 3


class DiskWatcher(PerceiveWorker):
    """
    Monitors disk usage via shutil.disk_usage() (stdlib, no psutil needed).

    Submits CYNICÃ—PERCEIVE at REFLEX when disk exceeds Ï†-thresholds.
    Also emits DISK_PRESSURE on the core bus â†’ triggers StorageGC.

    Deduplicates: only emits when the pressure level CHANGES
    (WARN â†’ CRITICAL â†’ EMERGENCY, or back to OK).

    Thresholds (Ï†-derived, fraction of disk used):
      WARN      â‰¥ 0.618 (61.8%) â†’ LOD 1, GC pre-warm
      CRITICAL  â‰¥ 0.764 (76.4%) â†’ LOD 2, GC aggressive
      EMERGENCY â‰¥ 0.90  (90%)   â†’ LOD 3, BARK

    interval: F(9)=34s â€” same cadence as MarketWatcher/SolanaWatcher.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(9))  # 34.0s
    name = "disk_watcher"

    def __init__(self, path: Optional[str] = None) -> None:
        # Monitor the drive where ~/.cynic lives (covers DB data dir too)
        self._path = path or os.path.expanduser("~")
        self._last_level: Optional[str] = None  # deduplicate emissions

    def _check_disk(self) -> dict[str, Any]:
        """Blocking disk check â€” called via run_in_executor."""
        usage = shutil.disk_usage(self._path)
        used_pct = usage.used / usage.total
        free_gb = usage.free / (1024**3)
        total_gb = usage.total / (1024**3)

        if used_pct >= _DISK_EMERGENCY:
            pressure = "EMERGENCY"
        elif used_pct >= _DISK_CRITICAL:
            pressure = "CRITICAL"
        elif used_pct >= _DISK_WARN:
            pressure = "WARN"
        else:
            pressure = "OK"

        return {
            "used_pct": used_pct,
            "free_gb": free_gb,
            "total_gb": total_gb,
            "pressure": pressure,
        }

    async def sense(self) -> Optional[Cell]:
        loop = asyncio.get_running_loop()
        try:
            info = await loop.run_in_executor(None, self._check_disk)
        except ValidationError as exc:
            logger.debug("DiskWatcher check failed: %s", exc)
            return None

        pressure = info["pressure"]

        from cynic.kernel.core.nerves import SOMATIC

        if pressure == "OK":
            if self._last_level is not None:
                logger.info("DiskWatcher: disk pressure cleared (was %s)", self._last_level)
                await SOMATIC.emit_disk_cleared(
                    used_pct=round(info["used_pct"], 4), free_gb=round(info["free_gb"], 2)
                )
            self._last_level = None
            return None

        # Deduplicate â€” only emit when level changes
        if pressure == self._last_level:
            return None
        self._last_level = pressure

        used_pct = info["used_pct"]
        free_gb = info["free_gb"]

        # Emit DISK_PRESSURE via nerves
        await SOMATIC.emit_disk_pressure(
            pressure=pressure, used_pct=round(used_pct, 4), free_gb=round(free_gb, 2)
        )

        risk = {
            "WARN": 0.4,
            "CRITICAL": 0.7,
            "EMERGENCY": 1.0,
        }.get(pressure, 0.4)

        return Cell(
            reality="CYNIC",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "disk_used_pct": round(used_pct * 100, 1),
                "disk_free_gb": round(free_gb, 2),
                "disk_total_gb": round(info["total_gb"], 2),
                "disk_pressure": pressure,
            },
            context=(
                f"Disk watcher: {used_pct * 100:.1f}% full " f"({free_gb:.1f} GB free) â€” {pressure}"
            ),
            risk=risk,
            complexity=0.2,
            budget_usd=0.001,
            metadata={"source": "disk_watcher", "disk_pressure": pressure},
        )
